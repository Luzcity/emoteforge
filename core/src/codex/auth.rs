//! Codex CLI のアカウント認証（ChatGPT サブスクログイン / API キー / デバイス認証）。
//!
//! codex には複数のログイン方式がある:
//! - `codex login` … ブラウザ OAuth（ChatGPT サブスク枠）。認証 URL を stderr に出す。
//! - `codex login --device-auth` … デバイスコード認証。URL とワンタイムコードを stdout（ANSI 色付き）に出す。ブラウザが自動で開かない環境向けの公式フォールバック。
//! - `codex login --with-api-key` … stdin から API キーを読む。ブラウザ不要。
//!
//! 状態確認は `codex login status`、解除は `codex logout`。
//!
//! GUI から spawn した子プロセスでは codex 自身のブラウザ自動起動が失敗しやすいため、
//! 出力から認証 URL を拾って自分でブラウザを開く（`open_in_browser`）。

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
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

/// ログイン進行中にユーザーへ提示する情報。
///
/// ブラウザ OAuth では `url` のみ。デバイス認証では `url`（コード入力ページ）と
/// `code`（ワンタイムコード）の両方が埋まる。フロントへイベントで送り、自動で
/// ブラウザが開かない場合の手動フォールバックとして表示する。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginPrompt {
    pub url: Option<String>,
    pub code: Option<String>,
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

