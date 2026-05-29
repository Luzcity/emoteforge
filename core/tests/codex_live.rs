//! 実 codex CLI に対するライブ統合テスト。
//! ネットワーク・サブスク認証・時間を要するため通常は #[ignore]。
//! 実行: `cargo test -p emoteforge_core --test codex_live -- --ignored --nocapture`

use std::path::{Path, PathBuf};
use std::time::Duration;

use emoteforge_core::catalog::Catalog;
use emoteforge_core::codex::{CliCodexRunner, Orchestrator};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

#[test]
#[ignore = "requires codex CLI + ChatGPT login + network"]
fn live_generate_cheer_emote() {
    let catalog = Catalog::load(&repo_root().join("catalog/catalog.json")).unwrap();
    let runner = CliCodexRunner {
        binary: "codex".into(),
        model: None,
        timeout: Duration::from_secs(180),
        cwd: Some(repo_root()),
    };
    let orch = Orchestrator::new(runner, &catalog, repo_root().join("schema/emote.schema.json"));

    let spec = orch
        .generate("プレイヤーが嬉しそうに乾杯して喜ぶエモート")
        .expect("codex generates a valid EmoteSpec");

    assert!(!spec.name.is_empty());
    assert!(!spec.clips.is_empty());
    assert_eq!(spec.meta.source, "codex");
    println!("generated: {}", serde_json::to_string_pretty(&spec).unwrap());
}
