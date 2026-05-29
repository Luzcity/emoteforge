//! アプリ状態。コンパイル時埋め込みバイトからカタログを構築して保持する。

use std::sync::Mutex;
use std::time::Duration;

use emoteforge_core::catalog::Catalog;
use emoteforge_core::preview::BridgeConfig;

// データの埋め込み方針は「tracked か generated か」で分ける:
// - catalog.json / emote.schema.json は git 管理下のため、ソース相対の include_bytes! で直接埋め込む。
// - dump_index.json はリリース CI が生成する成果物（未コミット）のため、build.rs が OUT_DIR へ
//   コピーしたものを埋め込む。未生成時は build.rs が `{}` を置くので dump_index なし扱いになる。
static CATALOG_BYTES: &[u8] = include_bytes!("../../catalog/catalog.json");
static DUMP_INDEX_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/dump_index.json"));
// schema はコマンド側（generate_emote）で codex CLI 用に一時ファイルへ書き出して使う。
pub static SCHEMA_BYTES: &[u8] = include_bytes!("../../schema/emote.schema.json");

/// 全コマンドで共有する状態。
pub struct AppState {
    pub catalog: Catalog,
    pub bridge: Mutex<BridgeConfig>,
    pub codex_bin: String,
    pub model: Mutex<Option<String>>,
    pub codex_timeout: Duration,
}

impl AppState {
    /// 埋め込みバイトからカタログを構築する。
    pub fn load() -> Result<Self, String> {
        let catalog = Catalog::load_bytes(CATALOG_BYTES)
            .map_err(|e| format!("failed to load embedded catalog: {e}"))?
            .with_dump_index_bytes(DUMP_INDEX_BYTES)
            .map_err(|e| format!("failed to load embedded dump index: {e}"))?;

        Ok(AppState {
            catalog,
            bridge: Mutex::new(BridgeConfig::default()),
            codex_bin: "codex".to_string(),
            model: Mutex::new(None),
            codex_timeout: Duration::from_secs(180),
        })
    }
}
