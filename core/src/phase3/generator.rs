//! Phase 3: text-to-motion 生成の抽象とローカルランナー実装。
//!
//! 検証境界: 実際の text-to-motion モデル（MoMask/MDM 等, GPU or HF Space）は本環境に無いため、
//! 「外部ランナー（プロンプトを受け取り MotionClip JSON を stdout に出すコマンド）」を呼ぶ
//! 差し替え可能な実装とする。モデルの品質・出力は未検証。

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::phase2::motion::MotionClip;

#[derive(Debug, thiserror::Error)]
pub enum MotionGenError {
    #[error("failed to spawn generator: {0}")]
    Spawn(#[source] std::io::Error),
    #[error("generator exited with status {code}: {stderr}")]
    NonZero { code: i32, stderr: String },
    #[error("generator timed out after {0:?}")]
    Timeout(Duration),
    #[error("failed to parse MotionClip from generator output: {0}")]
    Parse(#[source] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[source] std::io::Error),
}

/// プロンプト → MotionClip を生成する抽象。
pub trait MotionGenerator {
    fn generate(&self, prompt: &str) -> Result<MotionClip, MotionGenError>;
}

/// 外部コマンドを叩く実装。
/// 例: binary="python", args=["t2m_runner.py"] → `python t2m_runner.py` を起動し、
/// プロンプトを stdin で渡し、stdout に MotionClip JSON を期待する。
pub struct CommandMotionGenerator {
    pub binary: String,
    pub args: Vec<String>,
    pub timeout: Duration,
}

impl CommandMotionGenerator {
    pub fn new(binary: impl Into<String>, args: Vec<String>) -> Self {
        CommandMotionGenerator {
            binary: binary.into(),
            args,
            timeout: Duration::from_secs(300),
        }
    }
}

impl MotionGenerator for CommandMotionGenerator {
    fn generate(&self, prompt: &str) -> Result<MotionClip, MotionGenError> {
        let mut child = Command::new(&self.binary)
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(MotionGenError::Spawn)?;
        {
            use std::io::Write;
            let mut stdin = child.stdin.take().expect("piped stdin");
            // stdin を読まない生成器では BrokenPipe になり得るが、それは正常系として許容。
            if let Err(e) = stdin.write_all(prompt.as_bytes()) {
                if e.kind() != std::io::ErrorKind::BrokenPipe {
                    return Err(MotionGenError::Io(e));
                }
            }
            // drop で stdin を閉じ、読み手に EOF を伝える。
        }

        let start = Instant::now();
        let status = loop {
            if let Some(s) = child.try_wait().map_err(MotionGenError::Io)? {
                break s;
            }
            if start.elapsed() > self.timeout {
                let _ = child.kill();
                return Err(MotionGenError::Timeout(self.timeout));
            }
            std::thread::sleep(Duration::from_millis(50));
        };

        let mut out = String::new();
        if let Some(mut o) = child.stdout.take() {
            o.read_to_string(&mut out).map_err(MotionGenError::Io)?;
        }
        if !status.success() {
            let mut e = String::new();
            if let Some(mut se) = child.stderr.take() {
                let _ = se.read_to_string(&mut e);
            }
            return Err(MotionGenError::NonZero {
                code: status.code().unwrap_or(-1),
                stderr: e.trim().to_string(),
            });
        }
        serde_json::from_str(&out).map_err(MotionGenError::Parse)
    }
}

/// プロンプト → 生成 → リターゲット → .ycd.xml を一気通貫で行う。
pub fn generate_to_ycd_xml<G: MotionGenerator>(
    gen: &G,
    prompt: &str,
) -> Result<String, MotionGenError> {
    let clip = gen.generate(prompt)?;
    let rt = crate::phase2::retarget::retarget(&clip);
    Ok(crate::phase2::ycd_xml::build_ycd_xml(&rt))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase2::motion::{Frame, Joint, MotionClip};

    fn fixture_clip() -> MotionClip {
        MotionClip {
            name: "ai_wave".into(),
            joints: vec![Joint {
                name: "mixamorig:Hips".into(),
                parent: None,
                offset: [0.0; 3],
            }],
            frames: vec![Frame {
                rotations: vec![[0.0, 0.0, 0.0, 1.0]],
                root_translation: [0.0; 3],
            }],
            frame_time: 0.033,
        }
    }

    /// テスト用: 固定 MotionClip を返すモック。
    struct MockGen(MotionClip);
    impl MotionGenerator for MockGen {
        fn generate(&self, _prompt: &str) -> Result<MotionClip, MotionGenError> {
            Ok(self.0.clone())
        }
    }

    #[test]
    fn pipeline_generates_ycd_xml_from_mock() {
        let g = MockGen(fixture_clip());
        let xml = generate_to_ycd_xml(&g, "wave hello").unwrap();
        assert!(xml.contains("<ClipDictionary>"));
        assert!(xml.contains("value=\"11816\"")); // Hips → SKEL_Pelvis
    }

    #[test]
    fn command_generator_reads_stdout_json() {
        // `cat` でフィクスチャ JSON を stdout に流すコマンドを擬似ランナーにする。
        let json = serde_json::to_string(&fixture_clip()).unwrap();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), &json).unwrap();
        let g = CommandMotionGenerator::new("cat", vec![tmp.path().to_string_lossy().to_string()]);
        let clip = g.generate("ignored").unwrap();
        assert_eq!(clip.name, "ai_wave");
    }

    #[test]
    fn command_generator_errors_on_bad_json() {
        let g = CommandMotionGenerator::new("printf", vec!["not json".to_string()]);
        assert!(matches!(g.generate("x"), Err(MotionGenError::Parse(_))));
    }
}
