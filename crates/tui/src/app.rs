use openspore_brain::events::BrainEvent;

#[derive(Clone, Debug)]
pub struct ThoughtLayer {
    pub depth: usize,
    pub content: String,
    pub is_collapsed: bool,
    pub wrapped_cache: std::cell::RefCell<Option<(usize, Vec<String>)>>,
}

#[derive(Clone, Debug)]
pub enum MessageAuthor {
    User,
    Ai,
    #[allow(dead_code)]
    System,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SelectableLine {
    Header(usize),
    Content(usize, usize), // (turn_idx, line_idx)
    LayerHeader(usize, usize), // (turn_idx, layer_idx)
    LayerContent(usize, usize, usize), // (turn_idx, layer_idx, line_idx)
    Tool(usize, usize), // (turn_idx, tool_idx)
    Spacing,
}

#[derive(Clone, Debug)]
pub struct MessageTurn {
    pub author: MessageAuthor,
    pub content: String,
    pub layers: Vec<ThoughtLayer>,
    pub active_tools: Vec<(String, String)>, // (name, arg)
    pub is_thinking: bool,
    pub wrapped_cache: std::cell::RefCell<Option<(usize, Vec<String>)>>,
}

pub struct App {
    pub messages: Vec<MessageTurn>,
    pub input: String,
    pub should_quit: bool,
    pub flat_selection: usize,
    pub scroll_offset: usize, // Manual scroll viewport offset
    pub current_path: String,
    pub last_activity: String,
    pub mouse_captured: bool,
    pub scroll_follow_cursor: bool,
}

impl App {
    pub fn new() -> Self {
        let root = openspore_core::path_utils::get_app_root();
        Self {
            messages: Vec::new(),
            input: String::new(),
            should_quit: false,
            flat_selection: 0,
            scroll_offset: 0,
            current_path: root.to_string_lossy().to_string(),
            last_activity: String::from("No recent activity"),
            mouse_captured: true,
            scroll_follow_cursor: true,
        }
    }

    pub fn add_user_message(&mut self, content: String) {
        self.messages.push(MessageTurn {
            author: MessageAuthor::User,
            content,
            layers: Vec::new(),
            active_tools: Vec::new(),
            is_thinking: false,
            wrapped_cache: std::cell::RefCell::new(None),
        });
    }

    pub fn start_thinking(&mut self) {
        self.messages.push(MessageTurn {
            author: MessageAuthor::Ai,
            content: String::from("Thinking..."),
            layers: Vec::new(),
            active_tools: Vec::new(),
            is_thinking: true,
            wrapped_cache: std::cell::RefCell::new(None),
        });
    }

    pub fn handle_event(&mut self, event: BrainEvent) {
        if let Some(last) = self.messages.last_mut() {
            if last.is_thinking {
                match event {
                    BrainEvent::ThoughtLayer { depth, content } => {
                        last.layers.push(ThoughtLayer {
                            depth,
                            content,
                            is_collapsed: true, // Fold layers by default
                            wrapped_cache: std::cell::RefCell::new(None),
                        });
                        last.active_tools.clear(); // Clear tools after a layer finishes (next layer starts)
                    }
                    BrainEvent::ToolExecution { name, arg } => {
                        last.active_tools.push((name, arg));
                    }
                    BrainEvent::ToolResult { name, output: _, success: _ } => {
                        // We could track results, but for now just removing from active tools
                        last.active_tools.retain(|(n, _)| n != &name);
                    }
                    BrainEvent::FinalAnswer(content) => {
                        last.content = content;
                        last.is_thinking = false;
                        last.active_tools.clear();
                        *last.wrapped_cache.borrow_mut() = None; // Invalidate
                    }
                    BrainEvent::Error(e) => {
                        last.content = format!("Error: {}", e);
                        last.is_thinking = false;
                        last.active_tools.clear();
                        *last.wrapped_cache.borrow_mut() = None; // Invalidate
                    }
                }
            }
        }
    }

