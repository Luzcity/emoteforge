//! Codex 実行の抽象（CodexRunner）と、プロンプト→EmoteSpec を担う Orchestrator。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::catalog::{Catalog, CatalogEntry};
use crate::codex::prompt::build_prompt;
use crate::model::EmoteSpec;

#[derive(Debug, thiserror::Error)]
pub enum CodexError {
    #[error("failed to spawn codex: {0}")]
    Spawn(#[source] std::io::Error),
    #[error("codex exited with status {code}: {stderr}")]
    NonZero { code: i32, stderr: String },
    #[error("codex timed out after {0:?}")]
    Timeout(Duration),
    #[error("codex returned empty output")]
    Empty,
    #[error("failed to parse EmoteSpec from codex output: {0}")]
    Parse(#[source] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[source] std::io::Error),
}

/// codex 実行を抽象化（テストでモック可能）。最終メッセージ(JSON 文字列)を返す。
pub trait CodexRunner {
    fn run(&self, prompt: &str, schema_path: &Path) -> Result<String, CodexError>;
}

/// 実 codex CLI を `codex exec` で叩く実装。
pub struct CliCodexRunner {
    pub binary: String,
    pub model: Option<String>,
    pub timeout: Duration,
    /// codex を実行する作業ディレクトリ。
    pub cwd: Option<PathBuf>,
}

impl Default for CliCodexRunner {
    fn default() -> Self {
        CliCodexRunner {
            binary: "codex".to_string(),
            model: None,
            timeout: Duration::from_secs(120),
            cwd: None,
        }
    }
}

impl CodexRunner for CliCodexRunner {
    fn run(&self, prompt: &str, schema_path: &Path) -> Result<String, CodexError> {
        // 最終メッセージの出力先（一時ファイル）。
        let out_file =
            std::env::temp_dir().join(format!("emoteforge_codex_{}.json", std::process::id()));

        let mut cmd = Command::new(&self.binary);
        cmd.arg("exec")
            .arg("--skip-git-repo-check")
            .arg("--output-schema")
            .arg(schema_path)
            .arg("-o")
            .arg(&out_file);
        if let Some(model) = &self.model {
            cmd.arg("--model").arg(model);
        }
        if let Some(cwd) = &self.cwd {
            cmd.arg("--cd").arg(cwd);
        }
        // プロンプトは stdin で渡す（引数長/エスケープ事故を避ける）。
        cmd.arg("-");
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(CodexError::Spawn)?;
        {
            use std::io::Write;
            let mut stdin = child.stdin.take().expect("piped stdin");
            if let Err(e) = stdin.write_all(prompt.as_bytes()) {
                if e.kind() != std::io::ErrorKind::BrokenPipe {
                    return Err(CodexError::Io(e));
                }
            }
        }

        // タイムアウト付き待機。
        let start = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().map_err(CodexError::Io)? {
                break status;
            }
            if start.elapsed() > self.timeout {
                let _ = child.kill();
                return Err(CodexError::Timeout(self.timeout));
            }
            std::thread::sleep(Duration::from_millis(100));
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

        // -o ファイルから最終メッセージを読む。無ければ stdout を試す。
        let output = if out_file.exists() {
            let s = std::fs::read_to_string(&out_file).map_err(CodexError::Io)?;
            let _ = std::fs::remove_file(&out_file);
            s
        } else {
            let mut s = String::new();
            if let Some(mut o) = child.stdout.take() {
                o.read_to_string(&mut s).map_err(CodexError::Io)?;
            }
            s
        };

        if output.trim().is_empty() {
            return Err(CodexError::Empty);
        }
        Ok(output)
    }
}

/// プロンプト → カタログ文脈付与 → codex 実行 → EmoteSpec。
pub struct Orchestrator<'a, R: CodexRunner> {
    runner: R,
    catalog: &'a Catalog,
    schema_path: PathBuf,
    /// プロンプトに含める候補クリップ数。
    pub candidate_limit: usize,
}

impl<'a, R: CodexRunner> Orchestrator<'a, R> {
    pub fn new(runner: R, catalog: &'a Catalog, schema_path: PathBuf) -> Self {
        Orchestrator {
            runner,
            catalog,
            schema_path,
            candidate_limit: 40,
        }
    }

    pub fn generate(&self, user_prompt: &str) -> Result<EmoteSpec, CodexError> {
        let candidates = self.candidates_for(user_prompt);
        let prompt = build_prompt(user_prompt, &candidates);
        let raw = self.runner.run(&prompt, &self.schema_path)?;
        let spec = parse_spec(&raw)?;
        Ok(spec)
    }

