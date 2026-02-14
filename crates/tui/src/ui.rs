use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
    Frame,
};
use crate::app::{App, MessageAuthor};
use textwrap::wrap;

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let width = area.width as usize;

    // Calculate dynamic input height
    let input_text = format!("❯ {}", app.input);
    let mut wrapped_lines = Vec::new();

    // Explicitly handle newlines first, then wrap each line
    // This allows Shift+Enter (\n) to visually create a new line
    for line in input_text.split('\n') {
        let wrapped = wrap(line, width.saturating_sub(4));
        if wrapped.is_empty() {
             wrapped_lines.push(std::borrow::Cow::Borrowed(""));
        } else {
             wrapped_lines.extend(wrapped);
        }
    }

    let input_height = wrapped_lines.len().max(1) as u16;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12), // Claude Dashboard Header
            Constraint::Min(1),    // Messages Stream
			Constraint::Length(1), // Footer Help
            Constraint::Length(1), // Top Separator
            Constraint::Length(input_height), // Dynamic Input Prompt
            Constraint::Length(1), // Bottom Separator
        ])
        .split(area);

    // --- DASHBOARD HEADER ---
    let header_block = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title("───────────────────── OpenSpore v1.2.2 ──────────────────────")
        .border_style(Style::default().fg(Color::DarkGray));

    let inner_header = header_block.inner(chunks[0]);
    f.render_widget(header_block, chunks[0]);

    let header_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(inner_header);

    // Left Side: ASCII and Identity
    let l_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(4),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(header_layout[0]);

    f.render_widget(Paragraph::new("").alignment(ratatui::layout::Alignment::Center).bold(), l_layout[0]);

    let mushroom = vec![
        Line::from(Span::styled("     .▄▄█████▄▄.     ", Style::default().fg(Color::LightRed))),
        Line::from(Span::styled("   .███▀▀███▀▀███.   ", Style::default().fg(Color::LightRed))),
        Line::from(Span::styled("    ▀▀▀▀▀███▀▀▀▀▀    ", Style::default().fg(Color::Red))),
        Line::from(Span::styled("         ███         ", Style::default().fg(Color::White))),
    ];
    f.render_widget(Paragraph::new(mushroom).alignment(ratatui::layout::Alignment::Center), l_layout[1]);
    f.render_widget(Paragraph::new(format!("{}", app.current_path.replace("/Users/william-mbp", ""))).alignment(ratatui::layout::Alignment::Center).bold(), l_layout[3]);

    // Right Side: Activity and Tips
    let r_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(header_layout[1]);

    f.render_widget(Paragraph::new("│ Recent activity").dim(), r_layout[3]);
    f.render_widget(Paragraph::new(format!("│ {}", app.last_activity)).dim(), r_layout[4]);

    // --- MESSAGES AREA (Bottom-Up Logic) ---
    let mut list_items = Vec::new();
    let message_chunk = chunks[1];
    let msg_width = message_chunk.width.saturating_sub(4) as usize;
    let height = message_chunk.height as usize;

    let selectable_lines = app.get_selectable_lines(msg_width);

    for (idx, line_type) in selectable_lines.iter().enumerate() {
        let is_selected_line = idx == app.flat_selection;
        let is_layer_head = matches!(line_type, crate::app::SelectableLine::LayerHeader(_, _));

        let sidebar = if is_selected_line && is_layer_head {
            Span::styled("┃ ", Style::default().fg(Color::Magenta).bold())
        } else {
            Span::raw("  ")
        };

        match line_type {
            crate::app::SelectableLine::Spacing => {
                list_items.push(ListItem::new(Line::from(vec![sidebar.clone(), Span::raw(" ")])));
            }
            crate::app::SelectableLine::Header(i) => {
                if let Some(msg) = app.messages.get(*i) {
                    let (author_prefix, author_style) = match msg.author {
                        MessageAuthor::User => ("❯ User", Style::default().white().bold()),
                        MessageAuthor::Ai => ("🍄 Spore", Style::default().cyan().bold()),
                        MessageAuthor::System => ("⚙ Sys", Style::default().dark_gray()),
                    };
                    list_items.push(ListItem::new(Line::from(vec![
                        sidebar.clone(),
                        Span::styled(author_prefix, author_style)
                    ])));
                }
            }
            crate::app::SelectableLine::Content(i, j) => {
                if let Some(msg) = app.messages.get(*i) {
                    let cache = msg.wrapped_cache.borrow();
                    if let Some((_, wrapped)) = &*cache {
                        if let Some(line) = wrapped.get(*j) {
                            list_items.push(ListItem::new(Line::from(vec![
                                sidebar.clone(),
                                Span::styled(line.clone(), Style::default().white()),
                            ])));
                        }
                    }
                }
            }
            crate::app::SelectableLine::LayerHeader(i, j) => {
                if let Some(msg) = app.messages.get(*i) {
                    if let Some(layer) = msg.layers.get(*j) {
                        let status_icon = if layer.is_collapsed { "○" } else { "●" };
                        let layer_color = if layer.is_collapsed { Color::DarkGray } else { Color::Magenta };
                        list_items.push(ListItem::new(Line::from(vec![
                            sidebar.clone(),
                            Span::styled(format!("  {} Thinking Layer {}", status_icon, layer.depth), Style::default().fg(layer_color).dim()),
                        ])));
                    }
                }
            }
            crate::app::SelectableLine::LayerContent(i, j, k) => {
                if let Some(msg) = app.messages.get(*i) {
                    if let Some(layer) = msg.layers.get(*j) {
                        let cache = layer.wrapped_cache.borrow();
                        if let Some((_, wrapped)) = &*cache {
                            if let Some(line) = wrapped.get(*k) {
                                list_items.push(ListItem::new(Line::from(vec![
                                    sidebar.clone(),
                                    Span::raw("    "),
                                    Span::styled(line.clone(), Style::default().gray().italic()),
                                ])));
                            }
                        }
                    }
                }
            }
            crate::app::SelectableLine::Tool(i, j) => {
                if let Some(msg) = app.messages.get(*i) {
                    if let Some((name, arg)) = msg.active_tools.get(*j) {
                        list_items.push(ListItem::new(Line::from(vec![
                            sidebar.clone(),
                            Span::styled(format!("  🔧 Running {} ", name), Style::default().yellow()),
                            Span::styled(format!("({})", arg), Style::default().dark_gray()),
                        ])));
                    }
                }
            }
        }
    }

    // --- RENDER LOGIC (Scroll Decoupled) ---
    let total_lines = list_items.len();

    // SAFETY: Clamp selection to valid range.
    // This prevents "blank screen" if collapsing layers reduces line count below cursor position.
    if total_lines > 0 {
        app.flat_selection = app.flat_selection.min(total_lines - 1);
    } else {
        app.flat_selection = 0;
    }

    // Auto-scroll logic: If selector moves out of view, adjust scroll_offset
    let page_size = height;

    if app.scroll_follow_cursor {
        if app.flat_selection < app.scroll_offset {
            app.scroll_offset = app.flat_selection;
        } else if app.flat_selection >= app.scroll_offset + page_size {
            app.scroll_offset = app.flat_selection.saturating_sub(page_size).saturating_add(1);
        }
    }

    // Clamp scroll_offset
    if total_lines > 0 {
        app.scroll_offset = app.scroll_offset.min(total_lines.saturating_sub(1));
    } else {
         app.scroll_offset = 0;
    }

    let visible_items: Vec<ListItem> = list_items.into_iter()
        .skip(app.scroll_offset)
        .take(page_size)
        .collect();

    let messages = List::new(visible_items).style(Style::default().bg(Color::Reset));
    f.render_widget(messages, message_chunk);

    // --- FOOTER / INPUT AREA ---
    let mouse_status = if app.mouse_captured { "ON" } else { "OFF" };
    let footer_text = Line::from(format!("ESC: Quit  •   §: Mouse Scrolling({})  •  ↑↓: Scroll Layers  •  Space: Toggle Layers", mouse_status)).gray();
    f.render_widget(Paragraph::new(footer_text).alignment(ratatui::layout::Alignment::Right), chunks[2]);

    f.render_widget(Paragraph::new("─".repeat(width)).dim(), chunks[3]);

    // Render multi-line input
    let input_para = Paragraph::new(wrapped_lines.iter().map(|s| Line::from(s.to_string())).collect::<Vec<_>>())
        .bold();
    f.render_widget(input_para, chunks[4]);

    f.render_widget(Paragraph::new("─".repeat(width)).dim(), chunks[5]);
}
