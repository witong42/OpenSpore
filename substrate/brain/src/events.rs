use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrainEvent {
    ThoughtLayer {
        depth: usize,
        content: String,
    },
    ToolExecution {
        name: String,
        arg: String,
    },
    ToolResult {
        name: String,
        output: String,
        success: bool,
    },
    FinalAnswer(String),
    Error(String),
}
