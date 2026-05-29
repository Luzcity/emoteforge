//! Phase 3: AI text-to-motion 生成 → Phase 2 の .ycd パイプラインへ。
//!
//! 検証境界: 実モデル（GPU/HF）は本環境に無く、生成品質・モデル連携は未検証。
//! 生成器は差し替え可能（外部ランナー方式）。

pub mod generator;

pub use generator::{generate_to_ycd_xml, CommandMotionGenerator, MotionGenError, MotionGenerator};
