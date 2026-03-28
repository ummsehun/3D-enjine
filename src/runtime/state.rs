use std::path::PathBuf;

use glam::Vec3;

use crate::{
    renderer::{Camera, RenderStats},
    scene::{
        AnsiQuantization, BrailleProfile, CameraAlignPreset, CameraControlMode, CameraFocusMode,
        CameraMode, CenterLockMode, CinematicCameraMode, ColorMode, ContrastProfile, DetailProfile,
        KittyCompression, KittyTransport, PerfProfile, RenderBackend, RenderConfig, RenderMode,
        RenderOutputMode, TextureSamplingMode,
    },
};

pub(crate) const SYNC_OFFSET_STEP_MS: i32 = 10;
pub(crate) const SYNC_OFFSET_LIMIT_MS: i32 = 5_000;
pub(crate) const MAX_RENDER_COLS: u16 = 4096;
pub(crate) const MAX_RENDER_ROWS: u16 = 2048;
pub(crate) const VISIBILITY_LOW_THRESHOLD: f32 = 0.002;
pub(crate) const VISIBILITY_LOW_FRAMES_TO_RECOVER: u32 = 12;
pub(crate) const LOW_VIS_EXPOSURE_THRESHOLD: f32 = 0.008;
pub(crate) const LOW_VIS_EXPOSURE_TRIGGER_FRAMES: u32 = 6;
pub(crate) const LOW_VIS_EXPOSURE_RECOVER_THRESHOLD: f32 = 0.020;
pub(crate) const LOW_VIS_EXPOSURE_RECOVER_FRAMES: u32 = 24;
pub(crate) const MIN_VISIBLE_HEIGHT_RATIO: f32 = 0.10;
pub(crate) const MIN_VISIBLE_HEIGHT_TRIGGER_FRAMES: u32 = 10;
pub(crate) const MIN_VISIBLE_HEIGHT_RECOVER_RATIO: f32 = 0.16;
pub(crate) const MIN_VISIBLE_HEIGHT_RECOVER_FRAMES: u32 = 30;
pub(crate) const HYBRID_GRAPHICS_MAX_CELLS: usize = 24_000;
pub(crate) const HYBRID_GRAPHICS_SLOW_FRAME_MS: f32 = 45.0;
pub(crate) const HYBRID_GRAPHICS_SLOW_STREAK_LIMIT: u32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeContrastPreset {
    AdaptiveLow,
    AdaptiveNormal,
    AdaptiveHigh,
    Fixed,
}

impl RuntimeContrastPreset {
    pub(crate) fn from_profile(profile: ContrastProfile) -> Self {
        match profile {
            ContrastProfile::Adaptive => RuntimeContrastPreset::AdaptiveNormal,
            ContrastProfile::Fixed => RuntimeContrastPreset::Fixed,
        }
    }

