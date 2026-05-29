//! フロントエンドから呼ぶ Tauri コマンド群。core のロジックを薄くラップする。

use std::path::Path;
use std::time::Duration;

use emoteforge_core::catalog::CatalogEntry;
use emoteforge_core::codex::{CliCodexRunner, Orchestrator};
use emoteforge_core::export::{export, ResourceManifest};
use emoteforge_core::model::EmoteSpec;
use emoteforge_core::phase2::{build_ycd_xml, parse_bvh, retarget};
use emoteforge_core::phase3::{CommandMotionGenerator, MotionGenerator};
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
/// エクスポート前に全 emote を検証し、1 つでも無効なら中止する（無効リソースの生成防止）。
#[tauri::command]
pub fn export_emotes(
    specs: Vec<EmoteSpec>,
    out_dir: String,
    resource_name: String,
    state: tauri::State<'_, AppState>,
) -> Result<ResourceManifest, String> {
    // 各 emote を検証＆正規化。無効があればまとめて報告して中止。
    let mut normalized = Vec::with_capacity(specs.len());
    let mut errors = Vec::new();
    for spec in &specs {
        match validate(spec, &state.catalog) {
            Ok(n) => normalized.push(n),
            Err(report) => {
                let msgs: Vec<String> = report.issues.iter().map(|i| format!("{}: {}", i.field, i.message)).collect();
                errors.push(format!("「{}」: {}", spec.display_name, msgs.join("; ")));
            }
        }
    }
    if !errors.is_empty() {
        return Err(format!("無効な emote があるためエクスポートを中止しました:\n{}", errors.join("\n")));
    }
    export(&normalized, Path::new(&out_dir), &resource_name).map_err(|e| e.to_string())
}

/// Preview Bridge リソースを FiveM の resources ディレクトリへ書き出す。
#[tauri::command]
pub fn install_bridge_resource(resources_dir: String) -> Result<String, String> {
    install_bridge(Path::new(&resources_dir))
        .map(|p| p.display().to_string())
        .map_err(|e| e.to_string())
}

// ---- Phase 2/3: .ycd パイプライン ----

/// .ycd.xml 生成結果の要約。
#[derive(Debug, Serialize)]
pub struct YcdBuildResult {
    pub out_path: String,
    pub frame_count: usize,
    pub bone_count: usize,
    /// GTA ボーンへ対応付けできなかった元ボーン名（情報提示用）。
    pub unmapped: Vec<String>,
}

fn file_stem(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("clip")
        .to_string()
}

/// BVH を取り込み GTA リターゲット → CodeWalker `.ycd.xml` を書き出す。
/// （`.ycd` バイナリ化は CodeWalker/Sollumz が別途必要）
#[tauri::command]
pub fn import_bvh_ycd_xml(bvh_path: String, out_path: String) -> Result<YcdBuildResult, String> {
    let text = std::fs::read_to_string(&bvh_path).map_err(|e| e.to_string())?;
    let clip = parse_bvh(&text, &file_stem(&bvh_path)).map_err(|e| e.to_string())?;
    let rt = retarget(&clip);
    std::fs::write(&out_path, build_ycd_xml(&rt)).map_err(|e| e.to_string())?;
    Ok(YcdBuildResult {
        out_path,
        frame_count: rt.clip.frame_count(),
        bone_count: rt.bone_tags.len(),
        unmapped: rt.unmapped,
    })
}

/// 外部 text-to-motion ランナーで生成 → GTA リターゲット → `.ycd.xml` 書き出し。
/// runner はプロンプトを stdin で受け、MotionClip JSON を stdout に出すコマンド。
#[tauri::command]
pub fn generate_ai_motion_ycd_xml(
    prompt: String,
    runner_bin: String,
    runner_args: Vec<String>,
    out_path: String,
) -> Result<YcdBuildResult, String> {
    let gen = CommandMotionGenerator::new(runner_bin, runner_args);
    let clip = gen.generate(&prompt).map_err(|e| e.to_string())?;
    let rt = retarget(&clip);
    std::fs::write(&out_path, build_ycd_xml(&rt)).map_err(|e| e.to_string())?;
    Ok(YcdBuildResult {
        out_path,
        frame_count: rt.clip.frame_count(),
        bone_count: rt.bone_tags.len(),
        unmapped: rt.unmapped,
    })
}
