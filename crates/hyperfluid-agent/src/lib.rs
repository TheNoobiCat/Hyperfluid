//! C10 Agent Runtime
//!
//! Infinite agent loop, tool provision, system prompt assembly,
//! handoff management, knowledge accumulation, CLI interface.

pub mod config;
pub mod db;
pub mod isolation;
pub mod llm;
pub mod loop_;
pub mod prompt;
pub mod tools;
pub mod types;
