//! フロントエンドから呼ぶ Tauri コマンド群。core のロジックを薄くラップする。

use std::io::Write;
use std::net::IpAddr;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use emoteforge_core::catalog::CatalogEntry;
use emoteforge_core::codex::auth::{self, AuthStatus, LoginPrompt};
use tauri::{AppHandle, Emitter};
use emoteforge_core::codex::{CliCodexRunner, Orchestrator};
use emoteforge_core::export::{export, ResourceManifest};
use emoteforge_core::model::EmoteSpec;
use emoteforge_core::phase2::{build_ycd_xml, parse_bvh, retarget};
use emoteforge_core::phase3::{CommandMotionGenerator, MotionGenerator};
use emoteforge_core::preview::{self, install_bridge, BridgeConfig};
use emoteforge_core::validate::{validate, ValidationIssue};
use serde::Serialize;
use tempfile::NamedTempFile;

use crate::state::{AppState, SCHEMA_BYTES};

/// 生成/検証の結果。issues が空なら問題なし。
#[derive(Debug, Serialize)]
pub struct GeneratedEmote {
    pub spec: EmoteSpec,
    pub issues: Vec<ValidationIssue>,
}

/// プロンプトから emote を生成し、検証して返す。
#[tauri::command]
pub fn generate_emote(
    prompt: String,
    state: tauri::State<'_, AppState>,
) -> Result<GeneratedEmote, String> {
    let runner = CliCodexRunner {
        binary: state.codex_bin.clone(),
        model: state.model.lock().unwrap().clone(),
        timeout: state.codex_timeout,
        cwd: Some(std::env::temp_dir()),
    };
    // codex CLI は --output-schema にファイルパスを要求するため、埋め込み schema を
    // 一意な一時ファイルへ書き出す。generate() の間だけ生存し、関数末尾の drop で自動削除される
    // （固定名による衝突・セッション中の取りこぼしを避ける）。
    let mut schema_file =
        NamedTempFile::new().map_err(|e| format!("failed to create schema temp file: {e}"))?;
    schema_file
        .write_all(SCHEMA_BYTES)
        .map_err(|e| format!("failed to write embedded schema: {e}"))?;
    let orch = Orchestrator::new(runner, &state.catalog, schema_file.path().to_path_buf());
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
        Ok(normalized) => GeneratedEmote {
            spec: normalized,
            issues: vec![],
        },
        Err(report) => GeneratedEmote {
            spec,
            issues: report.issues,
        },
    }
}

/// カタログ検索（候補クリップの提示）。
#[tauri::command]
pub fn search_catalog(
    query: String,
    limit: usize,
    state: tauri::State<'_, AppState>,
) -> Vec<CatalogEntry> {
    state
        .catalog
        .search(&query, limit)
        .into_iter()
        .cloned()
        .collect()
}

/// 編集中の emote を実ゲームでプレビュー再生する。
/// 送信前に検証し、無効なら送らない（壊れた emote を実ゲームに送るのを防ぐ）。
#[tauri::command]
pub fn preview_emote(spec: EmoteSpec, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let normalized = validate(&spec, &state.catalog).map_err(|report| {
        let msgs: Vec<String> = report
            .issues
            .iter()
            .map(|i| format!("{}: {}", i.field, i.message))
            .collect();
        format!(
            "無効な emote のためプレビューを中止しました: {}",
            msgs.join("; ")
        )
    })?;
    let cfg = state.bridge.lock().unwrap().clone();
    preview::preview(&normalized, &cfg).map_err(|e| e.to_string())
}

/// プレビュー再生を停止する。
#[tauri::command]
pub fn stop_preview(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let cfg = state.bridge.lock().unwrap().clone();
    preview::stop(&cfg).map_err(|e| e.to_string())
}

/// Preview Bridge の接続先 URL を設定する。
/// localhost / 127.0.0.1 / プライベートネットワーク以外は拒否（emote データの外部送信防止）。
#[tauri::command]
pub fn set_bridge_url(url: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    validate_bridge_url(&url)?;
    let mut cfg = state.bridge.lock().unwrap();
    *cfg = BridgeConfig {
        base_url: url,
        timeout: Duration::from_secs(5),
    };
    Ok(())
}

fn validate_bridge_url(url: &str) -> Result<(), String> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| "bridge URL must use http:// scheme".to_string())?;
    let authority = rest.split('/').next().unwrap_or("");
    // IPv6 はブラケット表記（[::1] / [::1]:8080）。ブラケットを剥がしてからホスト判定する。
    let host = if authority.starts_with('[') {
        authority
            .split(']')
            .next()
            .unwrap_or("")
            .trim_start_matches('[')
    } else {
        authority
            .rsplit_once(':')
            .map(|(h, _)| h)
            .unwrap_or(authority)
    };
    // localhost リテラルは許可。それ以外は IP としてパースし、loopback または
    // RFC1918 プライベートアドレスのみ許可する。`10.evil.com` のような文字列プレフィックス
    // 一致による外部ホスト混入を防ぐため、IP として解釈できないホストは拒否する。
    if host == "localhost" {
        return Ok(());
    }
    let allowed = match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(v4)) => v4.is_loopback() || v4.is_private(),
        Ok(IpAddr::V6(v6)) => v6.is_loopback(),
        Err(_) => false,
    };
    if !allowed {
        return Err(format!(
            "bridge URL host must be localhost or a private/loopback IP, got: {host}"
        ));
    }
    Ok(())
}

