use glam::{EulerRot, Quat, Vec3};

use crate::{assets::vmd_camera::VmdCameraTrack, scene::CameraAlignPreset};

#[derive(Debug, Clone, Copy)]
pub struct CameraPose {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov_deg: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct MmdCameraTransform {
    pub unit_scale: f32,
    pub flip_z: bool,
    pub rot_sign: [f32; 3],
    pub align_preset: CameraAlignPreset,
}

impl MmdCameraTransform {
    pub fn from_preset(preset: CameraAlignPreset, unit_scale: f32) -> Self {
        match preset {
            CameraAlignPreset::Std => Self {
                unit_scale: unit_scale.clamp(0.01, 2.0),
                flip_z: true,
                rot_sign: [1.0, -1.0, -1.0],
                align_preset: preset,
            },
            CameraAlignPreset::AltA => Self {
                unit_scale: unit_scale.clamp(0.01, 2.0),
                flip_z: true,
                rot_sign: [1.0, -1.0, -1.0],
                align_preset: preset,
            },
            CameraAlignPreset::AltB => Self {
                unit_scale: unit_scale.clamp(0.01, 2.0),
                flip_z: false,
                rot_sign: [1.0, 1.0, 1.0],
                align_preset: preset,
            },
        }
    }

    fn convert_position(self, p: Vec3) -> Vec3 {
        let scaled = Vec3::new(
            p.x * self.unit_scale,
            p.y * self.unit_scale,
            if self.flip_z {
                -p.z * self.unit_scale
            } else {
                p.z * self.unit_scale
            },
        );
        match self.align_preset {
            CameraAlignPreset::Std => scaled,
            CameraAlignPreset::AltA => Vec3::new(scaled.x, scaled.z, -scaled.y),
            CameraAlignPreset::AltB => Vec3::new(scaled.x, -scaled.y, scaled.z),
        }
    }

    fn convert_rotation(self, r: Vec3) -> Vec3 {
        let signed = Vec3::new(
            r.x * self.rot_sign[0],
            r.y * self.rot_sign[1],
            r.z * self.rot_sign[2],
        );
        match self.align_preset {
            CameraAlignPreset::Std => signed,
            CameraAlignPreset::AltA => Vec3::new(signed.x, signed.z, signed.y),
            CameraAlignPreset::AltB => Vec3::new(signed.x, -signed.y, -signed.z),
        }
    }

