use std::path::{Path, PathBuf};

use crate::{
    cli::{BenchArgs, RunArgs, RunSceneArg, StartArgs},
    runtime::{
        config::GasciiConfig,
        sync_profile::{
            build_profile_key, default_profile_store_path, SyncProfileEntry, SyncProfileMode,
            SyncProfileStore,
        },
    },
    scene::{
        AnsiQuantization, AudioReactiveMode, BrailleProfile, CameraAlignPreset, CameraControlMode,
        CameraFocusMode, CameraMode, CellAspectMode, CenterLockMode, CinematicCameraMode,
        ClarityProfile, ColorMode, ContrastProfile, DetailProfile, GraphicsProtocol,
        KittyCompression, KittyInternalResPreset, KittyPipelineMode, KittyTransport, PerfProfile,
        RecoverStrategy, RenderBackend, RenderMode, RenderOutputMode, StageRole, SyncPolicy,
        SyncSpeedMode, TextureSamplingMode, ThemeStyle,
    },
};

const SYNC_OFFSET_LIMIT_MS: i32 = 5_000;

#[derive(Debug, Clone)]
pub(crate) struct ResolvedVisualOptions {
    pub(crate) output_mode: RenderOutputMode,
    pub(crate) recover_color_auto: bool,
    pub(crate) graphics_protocol: GraphicsProtocol,
    pub(crate) kitty_transport: KittyTransport,
    pub(crate) kitty_compression: KittyCompression,
    pub(crate) kitty_internal_res: KittyInternalResPreset,
    pub(crate) kitty_pipeline_mode: KittyPipelineMode,
    pub(crate) recover_strategy: RecoverStrategy,
    pub(crate) kitty_scale: f32,
    pub(crate) hq_target_fps: u32,
    pub(crate) subject_exposure_only: bool,
    pub(crate) subject_target_height_ratio: f32,
    pub(crate) subject_target_width_ratio: f32,
    pub(crate) quality_auto_distance: bool,
    pub(crate) texture_mip_bias: f32,
    pub(crate) stage_as_sub_only: bool,
    pub(crate) stage_role: StageRole,
    pub(crate) stage_luma_cap: f32,
    pub(crate) cell_aspect_mode: CellAspectMode,
    pub(crate) cell_aspect_trim: f32,
    pub(crate) contrast_profile: ContrastProfile,
    pub(crate) perf_profile: PerfProfile,
    pub(crate) detail_profile: DetailProfile,
    pub(crate) backend: RenderBackend,
    pub(crate) exposure_bias: f32,
    pub(crate) center_lock: bool,
    pub(crate) center_lock_mode: CenterLockMode,
    pub(crate) wasd_mode: CameraControlMode,
    pub(crate) freefly_speed: f32,
    pub(crate) camera_look_speed: f32,
    pub(crate) camera_mode: CameraMode,
    pub(crate) camera_align_preset: CameraAlignPreset,
    pub(crate) camera_unit_scale: f32,
    pub(crate) camera_vmd_fps: f32,
    pub(crate) camera_vmd_path: Option<PathBuf>,
    pub(crate) camera_focus: CameraFocusMode,
    pub(crate) material_color: bool,
    pub(crate) texture_sampling: TextureSamplingMode,
    pub(crate) texture_v_origin: crate::scene::TextureVOrigin,
    pub(crate) texture_sampler: crate::scene::TextureSamplerMode,
    pub(crate) clarity_profile: ClarityProfile,
    pub(crate) ansi_quantization: AnsiQuantization,
    pub(crate) model_lift: f32,
    pub(crate) edge_accent_strength: f32,
    pub(crate) bg_suppression: f32,
    pub(crate) braille_aspect_compensation: f32,
    pub(crate) stage_level: u8,
    pub(crate) stage_reactive: bool,
    pub(crate) color_mode: Option<ColorMode>,
    pub(crate) ascii_force_color: bool,
    pub(crate) braille_profile: BrailleProfile,
    pub(crate) theme_style: ThemeStyle,
    pub(crate) audio_reactive: AudioReactiveMode,
    pub(crate) cinematic_camera: CinematicCameraMode,
    pub(crate) reactive_gain: f32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ResolvedSyncOptions {
    pub(crate) sync_offset_ms: i32,
    pub(crate) sync_speed_mode: SyncSpeedMode,
    pub(crate) sync_policy: SyncPolicy,
    pub(crate) sync_hard_snap_ms: u32,
    pub(crate) sync_kp: f32,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedSyncProfileOptions {
    pub(crate) mode: SyncProfileMode,
    pub(crate) profile_dir: PathBuf,
    pub(crate) key_override: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeSyncProfileContext {
    pub(crate) mode: SyncProfileMode,
    pub(crate) store_path: PathBuf,
    pub(crate) key: String,
    pub(crate) hit: bool,
}

pub(crate) fn resolve_visual_options_for_start(
    args: &StartArgs,
    runtime_cfg: &GasciiConfig,
) -> ResolvedVisualOptions {
    ResolvedVisualOptions {
        output_mode: args
            .output_mode
            .map(Into::into)
            .unwrap_or(runtime_cfg.output_mode),
        recover_color_auto: args
            .recover_color
            .map(Into::into)
            .unwrap_or(runtime_cfg.recover_color_auto),
        graphics_protocol: args
            .graphics_protocol
            .map(Into::into)
            .unwrap_or(runtime_cfg.graphics_protocol),
        kitty_transport: args
            .kitty_transport
            .map(Into::into)
            .unwrap_or(runtime_cfg.kitty_transport),
        kitty_compression: args
            .kitty_compression
            .map(Into::into)
            .unwrap_or(runtime_cfg.kitty_compression),
        kitty_internal_res: args
            .kitty_internal_res
            .map(Into::into)
            .unwrap_or(runtime_cfg.kitty_internal_res),
        kitty_pipeline_mode: args
            .kitty_pipeline
            .map(Into::into)
            .unwrap_or(runtime_cfg.kitty_pipeline_mode),
        recover_strategy: args
            .recover_strategy
            .map(Into::into)
            .unwrap_or(runtime_cfg.recover_strategy),
        kitty_scale: args
            .kitty_scale
            .unwrap_or(runtime_cfg.kitty_scale)
            .clamp(0.5, 2.0),
        hq_target_fps: args
            .hq_target_fps
            .unwrap_or(runtime_cfg.hq_target_fps)
            .clamp(12, 120),
        subject_exposure_only: args
            .subject_exposure_only
            .map(Into::into)
            .unwrap_or(runtime_cfg.subject_exposure_only),
        subject_target_height_ratio: args
            .subject_target_height
            .unwrap_or(runtime_cfg.subject_target_height_ratio)
            .clamp(0.20, 0.95),
        subject_target_width_ratio: args
            .subject_target_width
            .unwrap_or(runtime_cfg.subject_target_width_ratio)
            .clamp(0.10, 0.95),
        quality_auto_distance: args
            .quality_auto_distance
            .map(Into::into)
            .unwrap_or(runtime_cfg.quality_auto_distance),
        texture_mip_bias: args
            .texture_mip_bias
            .unwrap_or(runtime_cfg.texture_mip_bias)
            .clamp(-2.0, 4.0),
        stage_as_sub_only: args
            .stage_sub_only
            .map(Into::into)
            .unwrap_or(runtime_cfg.stage_as_sub_only),
        stage_role: args
            .stage_role
            .map(Into::into)
            .unwrap_or(runtime_cfg.stage_role),
        stage_luma_cap: args
            .stage_luma_cap
            .unwrap_or(runtime_cfg.stage_luma_cap)
            .clamp(0.0, 1.0),
        cell_aspect_mode: args
            .cell_aspect_mode
            .map(Into::into)
            .unwrap_or(runtime_cfg.cell_aspect_mode),
        cell_aspect_trim: args
            .cell_aspect_trim
            .unwrap_or(runtime_cfg.cell_aspect_trim)
            .clamp(0.70, 1.30),
        contrast_profile: args
            .contrast_profile
            .map(Into::into)
            .unwrap_or(runtime_cfg.contrast_profile),
        perf_profile: args
            .perf_profile
            .map(Into::into)
            .unwrap_or(runtime_cfg.perf_profile),
        detail_profile: args
            .detail_profile
            .map(Into::into)
            .unwrap_or(runtime_cfg.detail_profile),
        backend: args.backend.map(Into::into).unwrap_or(runtime_cfg.backend),
        exposure_bias: args
            .exposure_bias
            .unwrap_or(runtime_cfg.exposure_bias)
            .clamp(-0.5, 0.8),
        center_lock: args
            .center_lock
            .map(Into::into)
            .unwrap_or(runtime_cfg.center_lock),
        center_lock_mode: args
            .center_lock_mode
            .map(Into::into)
            .unwrap_or(runtime_cfg.center_lock_mode),
        wasd_mode: args
            .wasd_mode
            .map(Into::into)
            .unwrap_or(runtime_cfg.wasd_mode),
        freefly_speed: args
            .freefly_speed
            .unwrap_or(runtime_cfg.freefly_speed)
            .clamp(0.1, 8.0),
        camera_look_speed: args
            .camera_look_speed
            .unwrap_or(runtime_cfg.camera_look_speed)
            .clamp(0.1, 8.0),
        camera_mode: args
            .camera_mode
            .map(Into::into)
            .unwrap_or(runtime_cfg.camera_mode),
        camera_align_preset: args
            .camera_align_preset
            .map(Into::into)
            .unwrap_or(runtime_cfg.camera_align_preset),
        camera_unit_scale: args
            .camera_unit_scale
            .unwrap_or(runtime_cfg.camera_unit_scale)
            .clamp(0.01, 2.0),
        camera_vmd_fps: args
            .camera_vmd_fps
            .unwrap_or(runtime_cfg.camera_vmd_fps)
            .clamp(1.0, 240.0),
        camera_vmd_path: args
            .camera_vmd
            .clone()
            .or(runtime_cfg.camera_vmd_path.clone()),
        camera_focus: args
            .camera_focus
            .map(Into::into)
            .unwrap_or(runtime_cfg.camera_focus),
        material_color: args
            .material_color
            .map(Into::into)
            .unwrap_or(runtime_cfg.material_color),
        texture_sampling: args
            .texture_sampling
            .map(Into::into)
            .unwrap_or(runtime_cfg.texture_sampling),
        texture_v_origin: args
            .texture_v_origin
            .map(Into::into)
            .unwrap_or(runtime_cfg.texture_v_origin),
        texture_sampler: args
            .texture_sampler
            .map(Into::into)
            .unwrap_or(runtime_cfg.texture_sampler),
        clarity_profile: args
            .clarity_profile
            .map(Into::into)
            .unwrap_or(runtime_cfg.clarity_profile),
        ansi_quantization: args
            .ansi_quantization
            .map(Into::into)
            .unwrap_or(runtime_cfg.ansi_quantization),
        model_lift: args
            .model_lift
            .unwrap_or(runtime_cfg.model_lift)
            .clamp(0.02, 0.45),
        edge_accent_strength: args
            .edge_accent_strength
            .unwrap_or(runtime_cfg.edge_accent_strength)
            .clamp(0.0, 1.5),
        bg_suppression: runtime_cfg.bg_suppression.clamp(0.0, 1.0),
        braille_aspect_compensation: runtime_cfg.braille_aspect_compensation,
        stage_level: args.stage_level.unwrap_or(runtime_cfg.stage_level).min(4),
        stage_reactive: runtime_cfg.stage_reactive,
        color_mode: args.color_mode.map(Into::into).or(runtime_cfg.color_mode),
        ascii_force_color: args
            .ascii_force_color
            .map(Into::into)
            .unwrap_or(runtime_cfg.ascii_force_color),
        braille_profile: args
            .braille_profile
            .map(Into::into)
            .unwrap_or(runtime_cfg.braille_profile),
        theme_style: args
            .theme
            .map(Into::into)
            .unwrap_or(runtime_cfg.theme_style),
        audio_reactive: args
            .audio_reactive
            .map(Into::into)
            .unwrap_or(runtime_cfg.audio_reactive),
        cinematic_camera: args
            .cinematic_camera
            .map(Into::into)
            .unwrap_or(runtime_cfg.cinematic_camera),
        reactive_gain: args
            .reactive_gain
            .unwrap_or(runtime_cfg.reactive_gain)
            .clamp(0.0, 1.0),
    }
}

pub(crate) fn resolve_visual_options_for_run(
    args: &RunArgs,
    runtime_cfg: &GasciiConfig,
) -> ResolvedVisualOptions {
    ResolvedVisualOptions {
        output_mode: args
            .output_mode
            .map(Into::into)
            .unwrap_or(runtime_cfg.output_mode),
        recover_color_auto: args
            .recover_color
            .map(Into::into)
            .unwrap_or(runtime_cfg.recover_color_auto),
        graphics_protocol: args
            .graphics_protocol
            .map(Into::into)
            .unwrap_or(runtime_cfg.graphics_protocol),
        kitty_transport: args
            .kitty_transport
            .map(Into::into)
            .unwrap_or(runtime_cfg.kitty_transport),
        kitty_compression: args
            .kitty_compression
            .map(Into::into)
            .unwrap_or(runtime_cfg.kitty_compression),
        kitty_internal_res: args
            .kitty_internal_res
            .map(Into::into)
            .unwrap_or(runtime_cfg.kitty_internal_res),
        kitty_pipeline_mode: args
            .kitty_pipeline
            .map(Into::into)
            .unwrap_or(runtime_cfg.kitty_pipeline_mode),
        recover_strategy: args
            .recover_strategy
            .map(Into::into)
            .unwrap_or(runtime_cfg.recover_strategy),
        kitty_scale: args
            .kitty_scale
            .unwrap_or(runtime_cfg.kitty_scale)
            .clamp(0.5, 2.0),
        hq_target_fps: args
            .hq_target_fps
            .unwrap_or(runtime_cfg.hq_target_fps)
            .clamp(12, 120),
        subject_exposure_only: args
            .subject_exposure_only
            .map(Into::into)
            .unwrap_or(runtime_cfg.subject_exposure_only),
        subject_target_height_ratio: args
            .subject_target_height
            .unwrap_or(runtime_cfg.subject_target_height_ratio)
            .clamp(0.20, 0.95),
        subject_target_width_ratio: args
            .subject_target_width
            .unwrap_or(runtime_cfg.subject_target_width_ratio)
            .clamp(0.10, 0.95),
        quality_auto_distance: args
            .quality_auto_distance
            .map(Into::into)
            .unwrap_or(runtime_cfg.quality_auto_distance),
        texture_mip_bias: args
            .texture_mip_bias
            .unwrap_or(runtime_cfg.texture_mip_bias)
            .clamp(-2.0, 4.0),
        stage_as_sub_only: args
            .stage_sub_only
            .map(Into::into)
            .unwrap_or(runtime_cfg.stage_as_sub_only),
        stage_role: args
            .stage_role
            .map(Into::into)
            .unwrap_or(runtime_cfg.stage_role),
        stage_luma_cap: args
            .stage_luma_cap
            .unwrap_or(runtime_cfg.stage_luma_cap)
            .clamp(0.0, 1.0),
        cell_aspect_mode: args
            .cell_aspect_mode
            .map(Into::into)
            .unwrap_or(runtime_cfg.cell_aspect_mode),
        cell_aspect_trim: args
            .cell_aspect_trim
            .unwrap_or(runtime_cfg.cell_aspect_trim)
            .clamp(0.70, 1.30),
        contrast_profile: args
            .contrast_profile
            .map(Into::into)
            .unwrap_or(runtime_cfg.contrast_profile),
        perf_profile: args
            .perf_profile
            .map(Into::into)
            .unwrap_or(runtime_cfg.perf_profile),
        detail_profile: args
            .detail_profile
            .map(Into::into)
            .unwrap_or(runtime_cfg.detail_profile),
        backend: args.backend.map(Into::into).unwrap_or(runtime_cfg.backend),
        exposure_bias: args
            .exposure_bias
            .unwrap_or(runtime_cfg.exposure_bias)
            .clamp(-0.5, 0.8),
        center_lock: args
            .center_lock
            .map(Into::into)
            .unwrap_or(runtime_cfg.center_lock),
        center_lock_mode: args
            .center_lock_mode
            .map(Into::into)
            .unwrap_or(runtime_cfg.center_lock_mode),
        wasd_mode: args
            .wasd_mode
            .map(Into::into)
            .unwrap_or(runtime_cfg.wasd_mode),
        freefly_speed: args
            .freefly_speed
            .unwrap_or(runtime_cfg.freefly_speed)
            .clamp(0.1, 8.0),
        camera_look_speed: args
            .camera_look_speed
            .unwrap_or(runtime_cfg.camera_look_speed)
            .clamp(0.1, 8.0),
        camera_mode: args
            .camera_mode
            .map(Into::into)
            .unwrap_or(runtime_cfg.camera_mode),
        camera_align_preset: args
            .camera_align_preset
            .map(Into::into)
            .unwrap_or(runtime_cfg.camera_align_preset),
        camera_unit_scale: args
            .camera_unit_scale
            .unwrap_or(runtime_cfg.camera_unit_scale)
            .clamp(0.01, 2.0),
        camera_vmd_fps: args
            .camera_vmd_fps
            .unwrap_or(runtime_cfg.camera_vmd_fps)
            .clamp(1.0, 240.0),
        camera_vmd_path: args
            .camera_vmd
            .clone()
            .or(runtime_cfg.camera_vmd_path.clone()),
        camera_focus: args
            .camera_focus
            .map(Into::into)
            .unwrap_or(runtime_cfg.camera_focus),
        material_color: args
            .material_color
            .map(Into::into)
            .unwrap_or(runtime_cfg.material_color),
        texture_sampling: args
            .texture_sampling
            .map(Into::into)
            .unwrap_or(runtime_cfg.texture_sampling),
        texture_v_origin: args
            .texture_v_origin
            .map(Into::into)
            .unwrap_or(runtime_cfg.texture_v_origin),
        texture_sampler: args
            .texture_sampler
            .map(Into::into)
            .unwrap_or(runtime_cfg.texture_sampler),
        clarity_profile: args
            .clarity_profile
            .map(Into::into)
            .unwrap_or(runtime_cfg.clarity_profile),
        ansi_quantization: args
            .ansi_quantization
            .map(Into::into)
            .unwrap_or(runtime_cfg.ansi_quantization),
        model_lift: args
            .model_lift
            .unwrap_or(runtime_cfg.model_lift)
            .clamp(0.02, 0.45),
        edge_accent_strength: args
            .edge_accent_strength
            .unwrap_or(runtime_cfg.edge_accent_strength)
            .clamp(0.0, 1.5),
        bg_suppression: runtime_cfg.bg_suppression.clamp(0.0, 1.0),
        braille_aspect_compensation: runtime_cfg.braille_aspect_compensation,
        stage_level: args.stage_level.unwrap_or(runtime_cfg.stage_level).min(4),
        stage_reactive: runtime_cfg.stage_reactive,
        color_mode: args.color_mode.map(Into::into).or(runtime_cfg.color_mode),
        ascii_force_color: args
            .ascii_force_color
            .map(Into::into)
            .unwrap_or(runtime_cfg.ascii_force_color),
        braille_profile: args
            .braille_profile
            .map(Into::into)
            .unwrap_or(runtime_cfg.braille_profile),
        theme_style: args
            .theme
            .map(Into::into)
            .unwrap_or(runtime_cfg.theme_style),
        audio_reactive: args
            .audio_reactive
            .map(Into::into)
            .unwrap_or(runtime_cfg.audio_reactive),
        cinematic_camera: args
            .cinematic_camera
            .map(Into::into)
            .unwrap_or(runtime_cfg.cinematic_camera),
        reactive_gain: args
            .reactive_gain
            .unwrap_or(runtime_cfg.reactive_gain)
            .clamp(0.0, 1.0),
    }
}

pub(crate) fn resolve_visual_options_for_bench(
    args: &BenchArgs,
    runtime_cfg: &GasciiConfig,
) -> ResolvedVisualOptions {
    ResolvedVisualOptions {
        output_mode: args
            .output_mode
            .map(Into::into)
            .unwrap_or(runtime_cfg.output_mode),
        recover_color_auto: runtime_cfg.recover_color_auto,
        graphics_protocol: args
            .graphics_protocol
            .map(Into::into)
            .unwrap_or(runtime_cfg.graphics_protocol),
        kitty_transport: args
            .kitty_transport
            .map(Into::into)
            .unwrap_or(runtime_cfg.kitty_transport),
        kitty_compression: args
            .kitty_compression
            .map(Into::into)
            .unwrap_or(runtime_cfg.kitty_compression),
        kitty_internal_res: args
            .kitty_internal_res
            .map(Into::into)
            .unwrap_or(runtime_cfg.kitty_internal_res),
        kitty_pipeline_mode: args
            .kitty_pipeline
            .map(Into::into)
            .unwrap_or(runtime_cfg.kitty_pipeline_mode),
        recover_strategy: args
            .recover_strategy
            .map(Into::into)
            .unwrap_or(runtime_cfg.recover_strategy),
        kitty_scale: args
            .kitty_scale
            .unwrap_or(runtime_cfg.kitty_scale)
            .clamp(0.5, 2.0),
        hq_target_fps: args
            .hq_target_fps
            .unwrap_or(runtime_cfg.hq_target_fps)
            .clamp(12, 120),
        subject_exposure_only: args
            .subject_exposure_only
            .map(Into::into)
            .unwrap_or(runtime_cfg.subject_exposure_only),
        subject_target_height_ratio: args
            .subject_target_height
            .unwrap_or(runtime_cfg.subject_target_height_ratio)
            .clamp(0.20, 0.95),
        subject_target_width_ratio: args
            .subject_target_width
            .unwrap_or(runtime_cfg.subject_target_width_ratio)
            .clamp(0.10, 0.95),
        quality_auto_distance: args
            .quality_auto_distance
            .map(Into::into)
            .unwrap_or(runtime_cfg.quality_auto_distance),
        texture_mip_bias: args
            .texture_mip_bias
            .unwrap_or(runtime_cfg.texture_mip_bias)
            .clamp(-2.0, 4.0),
        stage_as_sub_only: args
            .stage_sub_only
            .map(Into::into)
            .unwrap_or(runtime_cfg.stage_as_sub_only),
        stage_role: args
            .stage_role
            .map(Into::into)
            .unwrap_or(runtime_cfg.stage_role),
        stage_luma_cap: args
            .stage_luma_cap
            .unwrap_or(runtime_cfg.stage_luma_cap)
            .clamp(0.0, 1.0),
        cell_aspect_mode: args
            .cell_aspect_mode
            .map(Into::into)
            .unwrap_or(runtime_cfg.cell_aspect_mode),
        cell_aspect_trim: args
            .cell_aspect_trim
            .unwrap_or(runtime_cfg.cell_aspect_trim)
            .clamp(0.70, 1.30),
        contrast_profile: args
            .contrast_profile
            .map(Into::into)
            .unwrap_or(runtime_cfg.contrast_profile),
        perf_profile: args
            .perf_profile
            .map(Into::into)
            .unwrap_or(runtime_cfg.perf_profile),
        detail_profile: args
            .detail_profile
            .map(Into::into)
            .unwrap_or(runtime_cfg.detail_profile),
        backend: args.backend.map(Into::into).unwrap_or(runtime_cfg.backend),
        exposure_bias: args
            .exposure_bias
            .unwrap_or(runtime_cfg.exposure_bias)
            .clamp(-0.5, 0.8),
        center_lock: args
            .center_lock
            .map(Into::into)
            .unwrap_or(runtime_cfg.center_lock),
        center_lock_mode: args
            .center_lock_mode
            .map(Into::into)
            .unwrap_or(runtime_cfg.center_lock_mode),
        wasd_mode: runtime_cfg.wasd_mode,
        freefly_speed: runtime_cfg.freefly_speed.clamp(0.1, 8.0),
        camera_look_speed: runtime_cfg.camera_look_speed.clamp(0.1, 8.0),
        camera_mode: runtime_cfg.camera_mode,
        camera_align_preset: runtime_cfg.camera_align_preset,
        camera_unit_scale: runtime_cfg.camera_unit_scale.clamp(0.01, 2.0),
        camera_vmd_fps: runtime_cfg.camera_vmd_fps.clamp(1.0, 240.0),
        camera_vmd_path: runtime_cfg.camera_vmd_path.clone(),
        camera_focus: args
            .camera_focus
            .map(Into::into)
            .unwrap_or(runtime_cfg.camera_focus),
        material_color: args
            .material_color
            .map(Into::into)
            .unwrap_or(runtime_cfg.material_color),
        texture_sampling: args
            .texture_sampling
            .map(Into::into)
            .unwrap_or(runtime_cfg.texture_sampling),
        texture_v_origin: args
            .texture_v_origin
            .map(Into::into)
            .unwrap_or(runtime_cfg.texture_v_origin),
        texture_sampler: args
            .texture_sampler
            .map(Into::into)
            .unwrap_or(runtime_cfg.texture_sampler),
        clarity_profile: args
            .clarity_profile
            .map(Into::into)
            .unwrap_or(runtime_cfg.clarity_profile),
        ansi_quantization: args
            .ansi_quantization
            .map(Into::into)
            .unwrap_or(runtime_cfg.ansi_quantization),
        model_lift: args
            .model_lift
            .unwrap_or(runtime_cfg.model_lift)
            .clamp(0.02, 0.45),
        edge_accent_strength: args
            .edge_accent_strength
            .unwrap_or(runtime_cfg.edge_accent_strength)
            .clamp(0.0, 1.5),
        bg_suppression: runtime_cfg.bg_suppression.clamp(0.0, 1.0),
        braille_aspect_compensation: runtime_cfg.braille_aspect_compensation,
        stage_level: args.stage_level.unwrap_or(runtime_cfg.stage_level).min(4),
        stage_reactive: runtime_cfg.stage_reactive,
        color_mode: args.color_mode.map(Into::into).or(runtime_cfg.color_mode),
        ascii_force_color: args
            .ascii_force_color
            .map(Into::into)
            .unwrap_or(runtime_cfg.ascii_force_color),
        braille_profile: args
            .braille_profile
            .map(Into::into)
            .unwrap_or(runtime_cfg.braille_profile),
        theme_style: args
            .theme
            .map(Into::into)
            .unwrap_or(runtime_cfg.theme_style),
        audio_reactive: args
            .audio_reactive
            .map(Into::into)
            .unwrap_or(runtime_cfg.audio_reactive),
        cinematic_camera: args
            .cinematic_camera
            .map(Into::into)
            .unwrap_or(runtime_cfg.cinematic_camera),
        reactive_gain: args
            .reactive_gain
            .unwrap_or(runtime_cfg.reactive_gain)
            .clamp(0.0, 1.0),
    }
}

pub(crate) fn resolve_sync_options_for_start(
    args: &StartArgs,
    runtime_cfg: &GasciiConfig,
) -> ResolvedSyncOptions {
    ResolvedSyncOptions {
        sync_offset_ms: args
            .sync_offset_ms
            .unwrap_or(runtime_cfg.sync_offset_ms)
            .clamp(-SYNC_OFFSET_LIMIT_MS, SYNC_OFFSET_LIMIT_MS),
        sync_speed_mode: args
            .sync_speed_mode
            .map(Into::into)
            .unwrap_or(runtime_cfg.sync_speed_mode),
        sync_policy: args
            .sync_policy
            .map(Into::into)
            .unwrap_or(runtime_cfg.sync_policy),
        sync_hard_snap_ms: args
            .sync_hard_snap_ms
            .unwrap_or(runtime_cfg.sync_hard_snap_ms)
            .clamp(10, 2_000),
        sync_kp: args.sync_kp.unwrap_or(runtime_cfg.sync_kp).clamp(0.01, 1.0),
    }
}

pub(crate) fn resolve_sync_options_for_run(
    args: &RunArgs,
    runtime_cfg: &GasciiConfig,
    profile: Option<&SyncProfileEntry>,
) -> ResolvedSyncOptions {
    let profile_speed_mode = profile.and_then(|entry| entry.sync_speed_mode);
    let profile_hard_snap = profile.and_then(|entry| entry.sync_hard_snap_ms);
    let profile_kp = profile.and_then(|entry| entry.sync_kp);
    let profile_offset = profile.map(|entry| entry.sync_offset_ms);
    ResolvedSyncOptions {
        sync_offset_ms: args
            .sync_offset_ms
            .or(profile_offset)
            .unwrap_or(runtime_cfg.sync_offset_ms)
            .clamp(-SYNC_OFFSET_LIMIT_MS, SYNC_OFFSET_LIMIT_MS),
        sync_speed_mode: args
            .sync_speed_mode
            .map(Into::into)
            .or(profile_speed_mode)
            .unwrap_or(runtime_cfg.sync_speed_mode),
        sync_policy: args
            .sync_policy
            .map(Into::into)
            .unwrap_or(runtime_cfg.sync_policy),
        sync_hard_snap_ms: args
            .sync_hard_snap_ms
            .or(profile_hard_snap)
            .unwrap_or(runtime_cfg.sync_hard_snap_ms)
            .clamp(10, 2_000),
        sync_kp: args
            .sync_kp
            .or(profile_kp)
            .unwrap_or(runtime_cfg.sync_kp)
            .clamp(0.01, 1.0),
    }
}

pub(crate) fn resolve_sync_profile_options_for_start(
    args: &StartArgs,
    runtime_cfg: &GasciiConfig,
) -> ResolvedSyncProfileOptions {
    ResolvedSyncProfileOptions {
        mode: args
            .sync_profile_mode
            .map(Into::into)
            .unwrap_or(runtime_cfg.sync_profile_mode),
        profile_dir: args
            .sync_profile_dir
            .clone()
            .unwrap_or_else(|| runtime_cfg.sync_profile_dir.clone()),
        key_override: args
            .sync_profile_key
            .clone()
            .filter(|value| !value.is_empty()),
    }
}

pub(crate) fn resolve_sync_profile_options_for_run(
    args: &RunArgs,
    runtime_cfg: &GasciiConfig,
) -> ResolvedSyncProfileOptions {
    ResolvedSyncProfileOptions {
        mode: args
            .sync_profile_mode
            .map(Into::into)
            .unwrap_or(runtime_cfg.sync_profile_mode),
        profile_dir: args
            .sync_profile_dir
            .clone()
            .unwrap_or_else(|| runtime_cfg.sync_profile_dir.clone()),
        key_override: args
            .sync_profile_key
            .clone()
            .filter(|value| !value.is_empty()),
    }
}

pub(crate) fn resolve_sync_profile_for_assets(
    options: &ResolvedSyncProfileOptions,
    scene_kind: RunSceneArg,
    glb_path: Option<&Path>,
    music_path: Option<&Path>,
    camera_path: Option<&Path>,
) -> (Option<RuntimeSyncProfileContext>, Option<SyncProfileEntry>) {
    if matches!(options.mode, SyncProfileMode::Off) {
        return (None, None);
    }
    let scene_kind = match scene_kind {
        RunSceneArg::Cube => "cube",
        RunSceneArg::Obj => "obj",
        RunSceneArg::Glb => "glb",
        RunSceneArg::Pmx => "pmx",
    };
    let key = options
        .key_override
        .clone()
        .unwrap_or_else(|| build_profile_key(scene_kind, glb_path, music_path, camera_path));
    let store_path = default_profile_store_path(&options.profile_dir);
    let profile = match SyncProfileStore::load(&store_path) {
        Ok(store) => store.get(&key).cloned(),
        Err(err) => {
            eprintln!(
                "warning: failed to load sync profiles {}: {err}",
                store_path.display()
            );
            None
        }
    };
    (
        Some(RuntimeSyncProfileContext {
            mode: options.mode,
            store_path,
            key,
            hit: profile.is_some(),
        }),
        profile,
    )
}

pub(crate) fn default_color_mode_for_mode(mode: RenderMode) -> ColorMode {
    match mode {
        RenderMode::Braille => ColorMode::Ansi,
        RenderMode::Ascii => ColorMode::Mono,
    }
}

pub(crate) fn resolve_effective_color_mode(
    mode: RenderMode,
    requested: ColorMode,
    ascii_force_color: bool,
) -> ColorMode {
    if matches!(mode, RenderMode::Ascii) && ascii_force_color {
        ColorMode::Ansi
    } else {
        requested
    }
}

pub(crate) fn resolve_effective_camera_mode(mode: CameraMode, has_vmd_source: bool) -> CameraMode {
    if has_vmd_source && matches!(mode, CameraMode::Off) {
        CameraMode::Vmd
    } else {
        mode
    }
}

pub(crate) fn color_path_label(
    color_mode: ColorMode,
    quantization: AnsiQuantization,
) -> &'static str {
    match color_mode {
        ColorMode::Mono => "mono",
        ColorMode::Ansi => match quantization {
            AnsiQuantization::Q216 => "ansi-q216",
            AnsiQuantization::Off => "ansi-truecolor",
        },
    }
}
