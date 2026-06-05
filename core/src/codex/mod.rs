//! Codex CLI を「振付師」として使うためのオーケストレーション。

pub mod auth;
pub mod orchestrator;
pub mod prompt;

pub use auth::AuthStatus;
pub use orchestrator::{CliCodexRunner, CodexError, CodexRunner, Orchestrator};
