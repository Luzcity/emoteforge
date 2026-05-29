//! BVH モーションファイルのパーサ → MotionClip。
//! HIERARCHY(関節・OFFSET・CHANNELS) と MOTION(Frames/Frame Time/各行) を読む。

use crate::phase2::motion::{euler_to_quat, Frame, Joint, MotionClip};

#[derive(Debug, thiserror::Error)]
pub enum BvhError {
    #[error("parse error: {0}")]
    Parse(String),
}

struct JointDef {
    name: String,
    parent: Option<usize>,
    offset: [f32; 3],
    channels: Vec<Channel>,
}

#[derive(Clone, Copy, PartialEq)]
enum Channel {
    Xpos,
    Ypos,
    Zpos,
    Xrot,
    Yrot,
    Zrot,
}

fn parse_channel(s: &str) -> Option<Channel> {
    Some(match s {
        "Xposition" => Channel::Xpos,
        "Yposition" => Channel::Ypos,
        "Zposition" => Channel::Zpos,
        "Xrotation" => Channel::Xrot,
        "Yrotation" => Channel::Yrot,
        "Zrotation" => Channel::Zrot,
        _ => return None,
    })
}

/// BVH テキストを MotionClip へ。
pub fn parse_bvh(text: &str, name: &str) -> Result<MotionClip, BvhError> {
    let mut tokens = text.split_whitespace().peekable();
    expect(&mut tokens, "HIERARCHY")?;

    let mut joints: Vec<JointDef> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();

    loop {
        let tok = tokens
            .next()
            .ok_or_else(|| BvhError::Parse("unexpected EOF in hierarchy".into()))?;
        match tok {
            "ROOT" | "JOINT" => {
                let jname = tokens.next().ok_or_else(|| err("joint name"))?.to_string();
                let parent = stack.last().copied();
                let idx = joints.len();
                joints.push(JointDef {
                    name: jname,
                    parent,
                    offset: [0.0; 3],
                    channels: vec![],
                });
                expect(&mut tokens, "{")?;
                stack.push(idx);
            }
            "End" => {
                // "End Site" { OFFSET ... } — フレームデータを持たないので読み飛ばす。
                tokens.next(); // "Site"
                expect(&mut tokens, "{")?;
                expect(&mut tokens, "OFFSET")?;
                for _ in 0..3 {
                    tokens.next();
                }
                expect(&mut tokens, "}")?;
            }
            "OFFSET" => {
                let idx = *stack.last().ok_or_else(|| err("OFFSET outside joint"))?;
                joints[idx].offset = [
                    read_f32(&mut tokens)?,
                    read_f32(&mut tokens)?,
                    read_f32(&mut tokens)?,
                ];
            }
            "CHANNELS" => {
                let idx = *stack.last().ok_or_else(|| err("CHANNELS outside joint"))?;
                let n: usize = tokens
                    .next()
                    .ok_or_else(|| err("channel count"))?
                    .parse()
                    .map_err(|_| err("channel count int"))?;
                for _ in 0..n {
                    let c = tokens.next().ok_or_else(|| err("channel"))?;
                    let ch = parse_channel(c)
                        .ok_or_else(|| BvhError::Parse(format!("unknown channel {c}")))?;
                    joints[idx].channels.push(ch);
                }
            }
            "}" => {
                stack.pop();
                if stack.is_empty() {
                    break;
                }
            }
            "MOTION" => {
                return Err(BvhError::Parse("MOTION before hierarchy closed".into()));
            }
            other => return Err(BvhError::Parse(format!("unexpected token {other}"))),
        }
    }

    expect(&mut tokens, "MOTION")?;
    expect(&mut tokens, "Frames:")?;
    let frame_count: usize = tokens
        .next()
        .ok_or_else(|| err("frames"))?
        .parse()
        .map_err(|_| err("frames int"))?;
    expect(&mut tokens, "Frame")?;
    expect(&mut tokens, "Time:")?;
    let frame_time = read_f32(&mut tokens)?;

    let total_channels: usize = joints.iter().map(|j| j.channels.len()).sum();

    let mut frames = Vec::with_capacity(frame_count);
    for _ in 0..frame_count {
        let mut values = Vec::with_capacity(total_channels);
        for _ in 0..total_channels {
            values.push(read_f32(&mut tokens)?);
        }
        frames.push(decode_frame(&joints, &values));
    }

    let out_joints = joints
        .iter()
        .map(|j| Joint {
            name: j.name.clone(),
            parent: j.parent,
            offset: j.offset,
        })
        .collect();

    Ok(MotionClip {
        name: name.to_string(),
        joints: out_joints,
        frames,
        frame_time,
    })
}

