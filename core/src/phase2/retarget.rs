//! 標準ヒューマノイド/Mixamo スケルトン → GTA V ped スケルトンへのリターゲット。
//!
//! 注意（検証境界）: これはボーン名対応に基づく簡易リターゲット。レストポーズ差分の
//! 補正は行わない best-effort 実装で、実 GTA スケルトンでの最終確認は CodeWalker/Blender が必要。

use crate::phase2::motion::{Joint, MotionClip};
use std::collections::HashMap;

/// GTA V ped の主要ボーン名と bone tag（アニメで使う signed-ish 16bit タグ）。
/// 値は GTA V で広く知られた標準タグ。
pub const GTA_BONE_TAGS: &[(&str, u16)] = &[
    ("SKEL_ROOT", 0),
    ("SKEL_Pelvis", 11816),
    ("SKEL_Spine_Root", 57597),
    ("SKEL_Spine0", 23553),
    ("SKEL_Spine1", 24816),
    ("SKEL_Spine2", 24817),
    ("SKEL_Spine3", 24818),
    ("SKEL_Neck_1", 39317),
    ("SKEL_Head", 31086),
    ("SKEL_L_Clavicle", 64729),
    ("SKEL_L_UpperArm", 45509),
    ("SKEL_L_Forearm", 61007),
    ("SKEL_L_Hand", 18905),
    ("SKEL_R_Clavicle", 10706),
    ("SKEL_R_UpperArm", 40269),
    ("SKEL_R_Forearm", 28252),
    ("SKEL_R_Hand", 57005),
    ("SKEL_L_Thigh", 58271),
    ("SKEL_L_Calf", 63931),
    ("SKEL_L_Foot", 14201),
    ("SKEL_R_Thigh", 51826),
    ("SKEL_R_Calf", 36864),
    ("SKEL_R_Foot", 52301),
];

/// よくある humanoid/Mixamo ボーン名（正規化済み）→ GTA ボーン名。
fn humanoid_to_gta() -> HashMap<&'static str, &'static str> {
    [
        ("hips", "SKEL_Pelvis"),
        ("pelvis", "SKEL_Pelvis"),
        ("spine", "SKEL_Spine0"),
        ("spine1", "SKEL_Spine1"),
        ("spine2", "SKEL_Spine2"),
        ("spine3", "SKEL_Spine3"),
        ("chest", "SKEL_Spine3"),
        ("neck", "SKEL_Neck_1"),
        ("head", "SKEL_Head"),
        ("leftshoulder", "SKEL_L_Clavicle"),
        ("leftarm", "SKEL_L_UpperArm"),
        ("leftforearm", "SKEL_L_Forearm"),
        ("lefthand", "SKEL_L_Hand"),
        ("rightshoulder", "SKEL_R_Clavicle"),
        ("rightarm", "SKEL_R_UpperArm"),
        ("rightforearm", "SKEL_R_Forearm"),
        ("righthand", "SKEL_R_Hand"),
        ("leftupleg", "SKEL_L_Thigh"),
        ("leftleg", "SKEL_L_Calf"),
        ("leftfoot", "SKEL_L_Foot"),
        ("rightupleg", "SKEL_R_Thigh"),
        ("rightleg", "SKEL_R_Calf"),
        ("rightfoot", "SKEL_R_Foot"),
    ]
    .into_iter()
    .collect()
}

/// ボーン名 → GTA tag。
pub fn bone_tag(gta_name: &str) -> Option<u16> {
    GTA_BONE_TAGS
        .iter()
        .find(|(n, _)| *n == gta_name)
        .map(|(_, t)| *t)
}

