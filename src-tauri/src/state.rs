//! アプリ状態。起動時にカタログ・スキーマを解決して保持する。

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use emoteforge_core::catalog::Catalog;
use emoteforge_core::preview::BridgeConfig;
use tauri::{AppHandle, Manager};

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
    /// リソース(バンドル) → dev フォールバックの順でデータを解決して構築する。
    pub fn load(handle: &AppHandle) -> Result<Self, String> {
        // tauri.conf.json は `../catalog/...` を列挙しており、バンドル時は `_up_/` に再マップされる
        // （$RESOURCE/_up_/catalog/catalog.json）。バンドル/開発の双方を網羅するため候補を順に試す。
        let catalog_path = resolve(
            handle,
            &["_up_/catalog/catalog.json", "catalog/catalog.json"],
            "../catalog/catalog.json",
        );
        let dump_index_path = resolve(
            handle,
            &["_up_/catalog/dump_index.json", "catalog/dump_index.json"],
            "../catalog/dump_index.json",
        );
        let schema_path = resolve(
            handle,
            &["_up_/schema/emote.schema.json", "schema/emote.schema.json"],
            "../schema/emote.schema.json",
        );

        let mut catalog = Catalog::load(&catalog_path)
            .map_err(|e| format!("failed to load catalog ({}): {e}", catalog_path.display()))?;
        // dump_index は任意（存在検証を厳密化）。
        catalog = catalog
            .with_dump_index(&dump_index_path)
            .map_err(|e| format!("failed to load dump index: {e}"))?;

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

/// バンドルリソースのパスを解決。複数候補を順に試し、最後に dev フォールバック
/// （CARGO_MANIFEST_DIR 相対）を返す。
fn resolve(handle: &AppHandle, resource_candidates: &[&str], dev_rel: &str) -> PathBuf {
    for rel in resource_candidates {
        if let Ok(p) = handle
            .path()
            .resolve(rel, tauri::path::BaseDirectory::Resource)
        {
            if p.exists() {
                return p;
            }
        }
    }
    // resource_dir 直下にフラット配置されたケースも一応見る。
    if let Ok(res_dir) = handle.path().resource_dir() {
        for rel in resource_candidates {
            let fname = Path::new(rel).file_name();
            if let Some(f) = fname {
                let p = res_dir.join(f);
                if p.exists() {
                    return p;
                }
            }
        }
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join(dev_rel)
}
