use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use glam::Vec3;

use crate::{
    renderer::Camera,
    runtime::state::{
        is_terminal_size_unstable, CameraDirectorState, CameraShot, RuntimeContrastPreset,
        RuntimeInputResult, SYNC_OFFSET_LIMIT_MS, SYNC_OFFSET_STEP_MS,
    },
    scene::{
        BrailleProfile, CameraControlMode, CameraFocusMode, CinematicCameraMode, ColorMode,
        FreeFlyState, SceneCpu,
    },
};

pub(crate) fn process_runtime_input(
    orbit_enabled: &mut bool,
    orbit_speed: &mut f32,
    model_spin_enabled: &mut bool,
    zoom: &mut f32,
    focus_offset: &mut Vec3,
    camera_height_offset: &mut f32,
    center_lock_enabled: &mut bool,
    stage_level: &mut u8,
    sync_offset_ms: &mut i32,
    contrast_preset: &mut RuntimeContrastPreset,
    braille_profile: &mut BrailleProfile,
    color_mode: &mut ColorMode,
    cinematic_mode: &mut CinematicCameraMode,
    reactive_gain: &mut f32,
    exposure_bias: &mut f32,
    control_mode: &mut CameraControlMode,
    camera_look_speed: f32,
    freefly_state: &mut FreeFlyState,
) -> Result<RuntimeInputResult> {
    let mut result = RuntimeInputResult::default();
    while event::poll(Duration::from_millis(0))? {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Esc | KeyCode::Char('Q') => {
                    result.quit = true;
                    result.last_key = Some("q");
                    return Ok(result);
                }
                KeyCode::Char('o') | KeyCode::Char('O') => {
                    *orbit_enabled = !*orbit_enabled;
                    result.last_key = Some("o");
                    result.status_changed = true;
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    *model_spin_enabled = !*model_spin_enabled;
                    result.last_key = Some("r");
                    result.status_changed = true;
                }
                KeyCode::Char('w') | KeyCode::Char('W') => {
                    if matches!(*control_mode, CameraControlMode::FreeFly) {
                        if *center_lock_enabled {
                            *center_lock_enabled = false;
                            result.center_lock_auto_disabled = true;
                        }
                        freefly_translate(freefly_state, FreeFlyDirection::Forward);
                        result.status_changed = true;
                        result.last_key = Some("w");
                    }
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    if matches!(*control_mode, CameraControlMode::FreeFly) {
                        if *center_lock_enabled {
                            *center_lock_enabled = false;
                            result.center_lock_auto_disabled = true;
                        }
                        freefly_translate(freefly_state, FreeFlyDirection::Backward);
                        result.status_changed = true;
                        result.last_key = Some("s");
                    }
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    if matches!(*control_mode, CameraControlMode::FreeFly) {
                        if *center_lock_enabled {
                            *center_lock_enabled = false;
                            result.center_lock_auto_disabled = true;
                        }
                        freefly_translate(freefly_state, FreeFlyDirection::Left);
                        result.status_changed = true;
                        result.last_key = Some("a");
                    }
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    if matches!(*control_mode, CameraControlMode::FreeFly) {
                        if *center_lock_enabled {
                            *center_lock_enabled = false;
                            result.center_lock_auto_disabled = true;
                        }
                        freefly_translate(freefly_state, FreeFlyDirection::Right);
                        result.status_changed = true;
                        result.last_key = Some("d");
                    }
                }
                KeyCode::Char('q') => {
                    if matches!(*control_mode, CameraControlMode::FreeFly) {
                        if *center_lock_enabled {
                            *center_lock_enabled = false;
                            result.center_lock_auto_disabled = true;
                        }
                        freefly_translate(freefly_state, FreeFlyDirection::Down);
                        result.status_changed = true;
                        result.last_key = Some("q");
                    } else {
                        result.quit = true;
                        result.last_key = Some("q");
                        return Ok(result);
                    }
                }
                KeyCode::Char('e') => {
                    if matches!(*control_mode, CameraControlMode::FreeFly) {
                        if *center_lock_enabled {
                            *center_lock_enabled = false;
                            result.center_lock_auto_disabled = true;
                        }
                        freefly_translate(freefly_state, FreeFlyDirection::Up);
                        result.status_changed = true;
                        result.last_key = Some("e");
                    } else {
                        *exposure_bias = (*exposure_bias - 0.04).clamp(-0.5, 0.8);
                        result.status_changed = true;
                        result.last_key = Some("e");
                    }
                }
                KeyCode::Char('E') => {
                    *exposure_bias = (*exposure_bias + 0.04).clamp(-0.5, 0.8);
                    result.status_changed = true;
                    result.last_key = Some("E");
                }
                KeyCode::Char('+') | KeyCode::Char('=') => {
                    *stage_level = stage_level.saturating_add(1).min(4);
                    result.status_changed = true;
                    result.stage_changed = true;
                    result.last_key = Some("+");
                }
                KeyCode::Char('-') | KeyCode::Char('_') => {
                    *stage_level = stage_level.saturating_sub(1);
                    result.status_changed = true;
                    result.stage_changed = true;
                    result.last_key = Some("-");
                }
                KeyCode::Char('f') | KeyCode::Char('F') => {
                    result.freefly_toggled = true;
                    result.status_changed = true;
                    result.last_key = Some("f");
                }
                KeyCode::Char('t') | KeyCode::Char('T') => {
                    *center_lock_enabled = !*center_lock_enabled;
                    result.status_changed = true;
                    result.last_key = Some("t");
                }
                KeyCode::Char('x') | KeyCode::Char('X') => {
                    *orbit_speed = (*orbit_speed + 0.05).clamp(0.0, 3.0);
                    if *orbit_speed > 0.0 {
                        *orbit_enabled = true;
                    }
                    result.status_changed = true;
                    result.last_key = Some("x");
                }
                KeyCode::Char('z') | KeyCode::Char('Z') => {
                    *orbit_speed = (*orbit_speed - 0.05).clamp(0.0, 3.0);
                    result.status_changed = true;
                    result.last_key = Some("z");
                }
                KeyCode::Char('[') => {
                    *zoom = (*zoom + 0.08).clamp(0.2, 8.0);
                    result.zoom_changed = true;
                }
                KeyCode::Char(']') => {
                    *zoom = (*zoom - 0.08).clamp(0.2, 8.0);
                    result.zoom_changed = true;
                }
                KeyCode::Left => {
                    if matches!(*control_mode, CameraControlMode::FreeFly) {
                        if *center_lock_enabled {
                            *center_lock_enabled = false;
                            result.center_lock_auto_disabled = true;
                        }
                        freefly_rotate(freefly_state, -0.06 * camera_look_speed, 0.0);
                        result.status_changed = true;
                        result.last_key = Some("left");
                    } else if *center_lock_enabled {
                        result.center_lock_blocked_pan = true;
                    } else {
                        focus_offset.x -= 0.08;
                    }
                }
                KeyCode::Right => {
                    if matches!(*control_mode, CameraControlMode::FreeFly) {
                        if *center_lock_enabled {
                            *center_lock_enabled = false;
                            result.center_lock_auto_disabled = true;
                        }
                        freefly_rotate(freefly_state, 0.06 * camera_look_speed, 0.0);
                        result.status_changed = true;
                        result.last_key = Some("right");
                    } else if *center_lock_enabled {
                        result.center_lock_blocked_pan = true;
                    } else {
                        focus_offset.x += 0.08;
                    }
                }
                KeyCode::Up => {
                    if matches!(*control_mode, CameraControlMode::FreeFly) {
                        if *center_lock_enabled {
                            *center_lock_enabled = false;
                            result.center_lock_auto_disabled = true;
                        }
                        freefly_rotate(freefly_state, 0.0, 0.05 * camera_look_speed);
                        result.status_changed = true;
                        result.last_key = Some("up");
                    } else if *center_lock_enabled {
                        result.center_lock_blocked_pan = true;
                    } else {
                        focus_offset.y += 0.08;
                        *camera_height_offset += 0.08;
                    }
                }
                KeyCode::Down => {
                    if matches!(*control_mode, CameraControlMode::FreeFly) {
                        if *center_lock_enabled {
                            *center_lock_enabled = false;
                            result.center_lock_auto_disabled = true;
                        }
                        freefly_rotate(freefly_state, 0.0, -0.05 * camera_look_speed);
                        result.status_changed = true;
                        result.last_key = Some("down");
                    } else if *center_lock_enabled {
                        result.center_lock_blocked_pan = true;
                    } else {
                        focus_offset.y -= 0.08;
                        *camera_height_offset -= 0.08;
                    }
                }
                KeyCode::Char('j') | KeyCode::Char('J') => {
                    if *center_lock_enabled {
                        result.center_lock_blocked_pan = true;
                    } else {
                        focus_offset.x -= 0.08;
                    }
                }
                KeyCode::Char('l') | KeyCode::Char('L') => {
                    if *center_lock_enabled {
                        result.center_lock_blocked_pan = true;
                    } else {
                        focus_offset.x += 0.08;
                    }
                }
                KeyCode::Char('i') | KeyCode::Char('I') => {
                    if *center_lock_enabled {
                        result.center_lock_blocked_pan = true;
                    } else {
                        focus_offset.y += 0.08;
                        *camera_height_offset += 0.08;
                    }
                }
                KeyCode::Char('k') | KeyCode::Char('K') => {
                    if *center_lock_enabled {
                        result.center_lock_blocked_pan = true;
                    } else {
                        focus_offset.y -= 0.08;
                        *camera_height_offset -= 0.08;
                    }
                }
                KeyCode::Char('u') | KeyCode::Char('U') => {
                    if *center_lock_enabled {
                        result.center_lock_blocked_pan = true;
                    } else {
                        focus_offset.z += 0.08;
                    }
                }
                KeyCode::Char('m') | KeyCode::Char('M') => {
                    if *center_lock_enabled {
                        result.center_lock_blocked_pan = true;
                    } else {
                        focus_offset.z -= 0.08;
                    }
                }
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    *zoom = 1.0;
                    *focus_offset = Vec3::ZERO;
                    *camera_height_offset = 0.0;
                    result.status_changed = true;
                    result.zoom_changed = true;
                    result.last_key = Some("c");
                }
                KeyCode::Char(',') => {
                    *sync_offset_ms = (*sync_offset_ms - SYNC_OFFSET_STEP_MS)
                        .clamp(-SYNC_OFFSET_LIMIT_MS, SYNC_OFFSET_LIMIT_MS);
                    result.status_changed = true;
                    result.last_key = Some(",");
                }
                KeyCode::Char('.') => {
                    *sync_offset_ms = (*sync_offset_ms + SYNC_OFFSET_STEP_MS)
                        .clamp(-SYNC_OFFSET_LIMIT_MS, SYNC_OFFSET_LIMIT_MS);
                    result.status_changed = true;
                    result.last_key = Some(".");
                }
                KeyCode::Char('/') => {
                    *sync_offset_ms = 0;
                    result.status_changed = true;
                    result.last_key = Some("/");
                }
                KeyCode::Char('v') | KeyCode::Char('V') => {
                    *contrast_preset = contrast_preset.next();
                    result.status_changed = true;
                    result.last_key = Some("v");
                }
                KeyCode::Char('b') | KeyCode::Char('B') => {
                    *braille_profile = match *braille_profile {
                        BrailleProfile::Safe => BrailleProfile::Normal,
                        BrailleProfile::Normal => BrailleProfile::Dense,
                        BrailleProfile::Dense => BrailleProfile::Safe,
                    };
                    result.status_changed = true;
                    result.last_key = Some("b");
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    *color_mode = match *color_mode {
                        ColorMode::Mono => ColorMode::Ansi,
                        ColorMode::Ansi => ColorMode::Mono,
                    };
                    result.status_changed = true;
                    result.last_key = Some("n");
                }
                KeyCode::Char('p') | KeyCode::Char('P') => {
                    *cinematic_mode = match *cinematic_mode {
                        CinematicCameraMode::Off => CinematicCameraMode::On,
                        _ => CinematicCameraMode::Off,
                    };
                    result.status_changed = true;
                    result.last_key = Some("p");
                }
                KeyCode::Char('g') => {
                    *reactive_gain = (*reactive_gain - 0.05).clamp(0.0, 1.0);
                    result.status_changed = true;
                    result.last_key = Some("g");
                }
                KeyCode::Char('G') => {
                    *reactive_gain = (*reactive_gain + 0.05).clamp(0.0, 1.0);
                    result.status_changed = true;
                    result.last_key = Some("G");
                }
                _ => {}
            },
            Event::Resize(width, height) => {
                if is_terminal_size_unstable(width, height) {
                    result.terminal_size_unstable = true;
                    result.resized_terminal = None;
                } else {
                    result.terminal_size_unstable = false;
                    result.resized_terminal = Some((width, height));
                }
                result.status_changed = true;
                result.resized = true;
            }
            _ => {}
        }
    }
    Ok(result)
}