/// 使用する codex モデルを設定する（None でデフォルト）。
#[tauri::command]
pub fn set_codex_model(model: Option<String>, state: tauri::State<'_, AppState>) {
    *state.model.lock().unwrap() = model;
}

/// codex のログイン状態（ChatGPT サブスク枠か API キーか）を取得する。
#[tauri::command]
pub fn codex_login_status(state: tauri::State<'_, AppState>) -> Result<AuthStatus, String> {
    auth::login_status(&state.codex_bin).map_err(|e| e.to_string())
}

/// codex のログインを開始する。`method` で方式を選ぶ:
/// - `"browser"` … ブラウザ OAuth（ChatGPT サブスク枠）
/// - `"device"`  … デバイスコード認証（ブラウザが自動で開かない環境向け）
/// - `"apiKey"`  … API キー（`api_key` 必須）
///
/// browser/device はブラウザ操作の完了までブロックする。進行中、認証 URL やワンタイム
/// コードを `codex-login-prompt` イベントでフロントへ送り（手動フォールバック表示用）、
/// 同時にこちらでブラウザを開く（重複起動を避けるため URL は最初の一度だけ開く）。
#[tauri::command]
pub fn codex_login(
    app: AppHandle,
    method: String,
    api_key: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<AuthStatus, String> {
    let bin = state.codex_bin.clone();
    let timeout = Duration::from_secs(300);
    match method.as_str() {
        "apiKey" => {
            let key = api_key.ok_or("API キーが指定されていません")?;
            if key.trim().is_empty() {
                return Err("API キーが空です".to_string());
            }
            auth::login_with_api_key(&bin, &key).map_err(|e| e.to_string())
        }
        "browser" | "device" => {
            let opened = Arc::new(AtomicBool::new(false));
            let on_prompt = move |prompt: LoginPrompt| {
                if let Some(url) = &prompt.url {
                    if !opened.swap(true, Ordering::SeqCst) {
                        auth::open_in_browser(url);
                    }
                }
                let _ = app.emit("codex-login-prompt", &prompt);
            };
            if method == "device" {
                auth::login_device(&bin, timeout, on_prompt).map_err(|e| e.to_string())
            } else {
                auth::login_browser(&bin, timeout, on_prompt).map_err(|e| e.to_string())
            }
        }
        other => Err(format!("未知のログイン方式: {other}")),
    }
}

/// codex の保存済み認証情報を削除する。
#[tauri::command]
pub fn codex_logout(state: tauri::State<'_, AppState>) -> Result<(), String> {
    auth::logout(&state.codex_bin).map_err(|e| e.to_string())
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
    reject_path_traversal(&out_dir)?;
    // 各 emote を検証＆正規化。無効があればまとめて報告して中止。
    let mut normalized = Vec::with_capacity(specs.len());
    let mut errors = Vec::new();
    for spec in &specs {
        match validate(spec, &state.catalog) {
            Ok(n) => normalized.push(n),
            Err(report) => {
                let msgs: Vec<String> = report
                    .issues
                    .iter()
                    .map(|i| format!("{}: {}", i.field, i.message))
                    .collect();
                errors.push(format!("「{}」: {}", spec.display_name, msgs.join("; ")));
            }
        }
    }
    if !errors.is_empty() {
        return Err(format!(
            "無効な emote があるためエクスポートを中止しました:\n{}",
            errors.join("\n")
        ));
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
    reject_path_traversal(&bvh_path)?;
    reject_path_traversal(&out_path)?;
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
/// セキュリティ: runner_bin はファイル名のみ（PATH 解決）を許可し、絶対/相対パスは拒否。
/// 引数は Command::new に argv として直接渡る（シェルを介さない）ため、メタ文字検証は不要。
#[tauri::command]
pub fn generate_ai_motion_ycd_xml(
    prompt: String,
    runner_bin: String,
    runner_args: Vec<String>,
    out_path: String,
) -> Result<YcdBuildResult, String> {
    validate_runner_bin(&runner_bin)?;
    reject_path_traversal(&out_path)?;
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

/// runner_bin は PATH 上のバイナリ名のみ許可。パス区切りを含む場合は拒否。
fn validate_runner_bin(bin: &str) -> Result<(), String> {
    if bin.is_empty() {
        return Err("runner binary name must not be empty".into());
    }
    if bin.contains('/') || bin.contains('\\') {
        return Err(format!(
            "runner binary must be a simple command name (no path separators): {bin}"
        ));
    }
    if bin.contains("..") {
        return Err("runner binary must not contain '..'".into());
    }
    Ok(())
}

/// パスに `..` セグメントが含まれる場合に拒否する。
fn reject_path_traversal(path: &str) -> Result<(), String> {
    let p = Path::new(path);
    for component in p.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err(format!("path must not contain '..': {path}"));
        }
    }
    Ok(())
}
