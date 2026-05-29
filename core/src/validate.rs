//! EmoteSpec のバリデーションと軽微な正規化。
//! Codex はクリップ名をハルシネートし得るため、カタログ/ dump 索引と突き合わせる。

use crate::catalog::Catalog;
use crate::model::EmoteSpec;

/// バリデーション結果。問題があれば issues に詳細を載せる。
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationReport {
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn is_ok(&self) -> bool {
        self.issues.is_empty()
    }
}

/// 1 件の問題。clip 不実在時は suggestions に補修候補を載せる。
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationIssue {
    /// 問題の場所（例: "clips[0].clip", "name"）。
    pub field: String,
    pub message: String,
    pub suggestions: Vec<String>,
}

/// 正規化を行いつつ検証する。
/// 問題が無ければ正規化済み spec を Ok、あれば Err(report) を返す。
pub fn validate(spec: &EmoteSpec, catalog: &Catalog) -> Result<EmoteSpec, ValidationReport> {
    let mut issues = Vec::new();
    let mut normalized = spec.clone();

    // name 正規化（小文字・英数字とアンダースコアのみ）。
    let sanitized = sanitize_name(&normalized.name);
    if sanitized.is_empty() {
        issues.push(ValidationIssue {
            field: "name".into(),
            message: "name must contain at least one ascii alphanumeric character".into(),
            suggestions: vec![],
        });
    }
    normalized.name = sanitized;

    if normalized.clips.is_empty() {
        issues.push(ValidationIssue {
            field: "clips".into(),
            message: "emote must have at least one clip".into(),
            suggestions: vec![],
        });
    }

    for (i, clip) in normalized.clips.iter_mut().enumerate() {
        // 数値の健全化。
        clip.blend_in = clip.blend_in.clamp(0.0, 10.0);
        clip.blend_out = clip.blend_out.clamp(0.0, 10.0);
        clip.playback_rate = clip.playback_rate.clamp(0.1, 5.0);

        if clip.dict.trim().is_empty() || clip.clip.trim().is_empty() {
            issues.push(ValidationIssue {
                field: format!("clips[{i}]"),
                message: "dict and clip must not be empty".into(),
                suggestions: vec![],
            });
            continue;
        }

        if !catalog.contains(&clip.dict, &clip.clip) {
            // 同 dict に他クリップがあれば候補に、無ければ近い dict を検索。
            let same_dict = catalog.clips_of(&clip.dict);
            let suggestions = if !same_dict.is_empty() {
                same_dict.into_iter().take(8).collect()
            } else {
                catalog
                    .search(&clip.dict.replace(['@', '_'], " "), 5)
                    .iter()
                    .map(|e| format!("{} / {}", e.dict, e.clip))
                    .collect()
            };
            issues.push(ValidationIssue {
                field: format!("clips[{i}].clip"),
                message: format!("unknown animation: {} / {}", clip.dict, clip.clip),
                suggestions,
            });
        }
    }

    if issues.is_empty() {
        Ok(normalized)
    } else {
        Err(ValidationReport { issues })
    }
}

/// 識別子を resource/コマンド安全な形へ。
pub fn sanitize_name(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == '_' || ch == '-' || ch == ' ' {
            out.push('_');
        }
        // それ以外（日本語等）は捨てる。
    }
    // 連続アンダースコアを 1 つに、前後を trim。
    out.split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ClipRef, ClipSource, Meta, MovementType};
    use std::path::Path;

    fn catalog() -> Catalog {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../catalog/catalog.json");
        Catalog::load(&p).unwrap()
    }

    fn spec_with(dict: &str, clip: &str) -> EmoteSpec {
        EmoteSpec {
            name: "Test Emote!".into(),
            display_name: "テスト".into(),
            clips: vec![ClipRef {
                source: ClipSource::Builtin,
                dict: dict.into(),
                clip: clip.into(),
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

    #[test]
    fn valid_spec_passes_and_normalizes_name() {
        let cat = catalog();
        let first = cat.entries()[0].clone();
        let spec = spec_with(&first.dict, &first.clip);
        let out = validate(&spec, &cat).expect("should be valid");
        assert_eq!(out.name, "test_emote"); // "Test Emote!" -> "test_emote"
    }

    #[test]
    fn unknown_clip_reports_with_suggestions() {
        let cat = catalog();
        let dict = cat.entries()[0].dict.clone();
        let spec = spec_with(&dict, "definitely_not_a_real_clip");
        let report = validate(&spec, &cat).unwrap_err();
        assert!(report.issues.iter().any(|i| i.field.contains("clip")));
        let issue = &report.issues[0];
        assert!(!issue.suggestions.is_empty(), "should suggest real clips for the dict");
    }

    #[test]
    fn empty_clips_is_error() {
        let cat = catalog();
        let mut spec = spec_with("d", "c");
        spec.clips.clear();
        let report = validate(&spec, &cat).unwrap_err();
        assert!(report.issues.iter().any(|i| i.field == "clips"));
    }

    #[test]
    fn clamps_out_of_range_numbers() {
        let cat = catalog();
        let first = cat.entries()[0].clone();
        let mut spec = spec_with(&first.dict, &first.clip);
        spec.clips[0].playback_rate = 99.0;
        spec.clips[0].blend_in = -5.0;
        let out = validate(&spec, &cat).unwrap();
        assert_eq!(out.clips[0].playback_rate, 5.0);
        assert_eq!(out.clips[0].blend_in, 0.0);
    }

    #[test]
    fn sanitize_name_handles_japanese_and_symbols() {
        assert_eq!(sanitize_name("酔っ払い Cheers!!"), "cheers");
        assert_eq!(sanitize_name("Wave Hello"), "wave_hello");
        assert_eq!(sanitize_name("a__b--c"), "a_b_c");
    }
}