pub(crate) fn update_camera_director(
    state: &mut CameraDirectorState,
    mode: CinematicCameraMode,
    focus_mode: CameraFocusMode,
    elapsed_wall: f32,
    smoothed_energy: f32,
    reactive_gain: f32,
    extent_y: f32,
    jitter_scale: f32,
) -> (f32, f32, f32, f32) {
    if matches!(mode, CinematicCameraMode::Off) {
        return camera_shot_values(CameraShot::FullBody, extent_y);
    }
    if !matches!(focus_mode, CameraFocusMode::Auto) {
        let shot = match focus_mode {
            CameraFocusMode::Auto | CameraFocusMode::Full => CameraShot::FullBody,
            CameraFocusMode::Upper => CameraShot::UpperBody,
            CameraFocusMode::Face => CameraShot::FaceCloseup,
            CameraFocusMode::Hands => CameraShot::Hands,
        };
        return camera_shot_values(shot, extent_y);
    }

    let dt = (elapsed_wall - state.total_time_accum).max(0.0);
    state.total_time_accum = elapsed_wall;
    if matches!(state.shot, CameraShot::FaceCloseup) {
        state.face_time_accum += dt;
    }

    let mut should_cut = elapsed_wall >= state.next_cut_at;
    let face_ratio = if state.total_time_accum > 0.0 {
        state.face_time_accum / state.total_time_accum
    } else {
        0.0
    };
    if smoothed_energy > 0.72 && (elapsed_wall - state.transition_started_at) > 2.5 {
        should_cut = true;
    }
    if should_cut {
        let next_shot = match state.shot {
            CameraShot::FullBody => CameraShot::UpperBody,
            CameraShot::UpperBody => {
                if face_ratio < 0.25 {
                    CameraShot::FaceCloseup
                } else {
                    CameraShot::FullBody
                }
            }
            CameraShot::FaceCloseup => CameraShot::Hands,
            CameraShot::Hands => CameraShot::FullBody,
        };
        state.shot = next_shot;
        state.transition_started_at = elapsed_wall;
        state.previous_radius_mul = state.radius_mul;
        state.previous_height_offset = state.height_offset;
        state.previous_focus_y_offset = state.focus_y_offset;
        let (radius_mul, height_off, focus_y_off, base_duration) = match state.shot {
            CameraShot::FullBody => (1.0, 0.0, 0.0, 6.0),
            CameraShot::UpperBody => (0.66, extent_y * 0.08, extent_y * 0.16, 5.0),
            CameraShot::FaceCloseup => (0.42, extent_y * 0.26, extent_y * 0.39, 3.0),
            CameraShot::Hands => (0.52, extent_y * 0.04, extent_y * 0.12, 3.8),
        };
        state.radius_mul = radius_mul;
        state.height_offset = height_off;
        state.focus_y_offset = focus_y_off;
        let energy_advance = (smoothed_energy * 1.6).clamp(0.0, 1.0);
        state.next_cut_at = elapsed_wall + (base_duration - energy_advance).clamp(2.2, 8.0);
    }

    let transition_t = ((elapsed_wall - state.transition_started_at) / 0.35).clamp(0.0, 1.0);
    let eased_t = transition_t * transition_t * (3.0 - 2.0 * transition_t);
    let radius_mul =
        state.previous_radius_mul + (state.radius_mul - state.previous_radius_mul) * eased_t;
    let height_off = state.previous_height_offset
        + (state.height_offset - state.previous_height_offset) * eased_t;
    let focus_y_off = state.previous_focus_y_offset
        + (state.focus_y_offset - state.previous_focus_y_offset) * eased_t;

    state.jitter_phase += 0.09;
    let jitter_gain = match mode {
        CinematicCameraMode::On => 1.0,
        CinematicCameraMode::Aggressive => 1.7,
        CinematicCameraMode::Off => 0.0,
    };
    let jitter = (state.jitter_phase * 0.8).sin()
        * 0.015
        * smoothed_energy
        * reactive_gain
        * jitter_gain
        * jitter_scale;
    (radius_mul, height_off, focus_y_off, jitter)
}

