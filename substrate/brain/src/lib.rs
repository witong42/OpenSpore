//! OpenSpore Brain - Modular Cognitive Engine
//!
//! This module is organized into:
//! - types: Core data structures
//! - api: OpenRouter API communication with retry logic
//! - thinking: Main think() loop with tool execution
//! - learning: Knowledge/preference extraction
//! - compression: Session history compression
//! - context_assembler: System prompt construction

mod api;
mod thinking;
mod learning;
mod compression;
mod context_assembler;
mod parser;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

use std::sync::Arc;
use openspore_core::config::AppConfig;
use openspore_skills::SkillLoader;
use openspore_memory::MemorySystem;
use openspore_io::NativeBridge;

/// The Cognitive Engine of the Swarm
#[derive(Clone)]
pub struct Brain {
    pub client: reqwest::Client,
    pub config: AppConfig,
    pub skill_loader: Arc<SkillLoader>,
    pub memory: MemorySystem,
    pub context_manager: openspore_memory::context::ContextManager,
    pub io: Arc<Box<dyn NativeBridge + Send + Sync>>,
}

impl Brain {
    pub fn new(config: AppConfig) -> Self {
        let state = openspore_core::state::AppState::new(config.clone());
        let memory = MemorySystem::new(&state);
        let context_manager = openspore_memory::context::ContextManager::new(&state);

        Self {
            client: reqwest::Client::new(),
            config,
            skill_loader: Arc::new(SkillLoader::new()),
            memory,
            context_manager,
            io: Arc::new(openspore_io::get_bridge()),
        }
    }

    pub fn clone_brain(&self) -> Self {
        self.clone()
    }
}
