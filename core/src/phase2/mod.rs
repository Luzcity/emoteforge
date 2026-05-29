//! Phase 2: 外部モーション取込 → GTA リターゲット → .ycd(.xml) パイプライン。
//!
//! 検証境界: .ycd バイナリ生成は CodeWalker(.NET/Windows) / Blender(Sollumz) が必要で、
//! 本リポジトリの Linux 環境では最終段を検証できない。ここで実装するのは取込/リターゲット/
//! XML 生成までの、テスト可能なロジック。

pub mod bvh;
pub mod motion;
pub mod retarget;
pub mod ycd_xml;

pub use bvh::parse_bvh;
pub use motion::MotionClip;
pub use retarget::{retarget, RetargetResult};
pub use ycd_xml::{build_ycd_xml, motion_to_ycd_xml};
