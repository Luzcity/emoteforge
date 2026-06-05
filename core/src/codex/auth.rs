//! Codex CLI のアカウント認証（ChatGPT サブスクログイン）。
//!
//! codex は `codex login`（ブラウザ OAuth）で ChatGPT サブスク枠にログインできる。
//! 状態は `codex login status`、解除は `codex logout`。ここではそれらを薄くラップする。

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::Serialize;

use super::orchestrator::CodexError;

/// codex のログイン状態。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    /// ログイン済みか。
    pub logged_in: bool,
    /// ログイン方式（"ChatGPT" = サブスク枠、"API key" = APIキー）。判別不能なら None。
    pub method: Option<String>,
    /// CLI が返した生メッセージ（UI 表示用）。
    pub detail: String,
}

/// `codex login status` の出力からログイン状態を解釈する（純関数・テスト対象）。
///
/// 未ログイン時の正確な文言は環境依存なので、ログイン判定は exit code を主、
/// "logged in" の有無を従とする。方式は detail の内容から推測する。
pub fn parse_login_status(success: bool, stdout: &str, stderr: &str) -> AuthStatus {
    let combined = if stdout.trim().is_empty() {
        stderr
    } else {
        stdout
    };
    let detail = combined.trim().to_string();
    let logged_in = success && detail.to_lowercase().contains("logged in");
    let method = if logged_in {
        if detail.contains("ChatGPT") {
            Some("ChatGPT".to_string())
        } else if detail.to_lowercase().contains("api key") {
            Some("API key".to_string())
        } else {
            None
        }
    } else {
        None
    };
    AuthStatus {
        logged_in,
        method,
        detail,
    }
}

/// `codex login status` を実行して状態を返す。
pub fn login_status(binary: &str) -> Result<AuthStatus, CodexError> {
    let output = Command::new(binary)
        .arg("login")
        .arg("status")
        .stdin(Stdio::null())
        .output()
        .map_err(CodexError::Spawn)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    Ok(parse_login_status(
        output.status.success(),
        &stdout,
        &stderr,
    ))
}

/// `codex login`（ブラウザ OAuth、ChatGPT サブスク）を実行する。
///
/// ブラウザでの操作完了までブロックするため `timeout` を長めに取り、超過時は子を kill する。
/// stdio はパイプせず破棄する（パイプ読み取りのデッドロックを避ける。codex は自前で
/// ブラウザを開く）。完了後に最新のログイン状態を返す。
pub fn login(binary: &str, timeout: Duration) -> Result<AuthStatus, CodexError> {
    let mut child = Command::new(binary)
        .arg("login")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(CodexError::Spawn)?;

    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(CodexError::Io)? {
            break status;
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            return Err(CodexError::Timeout(timeout));
        }
        std::thread::sleep(Duration::from_millis(200));
    };

    if !status.success() {
        return Err(CodexError::NonZero {
            code: status.code().unwrap_or(-1),
            stderr: "codex login が失敗しました".to_string(),
        });
    }
    login_status(binary)
}

/// `codex logout` を実行して認証情報を削除する。
pub fn logout(binary: &str) -> Result<(), CodexError> {
    let output = Command::new(binary)
        .arg("logout")
        .stdin(Stdio::null())
        .output()
        .map_err(CodexError::Spawn)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CodexError::NonZero {
            code: output.status.code().unwrap_or(-1),
            stderr: stderr.trim().to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chatgpt_subscription_login() {
        let s = parse_login_status(true, "Logged in using ChatGPT\n", "");
        assert!(s.logged_in);
        assert_eq!(s.method.as_deref(), Some("ChatGPT"));
        assert_eq!(s.detail, "Logged in using ChatGPT");
    }

    #[test]
    fn parses_api_key_login() {
        let s = parse_login_status(true, "Logged in using an API key", "");
        assert!(s.logged_in);
        assert_eq!(s.method.as_deref(), Some("API key"));
    }

    #[test]
    fn not_logged_in_by_nonzero_exit() {
        // 文言に依存せず exit code でログアウト判定できること。
        let s = parse_login_status(false, "", "Not logged in");
        assert!(!s.logged_in);
        assert_eq!(s.method, None);
        assert_eq!(s.detail, "Not logged in");
    }

    #[test]
    fn logged_in_without_recognized_method_has_none() {
        let s = parse_login_status(true, "Logged in", "");
        assert!(s.logged_in);
        assert_eq!(s.method, None);
    }

    #[test]
    fn exit_zero_without_logged_in_phrase_is_not_logged_in() {
        // exit 0 でも "logged in" を含まなければ未ログイン扱い。
        let s = parse_login_status(true, "Some unrelated output", "");
        assert!(!s.logged_in);
    }
}