    fn distance_sign(self) -> f32 {
        match self.align_preset {
            CameraAlignPreset::Std | CameraAlignPreset::AltA => 1.0,
            CameraAlignPreset::AltB => -1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CameraTrackSampler {
    keyframes: Vec<CameraTrackKey>,
    duration_secs: f32,
}

#[derive(Debug, Clone, Copy)]
struct CameraTrackKey {
    time_sec: f32,
    distance: f32,
    position: Vec3,
    rotation: Vec3,
    fov_deg: f32,
    interp: [u8; 24],
}

impl CameraTrackSampler {
    pub fn from_vmd(track: &VmdCameraTrack, fps: f32) -> Option<Self> {
        if track.keyframes.is_empty() {
            return None;
        }
        let fps = fps.max(1.0);
        let mut keyframes = Vec::with_capacity(track.keyframes.len());
        for frame in &track.keyframes {
            keyframes.push(CameraTrackKey {
                time_sec: frame.frame_no as f32 / fps,
                distance: frame.distance,
                position: frame.position,
                rotation: frame.rotation,
                fov_deg: frame.fov_deg,
                interp: frame.interpolation,
            });
        }
        keyframes.sort_by(|a, b| a.time_sec.total_cmp(&b.time_sec));
        let duration_secs = keyframes.last().map(|k| k.time_sec).unwrap_or(0.0).max(0.0);
        Some(Self {
            keyframes,
            duration_secs,
        })
    }

    pub fn duration_secs(&self) -> f32 {
        self.duration_secs
    }

    pub fn sample_pose(
        &self,
        time_sec: f32,
        transform: MmdCameraTransform,
        looping: bool,
    ) -> Option<CameraPose> {
        if self.keyframes.is_empty() {
            return None;
        }
        let t = normalize_time(time_sec, self.duration_secs, looping);
        let (i0, i1, alpha) = sample_segment_by_time(&self.keyframes, t);
        let a = self.keyframes[i0];
        let b = self.keyframes[i1];
        let interp = sample_camera_interpolation(a.interp, alpha);

        let position_mmd = Vec3::new(
            lerp(a.position.x, b.position.x, interp[0]),
            lerp(a.position.y, b.position.y, interp[1]),
            lerp(a.position.z, b.position.z, interp[2]),
        );
        let rotation_mmd = Vec3::new(
            lerp(a.rotation.x, b.rotation.x, interp[3]),
            lerp(a.rotation.y, b.rotation.y, interp[3]),
            lerp(a.rotation.z, b.rotation.z, interp[3]),
        );
        let fov_deg = lerp(a.fov_deg, b.fov_deg, interp[5]).clamp(10.0, 120.0);
        let distance = lerp(a.distance.abs(), b.distance.abs(), interp[4]) * transform.unit_scale;

        let target = transform.convert_position(position_mmd);
        let r = transform.convert_rotation(rotation_mmd);
        let rot = Quat::from_euler(EulerRot::XYZ, r.x, r.y, r.z);
        let eye = target + rot * Vec3::new(0.0, 0.0, distance * transform.distance_sign());
        let mut up = (rot * Vec3::Y).normalize_or_zero();
        if up.length_squared() <= f32::EPSILON {
            up = Vec3::Y;
        }
        Some(CameraPose {
            eye,
            target,
            up,
            fov_deg,
        })
    }
}

fn normalize_time(time_sec: f32, duration_secs: f32, looping: bool) -> f32 {
    if duration_secs <= f32::EPSILON {
        return 0.0;
    }
    if looping {
        time_sec.rem_euclid(duration_secs)
    } else {
        time_sec.clamp(0.0, duration_secs)
    }
}

fn sample_segment_by_time(keys: &[CameraTrackKey], time_sec: f32) -> (usize, usize, f32) {
    if keys.len() == 1 || time_sec <= keys[0].time_sec {
        return (0, 0, 0.0);
    }
    let last = keys.len() - 1;
    if time_sec >= keys[last].time_sec {
        return (last, last, 0.0);
    }
    let upper = keys
        .partition_point(|key| key.time_sec < time_sec)
        .min(last);
    let lower = upper.saturating_sub(1);
    let t0 = keys[lower].time_sec;
    let t1 = keys[upper].time_sec;
    if (t1 - t0).abs() <= f32::EPSILON {
        return (lower, upper, 0.0);
    }
    (lower, upper, ((time_sec - t0) / (t1 - t0)).clamp(0.0, 1.0))
}

fn sample_camera_interpolation(interp: [u8; 24], alpha: f32) -> [f32; 6] {
    [
        vmd_bezier_channel([interp[0], interp[1], interp[2], interp[3]], alpha),
        vmd_bezier_channel([interp[4], interp[5], interp[6], interp[7]], alpha),
        vmd_bezier_channel([interp[8], interp[9], interp[10], interp[11]], alpha),
        vmd_bezier_channel([interp[12], interp[13], interp[14], interp[15]], alpha),
        vmd_bezier_channel([interp[16], interp[17], interp[18], interp[19]], alpha),
        vmd_bezier_channel([interp[20], interp[21], interp[22], interp[23]], alpha),
    ]
}

fn vmd_bezier_channel(ctrl: [u8; 4], alpha: f32) -> f32 {
    let t = alpha.clamp(0.0, 1.0);
    let x1 = (ctrl[0] as f32 / 127.0).clamp(0.0, 1.0);
    let y1 = (ctrl[1] as f32 / 127.0).clamp(0.0, 1.0);
    let x2 = (ctrl[2] as f32 / 127.0).clamp(0.0, 1.0);
    let y2 = (ctrl[3] as f32 / 127.0).clamp(0.0, 1.0);
    if (x1 - y1).abs() < 1e-5 && (x2 - y2).abs() < 1e-5 {
        return t;
    }

    // Find the bezier parameter u so that bezier_x(u) ~= t.
    let mut lo = 0.0f32;
    let mut hi = 1.0f32;
    let mut u = t;
    for _ in 0..14 {
        u = (lo + hi) * 0.5;
        let x = cubic_bezier(u, 0.0, x1, x2, 1.0);
        if x < t {
            lo = u;
        } else {
            hi = u;
        }
    }
    cubic_bezier(u, 0.0, y1, y2, 1.0).clamp(0.0, 1.0)
}

fn cubic_bezier(t: f32, p0: f32, p1: f32, p2: f32, p3: f32) -> f32 {
    let it = 1.0 - t;
    it * it * it * p0 + 3.0 * it * it * t * p1 + 3.0 * it * t * t * p2 + t * t * t * p3
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn estimate_subject_centroid(track: &VmdCameraTrack) -> Option<Vec3> {
    if track.keyframes.is_empty() {
        return None;
    }
    let mut sum = Vec3::ZERO;
    for key in &track.keyframes {
        sum += key.position;
    }
    Some(sum / (track.keyframes.len() as f32))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::vmd_camera::VmdCameraKeyframe;

    fn sample_track() -> VmdCameraTrack {
        VmdCameraTrack {
            model_name: "test".to_owned(),
            keyframes: vec![
                VmdCameraKeyframe {
                    frame_no: 0,
                    distance: 8.0,
                    position: Vec3::new(0.0, 10.0, 20.0),
                    rotation: Vec3::new(0.1, 0.2, 0.3),
                    interpolation: [20; 24],
                    fov_deg: 45.0,
                    perspective: true,
                },
                VmdCameraKeyframe {
                    frame_no: 30,
                    distance: 10.0,
                    position: Vec3::new(5.0, 15.0, 30.0),
                    rotation: Vec3::new(0.2, 0.3, 0.4),
                    interpolation: [20; 24],
                    fov_deg: 40.0,
                    perspective: true,
                },
            ],
            max_frame: 30,
        }
    }

    #[test]
    fn std_transform_flips_z_and_scales() {
        let tf = MmdCameraTransform::from_preset(CameraAlignPreset::Std, 0.08);
        let p = tf.convert_position(Vec3::new(1.0, 2.0, 3.0));
        assert!((p.x - 0.08).abs() < 1e-6);
        assert!((p.y - 0.16).abs() < 1e-6);
        assert!((p.z + 0.24).abs() < 1e-6);
    }

    #[test]
    fn sampler_returns_pose() {
        let track = sample_track();
        let sampler = CameraTrackSampler::from_vmd(&track, 30.0).expect("sampler");
        let pose = sampler
            .sample_pose(
                0.5,
                MmdCameraTransform::from_preset(CameraAlignPreset::Std, 0.08),
                true,
            )
            .expect("pose");
        assert!(pose.fov_deg >= 10.0);
        assert!(pose.eye.is_finite());
    }

    #[test]
    fn bezier_linear_fallback_behaves() {
        let v = vmd_bezier_channel([0, 0, 127, 127], 0.4);
        assert!((v - 0.4).abs() < 0.1);
    }
}
