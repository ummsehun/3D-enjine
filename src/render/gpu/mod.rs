//! GPU rendering backend using wgpu (Metal on macOS).

mod device;
mod pipeline;
mod texture;

pub use device::{AdapterInfo, GpuContext, GpuError};
pub use pipeline::{GpuPipeline, Uniforms, Vertex};
pub use texture::{GpuTexture, TextureSize};

use std::sync::Mutex;

use glam::Mat4;

use crate::renderer::{Camera, FrameBuffers, GlyphRamp, PixelFrame, RenderScratch, RenderStats};
use crate::scene::{RenderConfig, SceneCpu};

pub struct GpuRenderer {
    ctx: GpuContext,
}

impl GpuRenderer {
    pub fn new() -> Result<Self, GpuError> {
        let ctx = GpuContext::new()?;
        Ok(Self { ctx })
    }

    pub fn is_available() -> bool {
        std::thread::spawn(|| {
            futures::executor::block_on(async {
                let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                    backends: wgpu::Backends::METAL,
                    ..Default::default()
                });
                instance
                    .request_adapter(&wgpu::RequestAdapterOptions {
                        power_preference: wgpu::PowerPreference::HighPerformance,
                        ..Default::default()
                    })
                    .await
                    .is_some()
            })
        })
        .join()
        .unwrap_or(false)
    }

    pub fn render(
        &self,
        _config: &RenderConfig,
        _scene: &SceneCpu,
        _global_matrices: &[Mat4],
        _skin_matrices: &[Vec<Mat4>],
        _instance_morph_weights: &[Vec<f32>],
        _camera: Camera,
        _model_rotation_y: f32,
        width: u32,
        height: u32,
    ) -> Result<PixelFrame, GpuError> {
        let texture = GpuTexture::new(&self.ctx, TextureSize::new(width, height))?;
        let pipeline = GpuPipeline::new(&self.ctx, wgpu::TextureFormat::Rgba8UnormSrgb)?;

        let uniforms = Uniforms::default();
        pipeline.update_uniforms(&self.ctx.queue, &uniforms);

        let mut encoder = self
            .ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        {
            let mut render_pass = texture.begin_render_pass(&mut encoder);
            render_pass.set_pipeline(&pipeline.render_pipeline);
            render_pass.set_bind_group(0, &pipeline.bind_group, &[]);
        }

        self.ctx
            .queue
            .submit(std::iter::once(encoder.finish()));

        let rgba_data = texture.readback(&self.ctx.device, &self.ctx.queue)?;

        let mut pixel_frame = PixelFrame::new(width, height);
        pixel_frame.rgba8.copy_from_slice(&rgba_data);

        Ok(pixel_frame)
    }
}

static GPU_RENDERER: Mutex<Option<GpuRenderer>> = Mutex::new(None);

#[cfg(feature = "gpu")]
pub fn render_frame_gpu(
    frame: &mut FrameBuffers,
    config: &RenderConfig,
    scene: &SceneCpu,
    global_matrices: &[Mat4],
    skin_matrices: &[Vec<Mat4>],
    instance_morph_weights: &[Vec<f32>],
    glyph_ramp: &GlyphRamp,
    _scratch: &mut RenderScratch,
    camera: Camera,
    model_rotation_y: f32,
) -> Result<RenderStats, GpuError> {
    let width = u32::from(frame.width).max(1);
    let height = u32::from(frame.height).max(1);

    let pixel_frame = {
        let mut guard = GPU_RENDERER.lock().unwrap();
        let renderer = guard.get_or_insert_with(|| GpuRenderer::new().unwrap());
        renderer.render(
            config,
            scene,
            global_matrices,
            skin_matrices,
            instance_morph_weights,
            camera,
            model_rotation_y,
            width,
            height,
        )?
    };

    convert_pixel_frame_to_ascii(&pixel_frame, frame, config, glyph_ramp);

    Ok(RenderStats::default())
}

#[cfg(not(feature = "gpu"))]
pub fn render_frame_gpu(
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
) -> Result<RenderStats, GpuError> {
    Err(GpuError::NotImplemented)
}

#[cfg(feature = "gpu")]
fn convert_pixel_frame_to_ascii(
    pixel_frame: &PixelFrame,
    frame: &mut FrameBuffers,
    _config: &RenderConfig,
    _glyph_ramp: &GlyphRamp,
) {
    let width = usize::from(frame.width);
    let height = usize::from(frame.height);
    let px_width = pixel_frame.width_px as usize;
    let cell_width = (px_width / width).max(1);
    let cell_height = (pixel_frame.height_px as usize / height).max(1);

    for y in 0..height {
        for x in 0..width {
            let px_x = x * cell_width;
            let px_y = y * cell_height;
            let idx = y * width + x;
            let px_idx = (px_y.min(pixel_frame.height_px as usize - 1)) * px_width
                + px_x.min(pixel_frame.width_px as usize - 1);

            let r = pixel_frame.rgba8[px_idx * 4];
            let g = pixel_frame.rgba8[px_idx * 4 + 1];
            let b = pixel_frame.rgba8[px_idx * 4 + 2];
            let luminance = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;

            frame.glyphs[idx] = if luminance > 200.0 {
                '█'
            } else if luminance > 150.0 {
                '▓'
            } else if luminance > 100.0 {
                '▒'
            } else if luminance > 50.0 {
                '░'
            } else {
                ' '
            };
            frame.fg_rgb[idx] = [r, g, b];
        }
    }
    frame.has_color = true;
}

#[derive(Debug)]
pub enum GpuBackendError {
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