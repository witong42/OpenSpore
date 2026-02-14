use std::{
    io,
    time::Duration,
};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;
use openspore_brain::Brain;
use openspore_brain::events::BrainEvent;

mod app;
mod ui;

use app::App;

pub async fn run() -> anyhow::Result<()> {
    // 1. Pre-flight Checks (Outside of Terminal Alternate Screen)
    let config = match openspore_core::config::AppConfig::load() {
        Ok(cfg) => cfg,
        Err(_) => {
            eprintln!("❌ Error: Could not load configuration.");
            eprintln!("   Please ensure ~/.openspore/.env exists and contains a valid OPENROUTER_API_KEY.");
            eprintln!("   Run 'openspore doctor' for diagnostics.");
            return Ok(());
        }
    };

    let mut doctor = openspore_doctor::SporeDoctor::new();
    doctor.check_all();
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 2. Setup TUI Terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, event::EnableMouseCapture, event::EnableBracketedPaste)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    // Initialize Core Components
    let state = openspore_core::state::AppState::new(config.clone());
    let memory = openspore_memory::MemorySystem::new(&state);
    let brain = Brain::new(config.clone());

    // Start Watchman in background
    let watchman = std::sync::Arc::new(openspore_watchman::Watchman::new(config.clone(), brain.clone_brain(), memory.clone()));
    let wm = watchman.clone();
    tokio::spawn(async move {
        if let Err(_) = wm.start().await {}
    });

    // Start Telegram Gateway in background
    if let Some(token) = config.telegram_bot_token.as_ref() {
        if !token.is_empty() {
            if let Ok(tg) = openspore_telegram::TelegramChannel::new() {
                let tg_clone = tg.clone();
                tokio::spawn(async move {
                    let _ = tg_clone.start().await;
                });
            }
        }
    }

    // Start Autonomy Scheduler in background
    if config.autonomy_enabled {
        let brain_clone = brain.clone_brain();
        let memory_clone = memory.clone();
        let tg_opt = openspore_telegram::TelegramChannel::new().ok();
        tokio::spawn(async move {
            openspore_autonomy::SporeScheduler::start(brain_clone, memory_clone, tg_opt).await;
        });
    }

    let res = run_app(&mut terminal, &mut app, brain).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        event::DisableMouseCapture,
        event::DisableBracketedPaste
    )?;
    terminal.show_cursor()?;

    res
}

async fn run_app<B: ratatui::backend::Backend + std::io::Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    brain: Brain,
) -> anyhow::Result<()> {
    let (tx_events, mut rx_events) = mpsc::channel::<BrainEvent>(32);

    let area = terminal.size()?;
    let width = area.width.saturating_sub(4) as usize;

    while let Ok(event) = rx_events.try_recv() {
        app.handle_event(event);
        app.scroll_to_bottom(width);
    }

    loop {
        if app.should_quit {
            return Ok(());
        }

        let area = terminal.size()?;
        let width = area.width.saturating_sub(4) as usize;

        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    match key.code {
                        KeyCode::Esc => app.should_quit = true,
                        KeyCode::Enter => {
                            if key.modifiers.contains(event::KeyModifiers::SHIFT) || key.modifiers.contains(event::KeyModifiers::ALT) {
                                app.input.push('\n');
                            } else {
                                let input = app.input.drain(..).collect::<String>();
                                if !input.trim().is_empty() {
                                    app.add_user_message(input.clone());
                                    app.start_thinking();
                                    app.scroll_to_bottom(width);

                                    let b = brain.clone_brain();
                                    let tx = tx_events.clone();
                                    tokio::spawn(async move {
                                        b.think_with_observer(&input, Some(tx)).await;
                                    });
                                }
                            }
                        }
                        KeyCode::Up => {
                            if key.modifiers.contains(event::KeyModifiers::SHIFT) {
                                for _ in 0..5 { app.previous(width); }
                            } else {
                                app.previous(width);
                            }
                        }
                        KeyCode::Down => {
                            if key.modifiers.contains(event::KeyModifiers::SHIFT) {
                                for _ in 0..5 { app.next(width); }
                            } else {
                                app.next(width);
                            }
                        }
                        KeyCode::Backspace => {
                            app.input.pop();
                        }
                        // Toggle Layer Folding with Space
                        KeyCode::Char(' ') if app.input.is_empty() => {
                            app.toggle_selected_layers(width);
                        }
                        KeyCode::Char('§') if app.input.is_empty() => {
                            app.mouse_captured = !app.mouse_captured;
                            if app.mouse_captured {
                                execute!(terminal.backend_mut(), event::EnableMouseCapture)?;
                            } else {
                                execute!(terminal.backend_mut(), event::DisableMouseCapture)?;
                            }
                        }
                        KeyCode::Char(c) => {
                            app.input.push(c);
                        }
                        _ => {}
                    }
                }
                Event::Mouse(mouse) => {
                    match mouse.kind {
                        event::MouseEventKind::ScrollUp => {
                            let area = terminal.size()?;
                            let width = area.width.saturating_sub(4) as usize;
                            for _ in 0..3 { app.scroll_up(width); }
                        }
                        event::MouseEventKind::ScrollDown => {
                            let area = terminal.size()?;
                            let width = area.width.saturating_sub(4) as usize;
                            for _ in 0..3 { app.scroll_down(width); }
                        }
                        _ => {}
                    }
                }
                Event::Paste(text) => {
                    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
                    app.input.push_str(&normalized);
                }
                _ => {}
            }
        }

        // Handle Brain Events
        let was_at_bottom = app.flat_selection >= app.get_selectable_lines(width).len().saturating_sub(1);
        while let Ok(event) = rx_events.try_recv() {
            app.handle_event(event);
            if was_at_bottom {
                app.scroll_to_bottom(width);
            }
        }
    }
}
