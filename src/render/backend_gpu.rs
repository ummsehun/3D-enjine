use glam::Mat4;

use crate::renderer::{Camera, FrameBuffers, GlyphRamp, RenderScratch, RenderStats};
use crate::scene::{RenderConfig, SceneCpu};

#[cfg(feature = "gpu")]
use crate::render::gpu::{GpuError, GpuRenderer};

#[derive(Default)]
pub struct GpuRendererState {
    #[cfg(feature = "gpu")]
    renderer: Option<GpuRenderer>,
}

#[derive(Debug)]
pub enum GpuBackendError {
    #[cfg(feature = "gpu")]
    Gpu(GpuError),
    NotImplemented,
    Unsupported,
}

#[cfg(feature = "gpu")]
impl From<GpuError> for GpuBackendError {
    fn from(e: GpuError) -> Self {
        Self::Gpu(e)
    }
}

#[cfg(feature = "gpu")]
impl GpuRendererState {
    pub fn renderer_mut(&mut self) -> Result<&mut GpuRenderer, GpuError> {
        if self.renderer.is_none() {
            self.renderer = Some(GpuRenderer::new()?);
        }
        Ok(self.renderer.as_mut().expect("renderer initialized above"))
    }
}

#[cfg(feature = "gpu")]
pub fn render_frame_gpu(
    renderer_state: &mut GpuRendererState,
    frame: &mut FrameBuffers,
    config: &RenderConfig,
    scene: &SceneCpu,
    global_matrices: &[Mat4],
    skin_matrices: &[Vec<Mat4>],
    instance_morph_weights: &[Vec<f32>],
    glyph_ramp: &GlyphRamp,
    scratch: &mut RenderScratch,
    camera: Camera,
    model_rotation_y: f32,
) -> Result<RenderStats, GpuBackendError> {
    crate::render::gpu::render_frame_gpu(
        renderer_state,
        frame,
        config,
        scene,
        global_matrices,
        skin_matrices,
        instance_morph_weights,
        glyph_ramp,
        scratch,
        camera,
        model_rotation_y,
    )
    .map_err(GpuBackendError::from)
}

#[cfg(not(feature = "gpu"))]
pub fn render_frame_gpu(
    _renderer_state: &mut GpuRendererState,
    _frame: &mut FrameBuffers,
    _config: &RenderConfig,
    _scene: &SceneCpu,
    _global_matrices: &[Mat4],
    _skin_matrices: &[Vec<Mat4>],
    _instance_morph_weights: &[Vec<f32>],
    _glyph_ramp: &GlyphRamp,
    _scratch: &mut RenderScratch,
    _camera: Camera,
    _model_rotation_y: f32,
) -> Result<RenderStats, GpuBackendError> {
    Err(GpuBackendError::Unsupported)
}
