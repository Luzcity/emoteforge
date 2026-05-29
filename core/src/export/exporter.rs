//! EmoteSpec 群を、依存なしで動くスタンドアロン FiveM リソースへ書き出す。

use std::path::{Path, PathBuf};

use crate::export::lua_templates::{client_lua, fxmanifest};
use crate::model::EmoteSpec;

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("no emotes to export")]
    Empty,
    #[error("invalid resource name: {0}")]
    InvalidName(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialize error: {0}")]
    Serialize(#[from] serde_json::Error),
}

/// エクスポート結果の要約。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ResourceManifest {
    pub resource_name: String,
    pub dir: PathBuf,
    pub files: Vec<String>,
    pub emote_count: usize,
}

/// `specs` を `out_root/<resource_name>/` 配下のリソースとして書き出す。
pub fn export(
    specs: &[EmoteSpec],
    out_root: &Path,
    resource_name: &str,
) -> Result<ResourceManifest, ExportError> {
    if specs.is_empty() {
        return Err(ExportError::Empty);
    }
    if !is_valid_resource_name(resource_name) {
        return Err(ExportError::InvalidName(resource_name.to_string()));
    }

    let dir = out_root.join(resource_name);
    std::fs::create_dir_all(&dir)?;

    let emotes_json = serde_json::to_string_pretty(specs)?;
    std::fs::write(dir.join("emotes.json"), emotes_json)?;
    std::fs::write(dir.join("client.lua"), client_lua())?;
    std::fs::write(
        dir.join("fxmanifest.lua"),
        fxmanifest(resource_name, specs.len()),
    )?;

    Ok(ResourceManifest {
        resource_name: resource_name.to_string(),
        dir,
        files: vec![
            "fxmanifest.lua".into(),
            "client.lua".into(),
            "emotes.json".into(),
        ],
        emote_count: specs.len(),
    })
}

/// FiveM リソース名として安全か（パストラバーサル防止も兼ねる）。
pub fn is_valid_resource_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ClipRef, ClipSource, Meta, MovementType};

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
                flags: vec!["AF_LOOPING".into()],
            }],
            looping: true,
            upper_body_only: false,
            prop: None,
            facial: None,
            movement_type: MovementType::Stationary,
            meta: Meta { source: "codex".into(), schema_version: 1 },
        }
    }

    #[test]
    fn writes_all_three_files() {
        let tmp = tempfile::tempdir().unwrap();
        let m = export(&[sample()], tmp.path(), "my_emotes").unwrap();
        assert!(m.dir.join("fxmanifest.lua").exists());
        assert!(m.dir.join("client.lua").exists());
        assert!(m.dir.join("emotes.json").exists());
        assert_eq!(m.emote_count, 1);
    }

    #[test]
    fn emotes_json_round_trips() {
        let tmp = tempfile::tempdir().unwrap();
        let specs = vec![sample()];
        let m = export(&specs, tmp.path(), "my_emotes").unwrap();
        let text = std::fs::read_to_string(m.dir.join("emotes.json")).unwrap();
        let back: Vec<EmoteSpec> = serde_json::from_str(&text).unwrap();
        assert_eq!(back, specs);
    }

    #[test]
    fn manifest_and_client_reference_expected_tokens() {
        let tmp = tempfile::tempdir().unwrap();
        let m = export(&[sample()], tmp.path(), "my_emotes").unwrap();
        let manifest = std::fs::read_to_string(m.dir.join("fxmanifest.lua")).unwrap();
        assert!(manifest.contains("client_script 'client.lua'"));
        assert!(manifest.contains("emotes.json"));
        let client = std::fs::read_to_string(m.dir.join("client.lua")).unwrap();
        assert!(client.contains("RegisterCommand('emote'"));
        assert!(client.contains("RegisterCommand('emotestop'"));
        assert!(client.contains("TaskPlayAnim"));
    }

    #[test]
    fn client_lua_has_balanced_parens_and_braces() {
        let lua = client_lua();
        assert_eq!(lua.matches('(').count(), lua.matches(')').count(), "parens balance");
        assert_eq!(lua.matches('{').count(), lua.matches('}').count(), "braces balance");
    }

    #[test]
    fn rejects_empty_and_bad_names() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(matches!(export(&[], tmp.path(), "x"), Err(ExportError::Empty)));
        assert!(matches!(
            export(&[sample()], tmp.path(), "../evil"),
            Err(ExportError::InvalidName(_))
        ));
        assert!(!is_valid_resource_name("../evil"));
        assert!(is_valid_resource_name("my_emotes-01"));
    }
}
