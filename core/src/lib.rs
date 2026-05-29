//! emoteforge_core: FiveM エモート開発ソフトのコアロジック。
//! Tauri 非依存の純 Rust ライブラリ。GUI シェル（src-tauri）から利用する。

pub mod catalog;
pub mod codex;
pub mod export;
pub mod model;
pub mod preview;
pub mod validate;

pub use model::{ClipRef, ClipSource, EmoteSpec, Facial, Meta, MovementType, Prop};
