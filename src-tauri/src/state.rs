//! アプリ状態。コンパイル時埋め込みバイトからカタログ・スキーマを構築して保持する。

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use emoteforge_core::catalog::Catalog;
use emoteforge_core::preview::BridgeConfig;

// 3 ファイルをバイナリに直接埋め込む。
// dump_index.json は CI の generate-dump-index ステップ後に build.rs が OUT_DIR へコピーする。
// ローカルで存在しない場合は build.rs が `{}` を書いておくため、dump_index なし扱いになる。
static CATALOG_BYTES: &[u8] = include_bytes!("../../catalog/catalog.json");
static DUMP_INDEX_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/dump_index.json"));
static SCHEMA_BYTES: &[u8] = include_bytes!("../../schema/emote.schema.json");

/// 全コマンドで共有する状態。
pub struct AppState {
    pub catalog: Catalog,
    pub schema_path: PathBuf,
    pub bridge: Mutex<BridgeConfig>,
    pub codex_bin: String,
    pub model: Mutex<Option<String>>,
    pub codex_timeout: Duration,
}

impl AppState {
    /// 埋め込みバイトからカタログ・スキーマを構築する。
    /// schema は codex CLI へ `--output-schema <path>` で渡すため一時ファイルに書き出す。
    pub fn load() -> Result<Self, String> {
        let catalog = Catalog::load_bytes(CATALOG_BYTES)
            .map_err(|e| format!("failed to load embedded catalog: {e}"))?
            .with_dump_index_bytes(DUMP_INDEX_BYTES)
            .map_err(|e| format!("failed to load embedded dump index: {e}"))?;

        let schema_path = std::env::temp_dir().join("emoteforge_emote.schema.json");
        std::fs::write(&schema_path, SCHEMA_BYTES)
            .map_err(|e| format!("failed to write embedded schema: {e}"))?;

        Ok(AppState {
            catalog,
            schema_path,
            bridge: Mutex::new(BridgeConfig::default()),
            codex_bin: "codex".to_string(),
            model: Mutex::new(None),
            codex_timeout: Duration::from_secs(180),
        })
    }
}
