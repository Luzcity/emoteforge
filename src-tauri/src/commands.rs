//! フロントエンドから呼ぶ Tauri コマンド群。core のロジックを薄くラップする。

use std::path::Path;
use std::time::Duration;

use emoteforge_core::catalog::CatalogEntry;
use emoteforge_core::codex::{CliCodexRunner, Orchestrator};
use emoteforge_core::export::{export, ResourceManifest};
use emoteforge_core::model::EmoteSpec;
use emoteforge_core::preview::{self, install_bridge, BridgeConfig};
use emoteforge_core::validate::{validate, ValidationIssue};
use serde::Serialize;

use crate::state::AppState;

/// 生成/検証の結果。issues が空なら問題なし。
#[derive(Debug, Serialize)]
pub struct GeneratedEmote {
    pub spec: EmoteSpec,
    pub issues: Vec<ValidationIssue>,
}

/// プロンプトから emote を生成し、検証して返す。
#[tauri::command]
pub fn generate_emote(prompt: String, state: tauri::State<'_, AppState>) -> Result<GeneratedEmote, String> {
    let runner = CliCodexRunner {
        binary: state.codex_bin.clone(),
        model: state.model.lock().unwrap().clone(),
        timeout: state.codex_timeout,
        cwd: Some(std::env::temp_dir()),
    };
    let orch = Orchestrator::new(runner, &state.catalog, state.schema_path.clone());
    let spec = orch.generate(&prompt).map_err(|e| e.to_string())?;
    Ok(validate_and_wrap(spec, &state))
}

/// 手動編集後の emote を再検証する。
#[tauri::command]
pub fn validate_emote(spec: EmoteSpec, state: tauri::State<'_, AppState>) -> GeneratedEmote {
    validate_and_wrap(spec, &state)
}

fn validate_and_wrap(spec: EmoteSpec, state: &AppState) -> GeneratedEmote {
    match validate(&spec, &state.catalog) {
        Ok(normalized) => GeneratedEmote { spec: normalized, issues: vec![] },
        Err(report) => GeneratedEmote { spec, issues: report.issues },
    }
}

/// カタログ検索（候補クリップの提示）。
#[tauri::command]
pub fn search_catalog(query: String, limit: usize, state: tauri::State<'_, AppState>) -> Vec<CatalogEntry> {
    state
        .catalog
        .search(&query, limit)
        .into_iter()
        .cloned()
        .collect()
}

/// 編集中の emote を実ゲームでプレビュー再生する。
#[tauri::command]
pub fn preview_emote(spec: EmoteSpec, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let cfg = state.bridge.lock().unwrap().clone();
    preview::preview(&spec, &cfg).map_err(|e| e.to_string())
}

/// プレビュー再生を停止する。
#[tauri::command]
pub fn stop_preview(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let cfg = state.bridge.lock().unwrap().clone();
    preview::stop(&cfg).map_err(|e| e.to_string())
}

/// Preview Bridge の接続先 URL を設定する。
#[tauri::command]
pub fn set_bridge_url(url: String, state: tauri::State<'_, AppState>) {
    let mut cfg = state.bridge.lock().unwrap();
    *cfg = BridgeConfig { base_url: url, timeout: Duration::from_secs(5) };
}

/// 使用する codex モデルを設定する（None でデフォルト）。
#[tauri::command]
pub fn set_codex_model(model: Option<String>, state: tauri::State<'_, AppState>) {
    *state.model.lock().unwrap() = model;
}

/// emote 群をスタンドアロン FiveM リソースとしてエクスポートする。
#[tauri::command]
pub fn export_emotes(
    specs: Vec<EmoteSpec>,
    out_dir: String,
    resource_name: String,
) -> Result<ResourceManifest, String> {
    export(&specs, Path::new(&out_dir), &resource_name).map_err(|e| e.to_string())
}

/// Preview Bridge リソースを FiveM の resources ディレクトリへ書き出す。
#[tauri::command]
pub fn install_bridge_resource(resources_dir: String) -> Result<String, String> {
    install_bridge(Path::new(&resources_dir))
        .map(|p| p.display().to_string())
        .map_err(|e| e.to_string())
}
