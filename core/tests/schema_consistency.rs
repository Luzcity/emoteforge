//! モデル(serde) と schema/emote.schema.json の整合性テスト。
//! Rust struct をシリアライズした JSON が JSON Schema に valid であることを確認する。

use emoteforge_core::model::emote::*;

fn schema_path() -> std::path::PathBuf {
    // core/ から見たリポジトリルートの schema を参照。
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("schema")
        .join("emote.schema.json")
}

fn sample() -> EmoteSpec {
    EmoteSpec {
        name: "wave_hello".into(),
        display_name: "手を振る".into(),
        clips: vec![ClipRef {
            source: ClipSource::Builtin,
            dict: "friends@frlemar@ig_1".into(),
            clip: "wave".into(),
            blend_in: 1.0,
            blend_out: 1.0,
            duration: -1,
            playback_rate: 1.0,
            flags: vec!["AF_UPPERBODY".into()],
        }],
        looping: false,
        upper_body_only: true,
        prop: None,
        facial: None,
        movement_type: MovementType::Walkable,
        meta: Meta {
            source: "codex".into(),
            schema_version: 1,
        },
    }
}

#[test]
fn serialized_spec_is_valid_against_schema() {
    let schema_text = std::fs::read_to_string(schema_path()).expect("schema file");
    let schema_json: serde_json::Value = serde_json::from_str(&schema_text).expect("schema json");
    let compiled = jsonschema::JSONSchema::compile(&schema_json).expect("schema compiles");

    let instance = serde_json::to_value(sample()).unwrap();
    let result = compiled.validate(&instance);
    if let Err(errors) = result {
        let msgs: Vec<String> = errors.map(|e| format!("{} @ {}", e, e.instance_path)).collect();
        panic!("instance invalid against schema:\n{}", msgs.join("\n"));
    }
}

#[test]
fn spec_with_prop_and_facial_is_valid() {
    let schema_text = std::fs::read_to_string(schema_path()).unwrap();
    let schema_json: serde_json::Value = serde_json::from_str(&schema_text).unwrap();
    let compiled = jsonschema::JSONSchema::compile(&schema_json).unwrap();

    let mut spec = sample();
    spec.prop = Some(Prop {
        model: "p_cs_bottle_01".into(),
        bone: 18905,
        offset: [0.0, 0.0, 0.0],
        rot: [0.0, 0.0, 0.0],
    });
    spec.facial = Some(Facial {
        dict: "facials@gen_male@base".into(),
        clip: "mood_happy_1".into(),
    });
    let instance = serde_json::to_value(&spec).unwrap();
    assert!(compiled.is_valid(&instance));
}