fn camera_shot_values(shot: CameraShot, extent_y: f32) -> (f32, f32, f32, f32) {
    match shot {
        CameraShot::FullBody => (1.0, 0.0, 0.0, 0.0),
        CameraShot::UpperBody => (0.66, extent_y * 0.08, extent_y * 0.16, 0.0),
        CameraShot::FaceCloseup => (0.42, extent_y * 0.26, extent_y * 0.39, 0.0),
        CameraShot::Hands => (0.52, extent_y * 0.04, extent_y * 0.12, 0.0),
    }
}

#[derive(Debug, Clone, Copy)]
enum FreeFlyDirection {
    Forward,
    Backward,
    Left,
    Right,
    Up,
    Down,
}

pub(crate) fn freefly_state_from_camera(camera: Camera, move_speed: f32) -> FreeFlyState {
    let forward = (camera.target - camera.eye).normalize_or_zero();
    let direction = if forward.length_squared() <= f32::EPSILON {
        Vec3::new(0.0, 0.0, -1.0)
    } else {
        forward
    };
    let pitch = direction.y.clamp(-1.0, 1.0).asin();
    let yaw = direction.z.atan2(direction.x);
    FreeFlyState {
        eye: camera.eye,
        target: camera.target,
        yaw,
        pitch,
        move_speed: move_speed.clamp(0.1, 8.0),
    }
}

