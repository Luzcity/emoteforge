//! MotionClip → CodeWalker 互換 .ycd.xml ビルダー（best-effort）。
//!
//! 検証境界（重要）: 出力 XML の要素/属性名は CodeWalker の Clip.cs/YcdFile.cs の WriteXML を
//! 元に再現した best-effort 実装。**実バイナリ .ycd への変換と FiveM 実ロードは未検証**で、
//! CodeWalker（XML→バイナリ）/ Blender(Sollumz) が別途必要。回転は per-frame の RawFloat
//! チャンネル(quaternion x/y/z/w を 4 本)で書き出す簡易方式。
//!
//! 参照: https://github.com/dexyfex/CodeWalker (CodeWalker.Core/GameFiles/Resources/Clip.cs)

use crate::phase2::motion::MotionClip;
use crate::phase2::retarget::RetargetResult;

/// RetargetResult から .ycd.xml 文字列を生成する。
pub fn build_ycd_xml(rt: &RetargetResult) -> String {
    let clip = &rt.clip;
    let frame_count = clip.frame_count().max(1);
    let duration = clip.duration();
    let anim_hash = format!("hash_{}", sanitize_hash(&clip.name));
    let clip_hash = format!("anim_{}", sanitize_hash(&clip.name));

    let mut s = String::new();
    s.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    s.push_str("<ClipDictionary>\n");

    // ----- Clips（再生エントリ。Animation を参照し名前/長さを与える） -----
    s.push_str("  <Clips>\n");
    s.push_str("    <Item type=\"Animation\">\n");
    s.push_str(&format!("      <Hash>{clip_hash}</Hash>\n"));
    s.push_str(&format!("      <Name>pack/{}.clip</Name>\n", clip.name));
    s.push_str(&format!(
        "      <AnimationHash>{anim_hash}</AnimationHash>\n"
    ));
    s.push_str("      <StartTime value=\"0\" />\n");
    s.push_str(&format!("      <EndTime value=\"{duration:.6}\" />\n"));
    s.push_str("      <Rate value=\"1\" />\n");
    s.push_str("    </Item>\n");
    s.push_str("  </Clips>\n");

    // ----- Animations -----
    s.push_str("  <Animations>\n");
    s.push_str("    <Item>\n");
    s.push_str(&format!("      <Hash>{anim_hash}</Hash>\n"));
    s.push_str("      <Unknown10 value=\"0\" />\n");
    s.push_str(&format!("      <FrameCount value=\"{frame_count}\" />\n"));
    s.push_str(&format!(
        "      <SequenceFrameLimit value=\"{frame_count}\" />\n"
    ));
    s.push_str(&format!("      <Duration value=\"{duration:.6}\" />\n"));
    s.push_str("      <Unknown1C>Default</Unknown1C>\n");

    // BoneIds: 各 GTA ボーン × トラック(0=回転)。
    //
    // 注意: root translation（移動トラック）はあえて XML に出していない。CodeWalker は
    // トラックごとのチャンネル数/順序/型を厳密に期待するため、検証なしに移動チャンネルを
    // 足すとファイル全体が読めなくなるリスクがある（Codex レビュー指摘）。移動データは
    // MotionClip 側に保持しており、実 CodeWalker サンプルでチャンネル仕様を確定後に追加する。
    s.push_str("      <BoneIds>\n");
    for (_, tag) in &rt.bone_tags {
        s.push_str("        <Item>\n");
        s.push_str(&format!("          <BoneId value=\"{tag}\" />\n"));
        s.push_str("          <Track value=\"0\" />\n");
        s.push_str("          <Unk0 value=\"0\" />\n");
        s.push_str("        </Item>\n");
    }
    s.push_str("      </BoneIds>\n");

    // Sequences: 1 シーケンスに各ボーンの回転チャンネルを格納。
    s.push_str("      <Sequences>\n");
    s.push_str("        <Item>\n");
    s.push_str("          <Channels>\n");
    for (bi, _) in rt.bone_tags.iter().enumerate() {
        // quaternion を x/y/z/w の 4 本の RawFloat チャンネルで表現。
        for comp in 0..4 {
            s.push_str("            <Item type=\"RawFloat\">\n");
            s.push_str("              <Values>\n");
            for f in &clip.frames {
                let q = f.rotations[bi];
                s.push_str(&format!("                {:.6}\n", q[comp]));
            }
            s.push_str("              </Values>\n");
            s.push_str("            </Item>\n");
        }
    }
    s.push_str("          </Channels>\n");
    s.push_str("        </Item>\n");
    s.push_str("      </Sequences>\n");

    s.push_str("    </Item>\n");
    s.push_str("  </Animations>\n");
    s.push_str("</ClipDictionary>\n");
    s
}

/// 名前を Hash 文字列向けに正規化。
fn sanitize_hash(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

/// 便宜関数: MotionClip を直接受けて（リターゲット込みで）XML を返す。
pub fn motion_to_ycd_xml(clip: &MotionClip) -> String {
    let rt = crate::phase2::retarget::retarget(clip);
    build_ycd_xml(&rt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase2::motion::{Frame, Joint, MotionClip};
    use crate::phase2::retarget::retarget;

    fn clip() -> MotionClip {
        MotionClip {
            name: "wave_test".into(),
            joints: vec![
                Joint {
                    name: "SKEL_Pelvis".into(),
                    parent: None,
                    offset: [0.0; 3],
                },
                Joint {
                    name: "SKEL_R_UpperArm".into(),
                    parent: Some(0),
                    offset: [0.0; 3],
                },
            ],
            frames: vec![
                Frame {
                    rotations: vec![[0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 0.0, 1.0]],
                    root_translation: [0.0; 3],
                },
                Frame {
                    rotations: vec![[0.0, 0.0, 0.0, 1.0], [0.0, 0.1, 0.0, 0.99]],
                    root_translation: [0.0; 3],
                },
            ],
            frame_time: 0.033,
        }
    }

    #[test]
    fn produces_well_formed_root_and_sections() {
        let xml = build_ycd_xml(&retarget(&clip()));
        assert!(xml.contains("<ClipDictionary>") && xml.contains("</ClipDictionary>"));
        assert!(xml.contains("<Clips>") && xml.contains("<Animations>"));
        assert!(xml.contains("<BoneIds>") && xml.contains("<Sequences>"));
    }

    #[test]
    fn includes_bone_tags() {
        let xml = build_ycd_xml(&retarget(&clip()));
        // SKEL_Pelvis=11816, SKEL_R_UpperArm=40269
        assert!(xml.contains("value=\"11816\""));
        assert!(xml.contains("value=\"40269\""));
    }

    #[test]
    fn frame_count_matches() {
        let xml = build_ycd_xml(&retarget(&clip()));
        assert!(xml.contains("<FrameCount value=\"2\" />"));
    }

    #[test]
    fn well_formed_tag_balance() {
        let xml = build_ycd_xml(&retarget(&clip()));
        // 開きタグ <X ...> と閉じタグ </X> のざっくり整合（自己終了 /> は除外）。
        let opens = xml.matches('<').count();
        let closes = xml.matches('>').count();
        assert_eq!(opens, closes, "every angle bracket should pair");
    }

    #[test]
    fn channels_have_four_rotation_components_per_bone() {
        let xml = build_ycd_xml(&retarget(&clip()));
        // 2 bones * 4 quaternion components = 8 RawFloat channels（移動は仕様確定まで非出力）
        assert_eq!(xml.matches("type=\"RawFloat\"").count(), 8);
        // 移動トラック(Track 1)は意図的に出さない。
        assert!(!xml.contains("<Track value=\"1\" />"));
    }
}
