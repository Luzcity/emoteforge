//! Codex CLI を「振付師」として使うためのオーケストレーション。

pub mod orchestrator;
pub mod prompt;

pub use orchestrator::{CliCodexRunner, CodexError, CodexRunner, Orchestrator};