    pub(crate) fn next(self) -> Self {
        match self {
            RuntimeContrastPreset::AdaptiveLow => RuntimeContrastPreset::AdaptiveNormal,
            RuntimeContrastPreset::AdaptiveNormal => RuntimeContrastPreset::AdaptiveHigh,
            RuntimeContrastPreset::AdaptiveHigh => RuntimeContrastPreset::Fixed,
            RuntimeContrastPreset::Fixed => RuntimeContrastPreset::AdaptiveLow,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            RuntimeContrastPreset::AdaptiveLow => "adaptive-low",
            RuntimeContrastPreset::AdaptiveNormal => "adaptive-normal",
            RuntimeContrastPreset::AdaptiveHigh => "adaptive-high",
            RuntimeContrastPreset::Fixed => "fixed",
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct ContinuousSyncState {
    pub(crate) anim_time: f32,
    pub(crate) initialized: bool,
    pub(crate) drift_ema: f32,
    pub(crate) hard_snap_count: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeCameraSettings {
    pub(crate) mode: CameraMode,
    pub(crate) align_preset: CameraAlignPreset,
    pub(crate) unit_scale: f32,
    pub(crate) vmd_fps: f32,
    pub(crate) vmd_path: Option<PathBuf>,
    pub(crate) look_speed: f32,
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct ReactiveState {
    pub(crate) energy: f32,
    pub(crate) smoothed_energy: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CameraShot {
    FullBody,
    UpperBody,
    FaceCloseup,
    Hands,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CameraDirectorState {
    pub(crate) shot: CameraShot,
    pub(crate) next_cut_at: f32,
    pub(crate) transition_started_at: f32,
    pub(crate) previous_radius_mul: f32,
    pub(crate) previous_height_offset: f32,
    pub(crate) previous_focus_y_offset: f32,
    pub(crate) radius_mul: f32,
    pub(crate) height_offset: f32,
    pub(crate) focus_y_offset: f32,
    pub(crate) face_time_accum: f32,
    pub(crate) total_time_accum: f32,
    pub(crate) jitter_phase: f32,
}

impl Default for CameraDirectorState {
    fn default() -> Self {
        Self {
            shot: CameraShot::FullBody,
            next_cut_at: 6.0,
            transition_started_at: 0.0,
            previous_radius_mul: 1.0,
            previous_height_offset: 0.0,
            previous_focus_y_offset: 0.0,
            radius_mul: 1.0,
            height_offset: 0.0,
            focus_y_offset: 0.0,
            face_time_accum: 0.0,
            total_time_accum: 0.0,
            jitter_phase: 0.0,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct RuntimeInputResult {
    pub(crate) quit: bool,
    pub(crate) status_changed: bool,
    pub(crate) resized: bool,
    pub(crate) terminal_size_unstable: bool,
    pub(crate) resized_terminal: Option<(u16, u16)>,
    pub(crate) stage_changed: bool,
    pub(crate) center_lock_blocked_pan: bool,
    pub(crate) center_lock_auto_disabled: bool,
    pub(crate) freefly_toggled: bool,
    pub(crate) zoom_changed: bool,
    pub(crate) last_key: Option<&'static str>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RuntimeCameraState {
    pub(crate) control_mode: CameraControlMode,
    pub(crate) previous_control_mode: CameraControlMode,
    pub(crate) track_enabled: bool,
    pub(crate) active_track_mode: CameraMode,
    pub(crate) saved_track_mode: CameraMode,
}

impl RuntimeCameraState {
    pub(crate) fn new(
        control_mode: CameraControlMode,
        track_mode: CameraMode,
        has_track_source: bool,
    ) -> Self {
        let track_capable = has_track_source && !matches!(track_mode, CameraMode::Off);
        let effective_control_mode = if track_capable {
            CameraControlMode::Orbit
        } else {
            control_mode
        };
        Self {
            control_mode: effective_control_mode,
            previous_control_mode: effective_control_mode,
            track_enabled: track_capable,
            active_track_mode: track_mode,
            saved_track_mode: track_mode,
        }
    }

    pub(crate) fn toggle_freefly(&mut self, has_track_source: bool) -> bool {
        if !matches!(self.control_mode, CameraControlMode::FreeFly) {
            self.previous_control_mode = self.control_mode;
            self.control_mode = CameraControlMode::FreeFly;
            if self.track_enabled {
                self.saved_track_mode = self.active_track_mode;
            }
            self.track_enabled = false;
            true
        } else {
            self.control_mode = if matches!(self.previous_control_mode, CameraControlMode::FreeFly)
            {
                CameraControlMode::Orbit
            } else {
                self.previous_control_mode
            };
            self.active_track_mode = self.saved_track_mode;
            self.track_enabled =
                has_track_source && !matches!(self.active_track_mode, CameraMode::Off);
            false
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColorPathLevel {
    Truecolor,
    Q216,
    Mono,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ColorRecoveryState {
    pub(crate) level: ColorPathLevel,
    pub(crate) target_level: ColorPathLevel,
    pub(crate) auto_recover: bool,
    pub(crate) success_streak: u32,
}

impl ColorRecoveryState {
    pub(crate) fn from_requested(
        requested_color: ColorMode,
        requested_quantization: AnsiQuantization,
        auto_recover: bool,
    ) -> Self {
        let target_level = if matches!(requested_color, ColorMode::Mono) {
            ColorPathLevel::Mono
        } else if matches!(requested_quantization, AnsiQuantization::Off) {
            ColorPathLevel::Truecolor
        } else {
            ColorPathLevel::Q216
        };
        Self {
            level: target_level,
            target_level,
            auto_recover,
            success_streak: 0,
        }
    }

    pub(crate) fn set_requested(
        &mut self,
        requested_color: ColorMode,
        requested_quantization: AnsiQuantization,
    ) {
        self.target_level = if matches!(requested_color, ColorMode::Mono) {
            ColorPathLevel::Mono
        } else if matches!(requested_quantization, AnsiQuantization::Off) {
            ColorPathLevel::Truecolor
        } else {
            ColorPathLevel::Q216
        };
        self.level = self.target_level;
        self.success_streak = 0;
    }

    pub(crate) fn degrade(&mut self, ascii_force_color_active: bool, mode: RenderMode) -> bool {
        self.success_streak = 0;
        let previous = self.level;
        self.level = match self.level {
            ColorPathLevel::Truecolor => ColorPathLevel::Q216,
            ColorPathLevel::Q216 => {
                if matches!(mode, RenderMode::Ascii) && ascii_force_color_active {
                    ColorPathLevel::Q216
                } else {
                    ColorPathLevel::Mono
                }
            }
            ColorPathLevel::Mono => ColorPathLevel::Mono,
        };
        self.level != previous
    }

    pub(crate) fn on_present_success(&mut self) -> bool {
        if !self.auto_recover {
            self.success_streak = 0;
            return false;
        }
        if self.level == self.target_level {
            self.success_streak = 0;
            return false;
        }
        self.success_streak = self.success_streak.saturating_add(1);
        let threshold = match self.level {
            ColorPathLevel::Mono => 150,
            ColorPathLevel::Q216 => 210,
            ColorPathLevel::Truecolor => u32::MAX,
        };
        if self.success_streak < threshold {
            return false;
        }
        self.success_streak = 0;
        self.level = match self.level {
            ColorPathLevel::Mono => ColorPathLevel::Q216,
            ColorPathLevel::Q216 => ColorPathLevel::Truecolor,
            ColorPathLevel::Truecolor => ColorPathLevel::Truecolor,
        };
        true
    }

    pub(crate) fn apply(
        &self,
        color_mode: &mut ColorMode,
        quantization: &mut AnsiQuantization,
        mode: RenderMode,
        ascii_force_color_active: bool,
    ) {
        match self.level {
            ColorPathLevel::Truecolor => {
                *color_mode = ColorMode::Ansi;
                *quantization = AnsiQuantization::Off;
            }
            ColorPathLevel::Q216 => {
                *color_mode = ColorMode::Ansi;
                *quantization = AnsiQuantization::Q216;
            }
            ColorPathLevel::Mono => {
                if matches!(mode, RenderMode::Ascii) && ascii_force_color_active {
                    *color_mode = ColorMode::Ansi;
                    *quantization = AnsiQuantization::Q216;
                } else {
                    *color_mode = ColorMode::Mono;
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct OrbitState {
    pub(crate) angle: f32,
    pub(crate) speed: f32,
    pub(crate) enabled: bool,
}

impl OrbitState {
    pub(crate) fn new(initial_speed: f32) -> Self {
        Self {
            angle: std::f32::consts::FRAC_PI_2,
            speed: initial_speed.max(0.0),
            enabled: initial_speed > 0.0,
        }
    }

    pub(crate) fn advance(&mut self, dt: f32) {
        if self.enabled && self.speed > 0.0 {
            self.angle += self.speed * dt.max(0.0);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RuntimeAdaptiveQuality {
    pub(crate) target_frame_ms: f32,
    pub(crate) ema_frame_ms: f32,
    pub(crate) lod_level: usize,
    pub(crate) overload_streak: u32,
    pub(crate) underload_streak: u32,
}

impl RuntimeAdaptiveQuality {
    pub(crate) fn new(profile: PerfProfile) -> Self {
        Self {
            target_frame_ms: target_frame_ms(profile),
            ema_frame_ms: target_frame_ms(profile),
            lod_level: 0,
            overload_streak: 0,
            underload_streak: 0,
        }
    }

    pub(crate) fn observe(&mut self, frame_ms: f32) -> bool {
        self.ema_frame_ms += (frame_ms - self.ema_frame_ms) * 0.12;
        let high = self.target_frame_ms * 1.18;
        let low = self.target_frame_ms * 0.82;
        let mut changed = false;

        if self.ema_frame_ms > high {
            self.overload_streak = self.overload_streak.saturating_add(1);
            self.underload_streak = 0;
            if self.overload_streak >= 20 && self.lod_level < 2 {
                self.lod_level += 1;
                self.overload_streak = 0;
                changed = true;
            }
        } else if self.ema_frame_ms < low {
            self.underload_streak = self.underload_streak.saturating_add(1);
            self.overload_streak = 0;
            if self.underload_streak >= 60 && self.lod_level > 0 {
                self.lod_level -= 1;
                self.underload_streak = 0;
                changed = true;
            }
        } else {
            self.overload_streak = 0;
            self.underload_streak = 0;
        }
        changed
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct VisibilityWatchdog {
    pub(crate) low_visible_streak: u32,
}

impl VisibilityWatchdog {
    pub(crate) fn observe(&mut self, visible_ratio: f32) -> bool {
        if visible_ratio < VISIBILITY_LOW_THRESHOLD {
            self.low_visible_streak = self.low_visible_streak.saturating_add(1);
        } else {
            self.low_visible_streak = 0;
        }
        self.low_visible_streak >= VISIBILITY_LOW_FRAMES_TO_RECOVER
    }

    pub(crate) fn reset(&mut self) {
        self.low_visible_streak = 0;
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct CenterLockState {
    pub(crate) err_x_ema: f32,
    pub(crate) err_y_ema: f32,
}

impl CenterLockState {
    pub(crate) fn apply_camera_space(
        &mut self,
        stats: &RenderStats,
        mode: CenterLockMode,
        frame_width: u16,
        frame_height: u16,
        camera: &mut Camera,
        fov_deg: f32,
        cell_aspect: f32,
        extent_y: f32,
    ) {
        let fw = f32::from(frame_width.max(1));
        let fh = f32::from(frame_height.max(1));
        let root_in_view = stats.root_screen_px.filter(|(x, y)| {
            x.is_finite() && y.is_finite() && *x >= 0.0 && *x <= fw && *y >= 0.0 && *y <= fh
        });
        let anchor = match mode {
            CenterLockMode::Root => stats
                .subject_centroid_px
                .or(root_in_view)
                .or(stats.visible_centroid_px),
            CenterLockMode::Mixed => match (
                root_in_view,
                stats.subject_centroid_px.or(stats.visible_centroid_px),
            ) {
                (Some(root), Some(centroid)) => Some((
                    root.0 * 0.7 + centroid.0 * 0.3,
                    root.1 * 0.7 + centroid.1 * 0.3,
                )),
                (Some(root), None) => Some(root),
                (None, Some(centroid)) => Some(centroid),
                (None, None) => root_in_view,
            },
        };
        let Some((cx, cy)) = anchor else {
            self.err_x_ema *= 0.85;
            self.err_y_ema *= 0.85;
            return;
        };

        if cx < -fw * 0.25 || cx > fw * 1.25 || cy < -fh * 0.25 || cy > fh * 1.25 {
            self.err_x_ema *= 0.85;
            self.err_y_ema *= 0.85;
            return;
        }
        let nx = ((cx / fw - 0.5) * 2.0).clamp(-1.0, 1.0);
        let ny = ((cy / fh - 0.5) * 2.0).clamp(-1.0, 1.0);
        let dead_x = if nx.abs() < 0.015 { 0.0 } else { nx };
        let dead_y = if ny.abs() < 0.020 { 0.0 } else { ny };

        let large_error = dead_x.abs() > 0.35 || dead_y.abs() > 0.35;
        if large_error {
            self.err_x_ema = dead_x;
            self.err_y_ema = dead_y;
        } else {
            self.err_x_ema += (dead_x - self.err_x_ema) * 0.28;
            self.err_y_ema += (dead_y - self.err_y_ema) * 0.28;
        }

        let extent = extent_y.max(0.5);
        let mut forward = camera.target - camera.eye;
        if forward.length_squared() <= f32::EPSILON {
            return;
        }
        forward = forward.normalize();
        let mut right = forward.cross(camera.up);
        if right.length_squared() <= f32::EPSILON {
            return;
        }
        right = right.normalize();
        let mut up = right.cross(forward);
        if up.length_squared() <= f32::EPSILON {
            return;
        }
        up = up.normalize();

        let dist = (camera.target - camera.eye).length().max(0.2);
        let fov_y = fov_deg.to_radians().clamp(0.35, 2.6);
        let aspect = ((fw * cell_aspect.max(0.15)).max(1.0) / fh.max(1.0)).clamp(0.3, 5.0);
        let tan_y = (fov_y * 0.5).tan().max(0.01);
        let fov_x = 2.0 * (tan_y * aspect).atan();
        let tan_x = (fov_x * 0.5).tan().max(0.01);
        let shift_x = (self.err_x_ema * dist * tan_x * 0.95).clamp(-extent * 0.9, extent * 0.9);
        let shift_y = (-self.err_y_ema * dist * tan_y * 0.95).clamp(-extent * 0.75, extent * 0.75);
        let shift = right * shift_x + up * shift_y;
        camera.eye += shift;
        camera.target += shift;
    }

    pub(crate) fn reset(&mut self) {
        self.err_x_ema = 0.0;
        self.err_y_ema = 0.0;
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ScreenFitController {
    pub(crate) auto_zoom_gain: f32,
}

impl Default for ScreenFitController {
    fn default() -> Self {
        Self {
            auto_zoom_gain: 1.0,
        }
    }
}

impl ScreenFitController {
    pub(crate) fn on_resize(&mut self) {
        self.auto_zoom_gain = 1.0;
    }

    pub(crate) fn on_manual_zoom(&mut self) {
        self.auto_zoom_gain = self.auto_zoom_gain.clamp(0.55, 1.80);
    }

    pub(crate) fn target_for_mode(mode: RenderMode) -> f32 {
        match mode {
            RenderMode::Ascii => 0.72,
            RenderMode::Braille => 0.66,
        }
    }

    pub(crate) fn update(&mut self, visible_height_ratio: f32, mode: RenderMode, enabled: bool) {
        if !enabled {
            self.auto_zoom_gain = 1.0;
            return;
        }
        if !visible_height_ratio.is_finite() || visible_height_ratio <= 0.0 {
            return;
        }
        let target = Self::target_for_mode(mode);
        let err = target - visible_height_ratio;
        if err.abs() <= 0.02 {
            return;
        }
        let factor = (1.0 + err * 0.22).clamp(0.90, 1.10);
        self.auto_zoom_gain = (self.auto_zoom_gain * factor).clamp(0.55, 1.80);
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct ExposureAutoBoost {
    pub(crate) low_streak: u32,
    pub(crate) high_streak: u32,
    pub(crate) boost: f32,
}

impl ExposureAutoBoost {
    pub(crate) fn on_resize(&mut self) {
        self.low_streak = 0;
        self.high_streak = 0;
        self.boost = 0.0;
    }

    pub(crate) fn update(&mut self, visible_ratio: f32) {
        if visible_ratio < LOW_VIS_EXPOSURE_THRESHOLD {
            self.low_streak = self.low_streak.saturating_add(1);
            self.high_streak = 0;
            if self.low_streak >= LOW_VIS_EXPOSURE_TRIGGER_FRAMES {
                self.boost = (self.boost + 0.06).clamp(0.0, 0.45);
                self.low_streak = 0;
            }
            return;
        }

        if visible_ratio > LOW_VIS_EXPOSURE_RECOVER_THRESHOLD {
            self.high_streak = self.high_streak.saturating_add(1);
            self.low_streak = 0;
            if self.high_streak >= LOW_VIS_EXPOSURE_RECOVER_FRAMES {
                self.boost = (self.boost - 0.03).clamp(0.0, 0.45);
                self.high_streak = 0;
            }
            return;
        }

        self.low_streak = 0;
        self.high_streak = 0;
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct AutoRadiusGuard {
    pub(crate) low_height_streak: u32,
    pub(crate) recover_streak: u32,
    pub(crate) shrink_ratio: f32,
}

impl AutoRadiusGuard {
    pub(crate) fn update(&mut self, height_ratio: f32, enabled: bool) -> f32 {
        if !enabled {
            self.low_height_streak = 0;
            self.recover_streak = 0;
            self.shrink_ratio = 0.0;
            return 0.0;
        }

        if height_ratio < MIN_VISIBLE_HEIGHT_RATIO {
            self.low_height_streak = self.low_height_streak.saturating_add(1);
            self.recover_streak = 0;
            if self.low_height_streak >= MIN_VISIBLE_HEIGHT_TRIGGER_FRAMES {
                self.shrink_ratio = (self.shrink_ratio + 0.02).clamp(0.0, 0.12);
                self.low_height_streak = 0;
            }
        } else if height_ratio > MIN_VISIBLE_HEIGHT_RECOVER_RATIO {
            self.recover_streak = self.recover_streak.saturating_add(1);
            self.low_height_streak = 0;
            if self.recover_streak >= MIN_VISIBLE_HEIGHT_RECOVER_FRAMES {
                self.shrink_ratio = (self.shrink_ratio - 0.02).clamp(0.0, 0.12);
                self.recover_streak = 0;
            }
        } else {
            self.low_height_streak = 0;
            self.recover_streak = 0;
        }
        self.shrink_ratio
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct DistanceClampGuard {
    pub(crate) last_eye: Option<Vec3>,
}

impl DistanceClampGuard {
    pub(crate) fn apply(
        &mut self,
        camera: &mut Camera,
        subject_target: Vec3,
        extent_y: f32,
        alpha: f32,
    ) -> f32 {
        let min_dist = (extent_y * 0.42).clamp(0.35, 1.20);
        let to_eye = camera.eye - subject_target;
        let dist = to_eye.length();
        let mut desired_eye = camera.eye;
        if dist < min_dist {
            let dir = if dist <= f32::EPSILON {
                Vec3::new(0.0, 0.0, 1.0)
            } else {
                to_eye / dist
            };
            desired_eye = subject_target + dir * min_dist;
        }
        let base_eye = self.last_eye.unwrap_or(camera.eye);
        let a = alpha.clamp(0.0, 1.0);
        camera.eye = base_eye + (desired_eye - base_eye) * a;
        self.last_eye = Some(camera.eye);
        min_dist
    }

    pub(crate) fn reset(&mut self) {
        self.last_eye = None;
    }
}

pub(crate) fn dynamic_clip_planes(
    min_dist: f32,
    extent_y: f32,
    camera_dist: f32,
    has_stage: bool,
) -> (f32, f32) {
    let near = (min_dist * 0.06).clamp(0.015, 0.10);
    let subject_far = min_dist + extent_y * 6.0;
    let far_target = if has_stage {
        subject_far.max(camera_dist + extent_y * 16.0)
    } else {
        subject_far
    };
    let far = far_target.clamp(near + 3.0, 500.0);
    (near, far)
}

pub(crate) fn target_frame_ms(profile: PerfProfile) -> f32 {
    match profile {
        PerfProfile::Balanced => 33.3,
        PerfProfile::Cinematic => 50.0,
        PerfProfile::Smooth => 22.2,
    }
}

pub(crate) fn apply_runtime_contrast_preset(
    config: &mut RenderConfig,
    preset: RuntimeContrastPreset,
) {
    match preset {
        RuntimeContrastPreset::AdaptiveLow => {
            config.contrast_profile = ContrastProfile::Adaptive;
            config.contrast_floor = 0.08;
            config.contrast_gamma = 1.00;
            config.fog_scale = 1.00;
        }
        RuntimeContrastPreset::AdaptiveNormal => {
            config.contrast_profile = ContrastProfile::Adaptive;
            config.contrast_floor = 0.10;
            config.contrast_gamma = 0.90;
            config.fog_scale = 1.00;
        }
        RuntimeContrastPreset::AdaptiveHigh => {
            config.contrast_profile = ContrastProfile::Adaptive;
            config.contrast_floor = 0.14;
            config.contrast_gamma = 0.78;
            config.fog_scale = 0.80;
        }
        RuntimeContrastPreset::Fixed => {}
    }
}

pub(crate) fn cap_render_size(width: u16, height: u16) -> (u16, u16, bool) {
    if width == 0 || height == 0 {
        return (1, 1, false);
    }
    if width <= MAX_RENDER_COLS && height <= MAX_RENDER_ROWS {
        return (width, height, false);
    }
    let scale_w = (MAX_RENDER_COLS as f32) / (width as f32);
    let scale_h = (MAX_RENDER_ROWS as f32) / (height as f32);
    let scale = scale_w.min(scale_h).clamp(0.01, 1.0);
    let capped_w = ((width as f32) * scale).floor() as u16;
    let capped_h = ((height as f32) * scale).floor() as u16;
    (capped_w.max(1), capped_h.max(1), true)
}

pub(crate) fn is_terminal_size_unstable(width: u16, height: u16) -> bool {
    if width == 0 || height == 0 {
        return true;
    }
    if width == u16::MAX || height == u16::MAX {
        return true;
    }
    let w = width as u32;
    let h = height as u32;
    let max_w = (MAX_RENDER_COLS as u32) * 8;
    let max_h = (MAX_RENDER_ROWS as u32) * 8;
    w > max_w || h > max_h
}

pub(crate) fn apply_distant_subject_clarity_boost(
    config: &mut RenderConfig,
    subject_height_ratio: f32,
) {
    if !config.quality_auto_distance
        || !subject_height_ratio.is_finite()
        || subject_height_ratio <= 0.0
    {
        return;
    }
    let target = config.subject_target_height_ratio.clamp(0.20, 0.95);
    let distant_threshold = (target * 0.65).clamp(0.14, 0.52);
    let near_threshold = (target * 1.35).clamp(0.45, 0.98);

    if subject_height_ratio < distant_threshold {
        let t = ((distant_threshold - subject_height_ratio) / distant_threshold).clamp(0.0, 1.0);
        config.model_lift = (config.model_lift + 0.10 * t).clamp(0.02, 0.55);
        config.edge_accent_strength = (config.edge_accent_strength + 0.55 * t).clamp(0.0, 2.0);
        config.bg_suppression = (config.bg_suppression + 0.70 * t).clamp(0.0, 1.0);
        config.min_triangle_area_px2 = (config.min_triangle_area_px2 * (1.0 - 0.85 * t)).max(0.0);
        if t > 0.30 {
            config.triangle_stride = config.triangle_stride.saturating_sub(1).max(1);
        }
        if t > 0.70 {
            config.triangle_stride = config.triangle_stride.saturating_sub(1).max(1);
        }
        return;
    }

    if subject_height_ratio > near_threshold {
        let t = ((subject_height_ratio - near_threshold) / near_threshold).clamp(0.0, 1.0);
        config.edge_accent_strength = (config.edge_accent_strength * (1.0 - 0.4 * t)).max(0.05);
        config.bg_suppression = (config.bg_suppression + 0.10 * t).clamp(0.0, 1.0);
    }
}

pub(crate) fn apply_face_focus_detail_boost(config: &mut RenderConfig, subject_height_ratio: f32) {
    if !matches!(config.camera_focus, CameraFocusMode::Face) {
        return;
    }
    let ratio = subject_height_ratio.clamp(0.0, 1.0);
    let t = if ratio < 0.28 {
        ((0.28 - ratio) / 0.28).clamp(0.0, 1.0)
    } else {
        0.0
    };
    config.texture_mip_bias = (config.texture_mip_bias - 0.85 - 0.65 * t).clamp(-2.0, 4.0);
    config.edge_accent_strength = (config.edge_accent_strength + 0.20 + 0.30 * t).clamp(0.0, 2.0);
    config.bg_suppression = (config.bg_suppression + 0.16 + 0.22 * t).clamp(0.0, 1.0);
    if matches!(config.texture_sampling, TextureSamplingMode::Nearest) {
        config.texture_sampling = TextureSamplingMode::Bilinear;
    }
    if config.triangle_stride > 1 {
        config.triangle_stride = config.triangle_stride.saturating_sub(1);
    }
}

pub(crate) fn apply_pmx_surface_guardrails(
    config: &mut RenderConfig,
    is_pmx_scene: bool,
    subject_height_ratio: f32,
) {
    if !is_pmx_scene || !subject_height_ratio.is_finite() || subject_height_ratio <= 0.0 {
        return;
    }

    let target = config.subject_target_height_ratio.clamp(0.20, 0.95);
    let guardrail_threshold = (target * 0.92).clamp(0.35, 0.72);
    if subject_height_ratio >= guardrail_threshold {
        return;
    }

    let t = ((guardrail_threshold - subject_height_ratio) / guardrail_threshold).clamp(0.0, 1.0);
    config.triangle_stride = 1;
    config.min_triangle_area_px2 = (config.min_triangle_area_px2 * (1.0 - 0.75 * t)).max(0.0);
    config.min_triangle_area_px2 = config
        .min_triangle_area_px2
        .min((0.12 - 0.06 * t).max(0.04));
    config.edge_accent_strength = config.edge_accent_strength.min((0.26 - 0.10 * t).max(0.16));
}

pub(crate) fn apply_adaptive_quality_tuning(
    config: &mut RenderConfig,
    base_triangle_stride: usize,
    base_min_triangle_area_px2: f32,
    lod_level: usize,
) {
    let mut effective_lod = lod_level;
    if matches!(config.detail_profile, DetailProfile::Perf) {
        effective_lod = effective_lod.max(1);
    }
    config.triangle_stride = base_triangle_stride.max(match effective_lod {
        0 => 1,
        1 => 2,
        _ => 3,
    });
    config.min_triangle_area_px2 = base_min_triangle_area_px2.max(match effective_lod {
        0 => 0.0,
        1 => 0.6,
        _ => 1.2,
    });

    if effective_lod >= 1 {
        config.texture_sampling = TextureSamplingMode::Nearest;
    }
    if effective_lod >= 2 && matches!(config.detail_profile, DetailProfile::Perf) {
        config.material_color = false;
    }
}

pub(crate) fn jitter_scale_for_lod(lod_level: usize) -> f32 {
    match lod_level {
        0 => 1.0,
        1 => 0.65,
        _ => 0.35,
    }
}

pub(crate) fn resolve_runtime_backend(requested: RenderBackend) -> RenderBackend {
    match requested {
        RenderBackend::Cpu => RenderBackend::Cpu,
        RenderBackend::Gpu => {
            #[cfg(feature = "gpu")]
            {
                use crate::render::gpu::GpuRenderer;
                if GpuRenderer::is_available() {
                    RenderBackend::Gpu
                } else {
                    eprintln!(
                        "warning: gpu backend requested but no suitable gpu found; falling back to cpu."
                    );
                    RenderBackend::Cpu
                }
            }
            #[cfg(not(feature = "gpu"))]
            {
                eprintln!(
                    "warning: gpu backend requested but gpu feature not enabled; falling back to cpu."
                );
                RenderBackend::Cpu
            }
        }
    }
}

pub(crate) fn normalize_graphics_settings(config: &mut RenderConfig) -> Option<String> {
    if !matches!(
        config.output_mode,
        RenderOutputMode::Hybrid | RenderOutputMode::KittyHq
    ) {
        return None;
    }
    if matches!(config.kitty_transport, KittyTransport::Shm)
        && matches!(config.kitty_compression, KittyCompression::Zlib)
    {
        config.kitty_compression = KittyCompression::None;
        return Some("kitty transport=shm forces compression=none".to_owned());
    }
    None
}

pub(crate) fn format_runtime_status(
    sync_offset_ms: i32,
    sync_speed: f32,
    effective_aspect: f32,
    contrast: RuntimeContrastPreset,
    braille_profile: BrailleProfile,
    color_mode: ColorMode,
    cinematic_mode: CinematicCameraMode,
    reactive_gain: f32,
    exposure_bias: f32,
    stage_level: u8,
    center_lock: bool,
    lod_level: usize,
    target_ms: f32,
    frame_ema_ms: f32,
    sync_profile_hit: Option<bool>,
    sync_profile_dirty: bool,
    drift_ema: f32,
    hard_snap_count: u32,
    notice: Option<&str>,
) -> String {
    let profile_label = match sync_profile_hit {
        Some(true) => "hit",
        Some(false) => "miss",
        None => "off",
    };
    let core = format!(
        "offset={sync_offset_ms}ms  speed={sync_speed:.4}x  aspect={effective_aspect:.3}  contrast={}  braille={:?}  color={:?}  camera={:?}  gain={reactive_gain:.2}  exp={exposure_bias:+.2}  stage={}  center={}  lod={}  target={target_ms:.1}ms  ema={frame_ema_ms:.1}ms  profile={}{}  drift={drift_ema:.4}  snaps={hard_snap_count}",
        contrast.label(),
        braille_profile,
        color_mode,
        cinematic_mode,
        stage_level,
        if center_lock { "on" } else { "off" },
        lod_level,
        profile_label,
        if sync_profile_dirty { "*" } else { "" },
    );
    if let Some(extra) = notice {
        format!("{core}  note={extra}")
    } else {
        core
    }
}

pub(crate) fn overlay_osd(frame: &mut crate::renderer::FrameBuffers, text: &str) {
    if frame.width == 0 || frame.height == 0 {
        return;
    }
    let width = usize::from(frame.width);
    let y = usize::from(frame.height.saturating_sub(1));
    let row_start = y * width;
    let row_end = row_start + width;
    for glyph in &mut frame.glyphs[row_start..row_end] {
        *glyph = ' ';
    }
    for color in &mut frame.fg_rgb[row_start..row_end] {
        *color = [235, 235, 235];
    }
    for (i, ch) in text.chars().take(width).enumerate() {
        frame.glyphs[row_start + i] = ch;
    }
}