/// 名前を対応表キーへ正規化（"mixamorig:LeftArm" → "leftarm"）。
fn normalize(name: &str) -> String {
    let base = name.rsplit(':').next().unwrap_or(name);
    base.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

/// リターゲット結果。
#[derive(Debug, Clone, PartialEq)]
pub struct RetargetResult {
    /// GTA ボーン名へ付け替えた MotionClip（マッピングできた関節のみ）。
    pub clip: MotionClip,
    /// GTA ボーン名 → tag。ycd 書き出し用。
    pub bone_tags: Vec<(String, u16)>,
    /// 対応付けできなかった元ボーン名。
    pub unmapped: Vec<String>,
}

/// humanoid/Mixamo もしくは既に GTA 名のクリップを GTA スケルトンへリターゲットする。
pub fn retarget(src: &MotionClip) -> RetargetResult {
    let map = humanoid_to_gta();
    // 元 index → (gta_name, tag) の対応。
    let mut resolved: Vec<Option<(String, u16)>> = Vec::with_capacity(src.joints.len());
    let mut unmapped = Vec::new();
    for j in &src.joints {
        // 既に GTA 名ならそのまま、でなければ humanoid 表を引く。
        let gta = if bone_tag(&j.name).is_some() {
            Some(j.name.clone())
        } else {
            map.get(normalize(&j.name).as_str()).map(|s| s.to_string())
        };
        match gta.and_then(|n| bone_tag(&n).map(|t| (n, t))) {
            Some(pair) => resolved.push(Some(pair)),
            None => {
                unmapped.push(j.name.clone());
                resolved.push(None);
            }
        }
    }

    // 採用する関節の元 index 一覧。
    let kept: Vec<usize> = resolved
        .iter()
        .enumerate()
        .filter_map(|(i, r)| r.as_ref().map(|_| i))
        .collect();
    let old_to_new: HashMap<usize, usize> = kept
        .iter()
        .enumerate()
        .map(|(new, &old)| (old, new))
        .collect();

    // 親が脱落している場合は、最近接の生存祖先まで遡って再ペアレントする。
    let nearest_kept_ancestor = |mut p: Option<usize>| -> Option<usize> {
        while let Some(old_p) = p {
            if let Some(&new_p) = old_to_new.get(&old_p) {
                return Some(new_p);
            }
            p = src.joints[old_p].parent;
        }
        None
    };

    let joints = kept
        .iter()
        .map(|&old| {
            let (name, _) = resolved[old].clone().unwrap();
            let parent = nearest_kept_ancestor(src.joints[old].parent);
            Joint {
                name,
                parent,
                offset: src.joints[old].offset,
            }
        })
        .collect();

    let frames = src
        .frames
        .iter()
        .map(|f| crate::phase2::motion::Frame {
            rotations: kept.iter().map(|&old| f.rotations[old]).collect(),
            root_translation: f.root_translation,
        })
        .collect();

    let bone_tags = kept
        .iter()
        .map(|&old| {
            let (name, tag) = resolved[old].clone().unwrap();
            (name, tag)
        })
        .collect();

    RetargetResult {
        clip: MotionClip {
            name: src.name.clone(),
            joints,
            frames,
            frame_time: src.frame_time,
        },
        bone_tags,
        unmapped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase2::motion::{Frame, Joint, MotionClip};

    fn clip(names: &[&str]) -> MotionClip {
        let joints = names
            .iter()
            .enumerate()
            .map(|(i, n)| Joint {
                name: n.to_string(),
                parent: if i == 0 { None } else { Some(i - 1) },
                offset: [0.0; 3],
            })
            .collect::<Vec<_>>();
        let rotations = vec![[0.0, 0.0, 0.0, 1.0]; names.len()];
        MotionClip {
            name: "c".into(),
            joints,
            frames: vec![Frame {
                rotations,
                root_translation: [0.0; 3],
            }],
            frame_time: 0.033,
        }
    }

    #[test]
    fn maps_mixamo_names_to_gta() {
        let c = clip(&["mixamorig:Hips", "mixamorig:Spine", "mixamorig:LeftArm"]);
        let r = retarget(&c);
        assert_eq!(r.clip.joints[0].name, "SKEL_Pelvis");
        assert_eq!(r.clip.joints[2].name, "SKEL_L_UpperArm");
        assert!(r.unmapped.is_empty());
        assert_eq!(bone_tag("SKEL_L_Hand"), Some(18905));
    }

    #[test]
    fn keeps_existing_gta_names() {
        let c = clip(&["SKEL_Pelvis", "SKEL_Spine0"]);
        let r = retarget(&c);
        assert_eq!(r.clip.joint_count(), 2);
        assert_eq!(r.bone_tags[0], ("SKEL_Pelvis".to_string(), 11816));
    }

    #[test]
    fn reports_unmapped_and_drops_them() {
        let c = clip(&["mixamorig:Hips", "WeirdBone"]);
        let r = retarget(&c);
        assert_eq!(r.clip.joint_count(), 1);
        assert_eq!(r.unmapped, vec!["WeirdBone".to_string()]);
    }

    #[test]
    fn reparents_after_dropping() {
        // Hips(0) -> Weird(1) -> Spine(2): Weird dropped, Spine reparents to Hips(new 0)
        let c = clip(&["mixamorig:Hips", "Weird", "mixamorig:Spine"]);
        let r = retarget(&c);
        assert_eq!(r.clip.joint_count(), 2);
        assert_eq!(r.clip.joints[1].name, "SKEL_Spine0");
        assert_eq!(r.clip.joints[1].parent, Some(0));
    }
}