/// 1 行のチャンネル値を各関節のローカル回転 + ルート移動へ復号。
fn decode_frame(joints: &[JointDef], values: &[f32]) -> Frame {
    let mut rotations = Vec::with_capacity(joints.len());
    let mut root_translation = [0.0f32; 3];
    let mut cursor = 0;
    for (ji, j) in joints.iter().enumerate() {
        let mut euler = [0.0f32; 3]; // X,Y,Z 度
        let mut order: Vec<char> = Vec::new();
        let mut pos = [0.0f32; 3];
        for ch in &j.channels {
            let v = values[cursor];
            cursor += 1;
            match ch {
                Channel::Xpos => pos[0] = v,
                Channel::Ypos => pos[1] = v,
                Channel::Zpos => pos[2] = v,
                Channel::Xrot => {
                    euler[0] = v;
                    order.push('X');
                }
                Channel::Yrot => {
                    euler[1] = v;
                    order.push('Y');
                }
                Channel::Zrot => {
                    euler[2] = v;
                    order.push('Z');
                }
            }
        }
        // 回転順序はチャンネル出現順。
        let ord = [
            *order.first().unwrap_or(&'Z'),
            *order.get(1).unwrap_or(&'X'),
            *order.get(2).unwrap_or(&'Y'),
        ];
        rotations.push(euler_to_quat(euler, ord));
        if ji == 0 {
            root_translation = pos;
        }
    }
    Frame {
        rotations,
        root_translation,
    }
}

fn expect<'a, I: Iterator<Item = &'a str>>(
    it: &mut std::iter::Peekable<I>,
    want: &str,
) -> Result<(), BvhError> {
    match it.next() {
        Some(t) if t == want => Ok(()),
        Some(t) => Err(BvhError::Parse(format!("expected {want}, got {t}"))),
        None => Err(BvhError::Parse(format!("expected {want}, got EOF"))),
    }
}

fn read_f32<'a, I: Iterator<Item = &'a str>>(
    it: &mut std::iter::Peekable<I>,
) -> Result<f32, BvhError> {
    it.next()
        .ok_or_else(|| err("number"))?
        .parse()
        .map_err(|_| err("float parse"))
}

fn err(what: &str) -> BvhError {
    BvhError::Parse(what.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"HIERARCHY
ROOT Hips
{
  OFFSET 0.00 0.00 0.00
  CHANNELS 6 Xposition Yposition Zposition Zrotation Xrotation Yrotation
  JOINT Spine
  {
    OFFSET 0.00 5.00 0.00
    CHANNELS 3 Zrotation Xrotation Yrotation
    End Site
    {
      OFFSET 0.00 5.00 0.00
    }
  }
}
MOTION
Frames: 2
Frame Time: 0.033333
0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0 0.0
1.0 2.0 3.0 0.0 0.0 90.0 0.0 0.0 0.0
"#;

    #[test]
    fn parses_hierarchy_and_frames() {
        let clip = parse_bvh(SAMPLE, "test").unwrap();
        assert_eq!(clip.joint_count(), 2);
        assert_eq!(clip.joints[0].name, "Hips");
        assert_eq!(clip.joints[1].name, "Spine");
        assert_eq!(clip.joints[1].parent, Some(0));
        assert_eq!(clip.joints[1].offset, [0.0, 5.0, 0.0]);
        assert_eq!(clip.frame_count(), 2);
        assert!((clip.frame_time - 0.033333).abs() < 1e-6);
    }

    #[test]
    fn decodes_root_translation() {
        let clip = parse_bvh(SAMPLE, "test").unwrap();
        assert_eq!(clip.frames[1].root_translation, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_bvh("not a bvh", "x").is_err());
    }
}
