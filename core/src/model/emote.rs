use serde::{Deserialize, Serialize};

/// 1 つのエモート定義。Codex 出力・UI 編集・エクスポートで共有する中心型。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmoteSpec {
    /// resource 内で一意な識別子（コマンド名にも使う）。小文字英数字とアンダースコア。
    pub name: String,
    /// UI 表示用の名前（日本語可）。
    pub display_name: String,
    /// 再生するクリップ列。複数なら Lua チェーンで順次再生する。
    pub clips: Vec<ClipRef>,
    /// ループ再生するか。
    #[serde(rename = "loop")]
    pub looping: bool,
    /// 上半身のみに適用するか（歩きながらのエモート等）。
    pub upper_body_only: bool,
    /// 手に持つ prop（任意。未使用時は null）。
    #[serde(default)]
    pub prop: Option<Prop>,
    /// 表情オーバーレイ（任意。未使用時は null）。
    #[serde(default)]
    pub facial: Option<Facial>,
    /// 移動可否の分類。
    pub movement_type: MovementType,
    /// メタ情報。
    pub meta: Meta,
}

/// 再生する 1 クリップの参照とパラメータ。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipRef {
    /// クリップの供給元。Phase1 は Builtin（既存 GTA アニメ）のみ。
    #[serde(default)]
    pub source: ClipSource,
    /// アニメーション辞書名（animDict）。
    pub dict: String,
    /// クリップ名。
    pub clip: String,
    /// ブレンドイン秒。
    #[serde(default = "default_blend")]
    pub blend_in: f32,
    /// ブレンドアウト秒。
    #[serde(default = "default_blend")]
    pub blend_out: f32,
    /// 再生時間ミリ秒。-1 でクリップ長（ループ時は無視）。
    #[serde(default = "default_duration")]
    pub duration: i32,
    /// 再生速度（1.0 = 等速）。
    #[serde(default = "default_rate")]
    pub playback_rate: f32,
    /// アニメフラグ（AF_LOOPING 等）。
    #[serde(default)]
    pub flags: Vec<String>,
}

fn default_blend() -> f32 {
    1.0
}
fn default_duration() -> i32 {
    -1
}
fn default_rate() -> f32 {
    1.0
}

/// クリップの供給元。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ClipSource {
    /// 既存 GTA V アニメ辞書を参照（Phase1）。
    #[default]
    Builtin,
    /// 自前生成のクリップ辞書（Phase2 以降、stream 同梱）。
    Ycd,
}

/// 手に持つ prop。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prop {
    /// prop のモデル名。
    pub model: String,
    /// アタッチするボーン ID（例: 右手 = 18905）。
    pub bone: i32,
    /// 位置オフセット [x, y, z]。
    pub offset: [f32; 3],
    /// 回転 [pitch, roll, yaw]。
    pub rot: [f32; 3],
}

/// 表情オーバーレイ。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Facial {
    pub dict: String,
    pub clip: String,
}

/// 移動可否の分類。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum MovementType {
    /// その場で静止して再生。
    #[default]
    Stationary,
    /// 歩き回れる（上半身など）。
    Walkable,
    /// アクションループ。
    ActionLoop,
}

/// 生成元などのメタ情報。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    /// 生成元（codex / manual / ai など）。
    pub source: String,
    /// スキーマバージョン。
    pub schema_version: u32,
}

impl Default for Meta {
    fn default() -> Self {
        Meta {
            source: "manual".to_string(),
            schema_version: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> EmoteSpec {
        EmoteSpec {
            name: "drunk_cheers".into(),
            display_name: "酔っ払い乾杯".into(),
            clips: vec![ClipRef {
                source: ClipSource::Builtin,
                dict: "amb@world_human_drinking@coffee@male@idle_a".into(),
                clip: "idle_c".into(),
                blend_in: 1.0,
                blend_out: 1.0,
                duration: -1,
                playback_rate: 0.9,
                flags: vec!["AF_LOOPING".into(), "AF_UPPERBODY".into()],
            }],
            looping: true,
            upper_body_only: false,
            prop: Some(Prop {
                model: "prop_beer_bottle".into(),
                bone: 18905,
                offset: [0.0, 0.0, 0.0],
                rot: [0.0, 0.0, 0.0],
            }),
            facial: Some(Facial {
                dict: "facials@gen_male@base".into(),
                clip: "mood_drunk_1".into(),
            }),
            movement_type: MovementType::Stationary,
            meta: Meta {
                source: "codex".into(),
                schema_version: 1,
            },
        }
    }

    #[test]
    fn round_trips_through_json() {
        let spec = sample();
        let json = serde_json::to_string(&spec).unwrap();
        let back: EmoteSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(spec, back);
    }

    #[test]
    fn loop_field_serializes_as_loop() {
        let json = serde_json::to_value(sample()).unwrap();
        assert!(json.get("loop").is_some(), "JSON should use key `loop`");
        assert_eq!(
            json["clips"][0]["dict"],
            "amb@world_human_drinking@coffee@male@idle_a"
        );
    }

    #[test]
    fn clip_defaults_apply_when_missing() {
        let json = r#"{"dict":"d","clip":"c"}"#;
        let clip: ClipRef = serde_json::from_str(json).unwrap();
        assert_eq!(clip.blend_in, 1.0);
        assert_eq!(clip.duration, -1);
        assert_eq!(clip.playback_rate, 1.0);
        assert_eq!(clip.source, ClipSource::Builtin);
        assert!(clip.flags.is_empty());
    }

    #[test]
    fn none_prop_and_facial_serialize_as_null() {
        let mut spec = sample();
        spec.prop = None;
        spec.facial = None;
        let json = serde_json::to_value(&spec).unwrap();
        // strict schema は全キー必須のため、未使用でも null として存在させる。
        assert!(json["prop"].is_null());
        assert!(json["facial"].is_null());
        // null から None へ復元できる。
        let back: EmoteSpec = serde_json::from_value(json).unwrap();
        assert_eq!(back.prop, None);
        assert_eq!(back.facial, None);
    }
}