fn freefly_forward(state: &FreeFlyState) -> Vec3 {
    let cp = state.pitch.cos();
    Vec3::new(
        state.yaw.cos() * cp,
        state.pitch.sin(),
        state.yaw.sin() * cp,
    )
    .normalize_or_zero()
}

pub(crate) fn freefly_camera(state: FreeFlyState) -> Camera {
    Camera {
        eye: state.eye,
        target: state.target,
        up: Vec3::Y,
    }
}

fn freefly_translate(state: &mut FreeFlyState, direction: FreeFlyDirection) {
    let mut forward = (state.target - state.eye).normalize_or_zero();
    if forward.length_squared() <= f32::EPSILON {
        forward = freefly_forward(state);
    }
    if forward.length_squared() <= f32::EPSILON {
        forward = Vec3::new(0.0, 0.0, -1.0);
    }
    let mut right = forward.cross(Vec3::Y).normalize_or_zero();
    if right.length_squared() <= f32::EPSILON {
        right = Vec3::X;
    }
    let up = Vec3::Y;
    let axis = match direction {
        FreeFlyDirection::Forward => forward,
        FreeFlyDirection::Backward => -forward,
        FreeFlyDirection::Left => -right,
        FreeFlyDirection::Right => right,
        FreeFlyDirection::Up => up,
        FreeFlyDirection::Down => -up,
    };
    let step = 0.12 * state.move_speed.clamp(0.1, 8.0);
    let delta = axis * step;
    state.eye += delta;
    state.target += delta;
}

fn freefly_rotate(state: &mut FreeFlyState, yaw_delta: f32, pitch_delta: f32) {
    state.yaw += yaw_delta;
    state.pitch = (state.pitch + pitch_delta).clamp(-1.45, 1.45);
    let forward = freefly_forward(state);
    if forward.length_squared() <= f32::EPSILON {
        return;
    }
    let distance = (state.target - state.eye).length().max(0.5);
    state.target = state.eye + forward * distance;
}

pub(crate) fn orbit_camera(
    orbit_angle: f32,
    orbit_radius: f32,
    camera_height: f32,
    focus: Vec3,
) -> Camera {
    let eye_x = focus.x + orbit_angle.cos() * orbit_radius;
    let eye_z = focus.z + orbit_angle.sin() * orbit_radius;
    let eye = Vec3::new(eye_x, camera_height, eye_z);
    let target = focus;
    Camera {
        eye,
        target,
        up: Vec3::Y,
    }
}

pub(crate) fn max_scene_vertices(scene: &SceneCpu) -> usize {
    scene
        .meshes
        .iter()
        .map(|mesh| mesh.positions.len())
        .max()
        .unwrap_or(0)
}