    /// 検索ヒットを集め、薄い場合（日本語プロンプト等で英語タグに当たらない時）のみ
    /// カテゴリ横断の多様サンプルで「下限」まで底上げする。意味あるヒットは薄めない。
    fn candidates_for(&self, user_prompt: &str) -> Vec<&CatalogEntry> {
        let mut hits = self.catalog.search(user_prompt, self.candidate_limit);
        // ヒットが下限未満のときだけ補充（常に 40 まで埋めてノイズ化させない）。
        let target = hits
            .len()
            .max(Self::CANDIDATE_FLOOR)
            .min(self.candidate_limit);
        if hits.len() < target {
            let mut seen: std::collections::HashSet<&str> =
                hits.iter().map(|e| e.key.as_str()).collect();
            for e in self.catalog.diverse_sample(target) {
                if hits.len() >= target {
                    break;
                }
                if seen.insert(e.key.as_str()) {
                    hits.push(e);
                }
            }
        }
        hits
    }

    /// 検索が薄い時に確保する候補数の下限。
    const CANDIDATE_FLOOR: usize = 16;
}

/// codex 出力（JSON 文字列、前後にノイズがあり得る）から EmoteSpec を取り出す。
fn parse_spec(raw: &str) -> Result<EmoteSpec, CodexError> {
    // 厳密パースを試し、ダメなら最初の `{` 〜 最後の `}` を抽出して再試行。
    match serde_json::from_str::<EmoteSpec>(raw) {
        Ok(spec) => Ok(spec),
        Err(_) => {
            let start = raw.find('{');
            let end = raw.rfind('}');
            if let (Some(s), Some(e)) = (start, end) {
                if e > s {
                    return serde_json::from_str::<EmoteSpec>(&raw[s..=e])
                        .map_err(CodexError::Parse);
                }
            }
            serde_json::from_str::<EmoteSpec>(raw).map_err(CodexError::Parse)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn catalog() -> Catalog {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../catalog/catalog.json");
        Catalog::load(&p).unwrap()
    }

    struct MockRunner {
        response: String,
    }
    impl CodexRunner for MockRunner {
        fn run(&self, _prompt: &str, _schema: &Path) -> Result<String, CodexError> {
            Ok(self.response.clone())
        }
    }

    const VALID: &str = r#"{
      "name":"cheer","displayName":"乾杯","clips":[{"dict":"amb@world_human_cheering@male_a","clip":"base"}],
      "loop":true,"upperBodyOnly":false,"movementType":"stationary","meta":{"source":"codex","schemaVersion":1}
    }"#;

    #[test]
    fn generate_parses_valid_response() {
        let cat = catalog();
        let orch = Orchestrator::new(
            MockRunner {
                response: VALID.to_string(),
            },
            &cat,
            PathBuf::from("schema/emote.schema.json"),
        );
        let spec = orch.generate("乾杯して喜ぶ").unwrap();
        assert_eq!(spec.name, "cheer");
        assert_eq!(spec.clips.len(), 1);
        assert_eq!(spec.clips[0].clip, "base");
    }

    #[test]
    fn parse_spec_strips_surrounding_noise() {
        let noisy = format!("here is your emote:\n```json\n{}\n```\n", VALID);
        let spec = parse_spec(&noisy).unwrap();
        assert_eq!(spec.name, "cheer");
    }

    #[test]
    fn japanese_prompt_still_gets_candidates() {
        // 英語タグに当たらない日本語でも diverse_sample で候補が底上げされる。
        let cat = catalog();
        let orch = Orchestrator::new(
            MockRunner {
                response: VALID.to_string(),
            },
            &cat,
            PathBuf::from("schema/emote.schema.json"),
        );
        let cands = orch.candidates_for("酔っ払って千鳥足で踊る");
        assert!(
            cands.len() >= 12,
            "expected topped-up candidates, got {}",
            cands.len()
        );
        // 重複していないこと。
        let mut keys: Vec<&str> = cands.iter().map(|e| e.key.as_str()).collect();
        keys.sort();
        let before = keys.len();
        keys.dedup();
        assert_eq!(before, keys.len(), "candidates must be unique");
    }

    #[test]
    fn generate_errors_on_garbage() {
        let cat = catalog();
        let orch = Orchestrator::new(
            MockRunner {
                response: "not json at all".into(),
            },
            &cat,
            PathBuf::from("schema/emote.schema.json"),
        );
        assert!(orch.generate("x").is_err());
    }
}
