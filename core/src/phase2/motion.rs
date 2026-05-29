//! Phase2/3 共通のモーション中間表現（MotionClip）。
//! 外部取込（BVH 等）や AI 生成の出力をこの形に正規化し、リターゲット → ycd へ流す。

use serde::{Deserialize, Serialize};

/// 四元数 [x, y, z, w]。
pub type Quat = [f32; 4];
/// 3 次元ベクトル。
pub type Vec3 = [f32; 3];

/// スケルトンの 1 関節（ボーン）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Joint {
    pub name: String,
    /// 親関節の index。ルートは None。
    pub parent: Option<usize>,
    /// 親からのオフセット（バインドポーズ）。
    pub offset: Vec3,
}

/// 1 フレーム分の各関節のローカル変換。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// 関節ごとのローカル回転（joints と同じ並び）。
    pub rotations: Vec<Quat>,
    /// ルートのローカル移動（ルート関節のみ意味を持つ）。
    pub root_translation: Vec3,
}

/// 取り込んだ/生成したモーション 1 本。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MotionClip {
    pub name: String,
    pub joints: Vec<Joint>,
    pub frames: Vec<Frame>,
    /// フレーム間隔（秒）。
    pub frame_time: f32,
}

impl MotionClip {
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
    pub fn joint_count(&self) -> usize {
        self.joints.len()
    }
    /// 総再生時間（秒）。
    pub fn duration(&self) -> f32 {
        if self.frames.is_empty() {
            0.0
        } else {
            self.frames.len() as f32 * self.frame_time
        }
    }
    pub fn joint_index(&self, name: &str) -> Option<usize> {
        self.joints.iter().position(|j| j.name == name)
    }
}

/// 任意順の Euler（度, [X,Y,Z] 固定インデックス）を四元数へ。
/// order は適用順の軸（例 ['Z','X','Y']）。各軸の角度は angles_deg から軸対応で引く。
pub fn euler_to_quat(angles_deg: [f32; 3], order: [char; 3]) -> Quat {
    let mut q: Quat = [0.0, 0.0, 0.0, 1.0];
    for axis in order {
        let angle = match axis {
            'X' | 'x' => angles_deg[0],
            'Y' | 'y' => angles_deg[1],
            'Z' | 'z' => angles_deg[2],
            _ => 0.0,
        };
        q = quat_mul(q, axis_angle(axis, angle.to_radians()));
    }
    normalize(q)
}

fn axis_angle(axis: char, rad: f32) -> Quat {
    let (s, c) = (rad * 0.5).sin_cos();
    match axis {
        'X' | 'x' => [s, 0.0, 0.0, c],
        'Y' | 'y' => [0.0, s, 0.0, c],
        'Z' | 'z' => [0.0, 0.0, s, c],
        _ => [0.0, 0.0, 0.0, 1.0],
    }
}

/// q1 * q2（[x,y,z,w]）。
pub fn quat_mul(a: Quat, b: Quat) -> Quat {
    let [ax, ay, az, aw] = a;
    let [bx, by, bz, bw] = b;
    [
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
        aw * bw - ax * bx - ay * by - az * bz,
    ]
}

pub fn normalize(q: Quat) -> Quat {
    let n = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if n == 0.0 {
        [0.0, 0.0, 0.0, 1.0]
    } else {
        [q[0] / n, q[1] / n, q[2] / n, q[3] / n]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_euler_is_identity_quat() {
        let q = euler_to_quat([0.0, 0.0, 0.0], ['Z', 'X', 'Y']);
        assert!((q[3].abs() - 1.0).abs() < 1e-6);
        assert!(q[0].abs() < 1e-6 && q[1].abs() < 1e-6 && q[2].abs() < 1e-6);
    }

    #[test]
    fn ninety_deg_z_rotation() {
        let q = euler_to_quat([0.0, 0.0, 90.0], ['Z', 'X', 'Y']);
        // 90° about Z → z = sin(45°), w = cos(45°)
        let s = (45f32).to_radians().sin();
        assert!((q[2] - s).abs() < 1e-5, "z={} expected {}", q[2], s);
        assert!((q[3] - s).abs() < 1e-5);
    }

    #[test]
    fn quat_is_normalized() {
        let q = euler_to_quat([30.0, 45.0, 60.0], ['Z', 'X', 'Y']);
        let n = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
        assert!((n - 1.0).abs() < 1e-5);
    }

    #[test]
    fn motion_clip_duration() {
        let clip = MotionClip {
            name: "t".into(),
            joints: vec![Joint {
                name: "root".into(),
                parent: None,
                offset: [0.0; 3],
            }],
            frames: vec![
                Frame {
                    rotations: vec![[0.0, 0.0, 0.0, 1.0]],
                    root_translation: [0.0; 3],
                },
                Frame {
                    rotations: vec![[0.0, 0.0, 0.0, 1.0]],
                    root_translation: [0.0; 3],
                },
            ],
            frame_time: 0.033,
        };
        assert_eq!(clip.frame_count(), 2);
        assert!((clip.duration() - 0.066).abs() < 1e-6);
        assert_eq!(clip.joint_index("root"), Some(0));
    }
}