    pub fn toggle_selected_layers(&mut self, width: usize) {
        let lines = self.get_selectable_lines(width);
        // Toggle based on the line where the magenta selector is (flat_selection)
        if let Some(line) = lines.get(self.flat_selection) {
            match line {
                SelectableLine::LayerHeader(turn_idx, layer_idx) |
                SelectableLine::LayerContent(turn_idx, layer_idx, _) => {
                    if let Some(msg) = self.messages.get_mut(*turn_idx) {
                        if let Some(layer) = msg.layers.get_mut(*layer_idx) {
                            layer.is_collapsed = !layer.is_collapsed;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn next(&mut self, width: usize) {
        self.scroll_follow_cursor = true;
        let lines = self.get_selectable_lines(width);
        let start = self.flat_selection + 1;
        for i in start..lines.len() {
            if matches!(lines[i], SelectableLine::LayerHeader(_, _)) {
                self.flat_selection = i;
                return;
            }
        }
    }

    pub fn previous(&mut self, width: usize) {
        self.scroll_follow_cursor = true;
        let lines = self.get_selectable_lines(width);
        if self.flat_selection == 0 { return; }
        let start = self.flat_selection - 1;
        for i in (0..=start).rev() {
            if matches!(lines[i], SelectableLine::LayerHeader(_, _)) {
                self.flat_selection = i;
                return;
            }
        }
    }

    pub fn scroll_up(&mut self, _width: usize) {
        self.scroll_follow_cursor = false;
        // Direct Viewport Scroll
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_down(&mut self, _width: usize) {
        self.scroll_follow_cursor = false;
        // Direct Viewport Scroll
        self.scroll_offset += 1;
    }

    pub fn scroll_to_bottom(&mut self, width: usize) {
        self.scroll_follow_cursor = true;
        let count = self.get_selectable_lines(width).len();
        if count > 0 {
            self.flat_selection = count - 1;
        }
    }

    pub fn get_selectable_lines(&self, width: usize) -> Vec<SelectableLine> {
        let mut lines = Vec::new();
        for (i, msg) in self.messages.iter().enumerate() {
            lines.push(SelectableLine::Spacing);
            lines.push(SelectableLine::Header(i));

            for (j, layer) in msg.layers.iter().enumerate() {
                lines.push(SelectableLine::LayerHeader(i, j));
                if !layer.is_collapsed {
                    let mut cache = layer.wrapped_cache.borrow_mut();
                    let layer_width = width.saturating_sub(4);
                    let wrapped = if let Some((w, l)) = &*cache {
                        if *w == layer_width { l } else {
                            *cache = Some((layer_width, textwrap::wrap(&layer.content, layer_width).iter().map(|s| s.to_string()).collect()));
                            &cache.as_ref().unwrap().1
                        }
                    } else {
                        *cache = Some((layer_width, textwrap::wrap(&layer.content, layer_width).iter().map(|s| s.to_string()).collect()));
                        &cache.as_ref().unwrap().1
                    };

                    for k in 0..wrapped.len() {
                        lines.push(SelectableLine::LayerContent(i, j, k));
                    }
                }
            }

            for j in 0..msg.active_tools.len() {
                lines.push(SelectableLine::Tool(i, j));
            }

            // AI Final Content (after layers/thinking)
            if !msg.is_thinking || msg.content != "Thinking..." {
                let mut cache = msg.wrapped_cache.borrow_mut();
                let wrapped = if let Some((w, l)) = &*cache {
                    if *w == width { l } else {
                        *cache = Some((width, textwrap::wrap(&msg.content, width).iter().map(|s| s.to_string()).collect()));
                        &cache.as_ref().unwrap().1
                    }
                } else {
                    *cache = Some((width, textwrap::wrap(&msg.content, width).iter().map(|s| s.to_string()).collect()));
                    &cache.as_ref().unwrap().1
                };

                for j in 0..wrapped.len() {
                    lines.push(SelectableLine::Content(i, j));
                }
            }
        }
        lines
    }
}
