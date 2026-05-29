//! 実ゲーム(FiveM)ライブプレビュー。
//! - bridge_client: 編集中の EmoteSpec を localhost の Bridge resource へ POST。
//! - install: Bridge resource のファイル群を書き出す。

pub mod http;

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::export::lua_templates::{bridge_client_lua, BRIDGE_FXMANIFEST, BRIDGE_SERVER_LUA};
use crate::model::EmoteSpec;

#[derive(Debug, thiserror::Error)]
pub enum PreviewError {
    #[error("serialize error: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("http error: {0}")]
    Http(#[from] http::HttpError),
    #[error("bridge returned status {0}: {1}")]
    BadStatus(u16, String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Preview Bridge への接続設定。base_url 例: http://localhost:30120/emoteforge_bridge
#[derive(Debug, Clone)]
pub struct BridgeConfig {
    pub base_url: String,
    pub timeout: Duration,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        BridgeConfig {
            base_url: "http://localhost:30120/emoteforge_bridge".to_string(),
            timeout: Duration::from_secs(5),
        }
    }
}

/// 編集中の emote を実ゲームで再生する。
pub fn preview(spec: &EmoteSpec, cfg: &BridgeConfig) -> Result<(), PreviewError> {
    let body = serde_json::to_string(spec)?;
    let url = format!("{}/preview", cfg.base_url.trim_end_matches('/'));
    let (status, resp) = http::post_json(&url, &body, cfg.timeout)?;
    if !(200..300).contains(&status) {
        return Err(PreviewError::BadStatus(status, resp));
    }
    Ok(())
}

/// 再生中の emote を停止する。
pub fn stop(cfg: &BridgeConfig) -> Result<(), PreviewError> {
    let url = format!("{}/stop", cfg.base_url.trim_end_matches('/'));
    let (status, resp) = http::post_json(&url, "{}", cfg.timeout)?;
    if !(200..300).contains(&status) {
        return Err(PreviewError::BadStatus(status, resp));
    }
    Ok(())
}

/// Bridge resource のファイル群を `resources_dir/emoteforge_bridge/` に書き出す。
/// FiveM サーバーの resources ディレクトリに置き、`ensure emoteforge_bridge` で有効化する。
pub fn install_bridge(resources_dir: &Path) -> Result<PathBuf, PreviewError> {
    let dir = resources_dir.join("emoteforge_bridge");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("fxmanifest.lua"), BRIDGE_FXMANIFEST)?;
    std::fs::write(dir.join("server.lua"), BRIDGE_SERVER_LUA)?;
    std::fs::write(dir.join("client.lua"), bridge_client_lua())?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ClipRef, ClipSource, Meta, MovementType};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    fn sample() -> EmoteSpec {
        EmoteSpec {
            name: "cheer".into(),
            display_name: "乾杯".into(),
            clips: vec![ClipRef {
                source: ClipSource::Builtin,
                dict: "amb@world_human_cheering@male_a".into(),
                clip: "base".into(),
                blend_in: 1.0,
                blend_out: 1.0,
                duration: -1,
                playback_rate: 1.0,
                flags: vec![],
            }],
            looping: true,
            upper_body_only: false,
            prop: None,
            facial: None,
            movement_type: MovementType::Stationary,
            meta: Meta::default(),
        }
    }

    /// 1 リクエストだけ受ける簡易サーバを立て、受信ボディを返す。
    fn one_shot_server(response: &'static str) -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let body = req.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
                let _ = tx.send(body);
                let _ = stream.write_all(response.as_bytes());
            }
        });
        (format!("http://127.0.0.1:{port}"), rx)
    }

    #[test]
    fn preview_posts_emote_json() {
        let (base, rx) = one_shot_server("HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\n{\"ok\":true}");
        let cfg = BridgeConfig { base_url: base, timeout: Duration::from_secs(2) };
        preview(&sample(), &cfg).unwrap();
        let received = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let parsed: EmoteSpec = serde_json::from_str(&received).unwrap();
        assert_eq!(parsed.name, "cheer");
    }

    #[test]
    fn preview_errors_on_non_2xx() {
        let (base, _rx) = one_shot_server("HTTP/1.1 400 Bad Request\r\nContent-Length: 2\r\n\r\nno");
        let cfg = BridgeConfig { base_url: base, timeout: Duration::from_secs(2) };
        let err = preview(&sample(), &cfg).unwrap_err();
        assert!(matches!(err, PreviewError::BadStatus(400, _)));
    }

    #[test]
    fn preview_errors_when_unreachable() {
        // 使われていないポートへ接続 → Connect エラー。
        let cfg = BridgeConfig {
            base_url: "http://127.0.0.1:1".to_string(),
            timeout: Duration::from_millis(500),
        };
        assert!(preview(&sample(), &cfg).is_err());
    }

    #[test]
    fn install_bridge_writes_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = install_bridge(tmp.path()).unwrap();
        assert!(dir.join("fxmanifest.lua").exists());
        assert!(dir.join("server.lua").exists());
        assert!(dir.join("client.lua").exists());
    }
}