/// ANSI エスケープシーケンス（`\x1b[...m` 等の CSI）を取り除く（純関数・テスト対象）。
///
/// codex のデバイス認証出力は色付き（`\x1b[94m...\x1b[0m`）なので、URL/コード抽出前に剥がす。
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // CSI シーケンス: ESC '[' …終端文字(英字)。終端まで読み飛ばす。
            if chars.peek() == Some(&'[') {
                chars.next();
                for n in chars.by_ref() {
                    if n.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// 1 行から OAuth/デバイス認証用の URL を抽出する（純関数・テスト対象）。
///
/// codex はローカルサーバを `http://localhost:1455` に立て、ユーザーが開くべき認証先
/// （`https://auth.openai.com/...`）を案内する。開くべきは後者なので `https://` で始まる
/// トークンのみを拾い、localhost(http) は無視する。
pub fn find_login_url(line: &str) -> Option<String> {
    line.split_whitespace()
        .find_map(|tok| tok.find("https://").map(|i| &tok[i..]))
        .map(|url| {
            url.trim_end_matches(['.', ',', ')', ']', '"', '\''])
                .to_string()
        })
}

/// 1 行からデバイス認証のワンタイムコードを抽出する（純関数・テスト対象）。
///
/// codex のコードは `5HLO-1GL9B` のような「大文字英数＋ハイフン」の塊で、コード行には
/// それ単体が出る（前後にインデントのみ）。誤検出を避けるため、行全体が当該パターンの
/// 場合だけ採用する。
pub fn find_device_code(line: &str) -> Option<String> {
    let t = line.trim();
    let looks_like_code = t.len() >= 7
        && t.contains('-')
        && t.chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-');
    looks_like_code.then(|| t.to_string())
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

/// OS デフォルトブラウザで URL を開く。
///
/// codex 任せの自動起動が GUI 子プロセスで失敗するため、こちらで明示的に開く。
/// 失敗は致命的でない（UI 側でも URL を提示する）ので握りつぶしてログのみ。
/// Windows は `explorer <url>` を使う（`cmd /C start` は OAuth URL 中の `&` を
/// コマンド区切りと誤解しうるため避ける。explorer は単一 argv をそのまま開く）。
pub fn open_in_browser(url: &str) {
    #[cfg(target_os = "windows")]
    let spawned = Command::new("explorer").arg(url).spawn();
    #[cfg(target_os = "macos")]
    let spawned = Command::new("open").arg(url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let spawned = Command::new("xdg-open").arg(url).spawn();

    if let Err(e) = spawned {
        eprintln!(
            "[EmoteForge] ブラウザ起動に失敗しました（手動で URL を開いてください）: {e} url={url}"
        );
    }
}

/// ブラウザ OAuth（ChatGPT サブスク）でログインする。
///
/// 認証 URL を拾い次第 `on_prompt` に渡し（呼び出し側がブラウザを開く）、OAuth コールバック
/// 完了までブロックする。`timeout` 超過で子プロセスを kill する。
pub fn login_browser<F>(
    binary: &str,
    timeout: Duration,
    on_prompt: F,
) -> Result<AuthStatus, CodexError>
where
    F: Fn(LoginPrompt) + Send + Sync + 'static,
{
    run_interactive_login(binary, &[], timeout, on_prompt)
}

/// デバイスコード認証でログインする。
///
/// URL とワンタイムコードを拾い次第 `on_prompt` に渡し（UI でコードを提示）、ユーザーが
/// ブラウザでコードを入力し終えるまでブロックする。`timeout` 超過で kill する。
pub fn login_device<F>(
    binary: &str,
    timeout: Duration,
    on_prompt: F,
) -> Result<AuthStatus, CodexError>
where
    F: Fn(LoginPrompt) + Send + Sync + 'static,
{
    run_interactive_login(binary, &["--device-auth"], timeout, on_prompt)
}

/// API キーでログインする。`codex login --with-api-key` に stdin でキーを渡す。
///
/// キーを argv に載せないのは、プロセス一覧等への漏洩を避けるため。codex がハングしても
/// 無期限ブロックしないよう `timeout` 付きで待機し、超過時は子プロセスを kill する。
pub fn login_with_api_key(
    binary: &str,
    api_key: &str,
    timeout: Duration,
) -> Result<AuthStatus, CodexError> {
    let mut child = Command::new(binary)
        .arg("login")
        .arg("--with-api-key")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(CodexError::Spawn)?;

    {
        // キー＋改行を一度に書く。drop(stdin) で EOF を送る。
        // BrokenPipe（codex が既に終了）は致命的でないので無視し、後段の exit code で判定する。
        let mut stdin = child.stdin.take().expect("piped stdin");
        let mut payload = api_key.trim().as_bytes().to_vec();
        payload.push(b'\n');
        if let Err(e) = stdin.write_all(&payload) {
            if e.kind() != std::io::ErrorKind::BrokenPipe {
                return Err(CodexError::Io(e));
            }
        }
    }

    // タイムアウト付き待機。`--with-api-key` の出力は小さくパイプを詰まらせないため、
    // 終了後にまとめて stderr を読む。
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(CodexError::Io)? {
            break status;
        }
        if start.elapsed() > timeout {
            // タイムアウト確定。kill/wait はベストエフォート（失敗しても返す結果は変わらない）。
            let _ = child.kill();
            let _ = child.wait();
            return Err(CodexError::Timeout(timeout));
        }
        thread::sleep(Duration::from_millis(100));
    };

    if !status.success() {
        let mut stderr = String::new();
        if let Some(mut e) = child.stderr.take() {
            let _ = e.read_to_string(&mut stderr);
        }
        return Err(CodexError::NonZero {
            code: status.code().unwrap_or(-1),
            stderr: stderr.trim().to_string(),
        });
    }
    login_status(binary)
}

/// インタラクティブなログイン（ブラウザ/デバイス）の共通処理。
///
/// stdout/stderr を両方パイプし、別スレッドで走査して URL/コードを拾う。`on_prompt` は
/// 新たな情報が増えるたび累積スナップショットで呼ばれる。drain を怠るとパイプバッファ満杯で
/// codex が書き込みブロックしデッドロックするため、両ストリームを最後まで読み切る。
fn run_interactive_login<F>(
    binary: &str,
    extra_args: &[&str],
    timeout: Duration,
    on_prompt: F,
) -> Result<AuthStatus, CodexError>
where
    F: Fn(LoginPrompt) + Send + Sync + 'static,
{
    let mut cmd = Command::new(binary);
    cmd.arg("login");
    for a in extra_args {
        cmd.arg(a);
    }
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(CodexError::Spawn)?;

    let on_prompt = Arc::new(on_prompt);
    let prompt = Arc::new(Mutex::new(LoginPrompt::default()));
    let mut readers: Vec<JoinHandle<()>> = Vec::new();
    if let Some(out) = child.stdout.take() {
        readers.push(spawn_prompt_scanner(
            out,
            Arc::clone(&on_prompt),
            Arc::clone(&prompt),
        ));
    }
    if let Some(err) = child.stderr.take() {
        readers.push(spawn_prompt_scanner(
            err,
            Arc::clone(&on_prompt),
            Arc::clone(&prompt),
        ));
    }

    let start = Instant::now();
    let outcome = loop {
        if let Some(status) = child.try_wait().map_err(CodexError::Io)? {
            break Ok(status);
        }
        if start.elapsed() > timeout {
            // タイムアウト確定。kill/wait はベストエフォート（失敗しても返す結果は変わらない）。
            let _ = child.kill();
            let _ = child.wait();
            break Err(CodexError::Timeout(timeout));
        }
        thread::sleep(Duration::from_millis(200));
    };

    // 子が終了/kill されればパイプが閉じてリーダーは自然終了する。回収する。
    // リーダーは行を読んで callback を呼ぶだけで、join 失敗（スレッド panic）から復帰する
    // 必要はないため無視する。
    for r in readers {
        let _ = r.join();
    }

    let status = outcome?;
    if !status.success() {
        return Err(CodexError::NonZero {
            code: status.code().unwrap_or(-1),
            stderr: "codex login が失敗しました".to_string(),
        });
    }
    login_status(binary)
}

/// ストリームを行単位で走査し、URL/コードを見つけたら累積して `on_prompt` を呼ぶスレッドを起こす。
fn spawn_prompt_scanner<R, F>(
    stream: R,
    on_prompt: Arc<F>,
    prompt: Arc<Mutex<LoginPrompt>>,
) -> JoinHandle<()>
where
    R: Read + Send + 'static,
    F: Fn(LoginPrompt) + Send + Sync + 'static,
{
    thread::spawn(move || {
        let reader = BufReader::new(stream);
        for line in reader.lines() {
            let Ok(line) = line else { break };
            let clean = strip_ansi(&line);
            let url = find_login_url(&clean);
            let code = find_device_code(&clean);
            if url.is_none() && code.is_none() {
                continue;
            }
            let snapshot = {
                let mut p = prompt.lock().unwrap();
                let mut changed = false;
                if url.is_some() && p.url.is_none() {
                    p.url = url;
                    changed = true;
                }
                if code.is_some() && p.code.is_none() {
                    p.code = code;
                    changed = true;
                }
                if !changed {
                    continue;
                }
                p.clone()
            };
            on_prompt(snapshot);
        }
    })
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

    #[test]
    fn strip_ansi_removes_color_codes() {
        let s = strip_ansi("\u{1b}[94mhttps://auth.openai.com/codex/device\u{1b}[0m");
        assert_eq!(s, "https://auth.openai.com/codex/device");
    }

    #[test]
    fn strip_ansi_keeps_plain_text() {
        assert_eq!(
            strip_ansi("plain text, no escapes"),
            "plain text, no escapes"
        );
    }

    #[test]
    fn find_login_url_picks_https_over_localhost() {
        // ブラウザ OAuth: ローカルサーバ(http)ではなく認証先(https)を選ぶ。
        let line = "navigate to https://auth.openai.com/oauth/authorize?a=1&b=2 to authenticate";
        assert_eq!(
            find_login_url(line).as_deref(),
            Some("https://auth.openai.com/oauth/authorize?a=1&b=2")
        );
        assert_eq!(
            find_login_url("Starting server on http://localhost:1455."),
            None
        );
    }

    #[test]
    fn find_login_url_trims_trailing_punctuation() {
        assert_eq!(
            find_login_url("see (https://example.com/x).").as_deref(),
            Some("https://example.com/x")
        );
    }

    #[test]
    fn find_device_code_matches_one_time_code() {
        assert_eq!(
            find_device_code("   5HLO-1GL9B").as_deref(),
            Some("5HLO-1GL9B")
        );
        assert_eq!(find_device_code("ABCD-1234").as_deref(), Some("ABCD-1234"));
    }

    #[test]
    fn find_device_code_ignores_urls_and_prose() {
        assert_eq!(
            find_device_code("https://auth.openai.com/codex/device"),
            None
        );
        assert_eq!(find_device_code("Enter this one-time code"), None);
        assert_eq!(find_device_code("ChatGPT"), None);
        // ハイフンなし・短すぎは不採用。
        assert_eq!(find_device_code("ABCDEFG"), None);
        assert_eq!(find_device_code("A-B"), None);
    }
}
