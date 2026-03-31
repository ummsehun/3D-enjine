use crate::render::backend::GpuRendererState;
use crate::renderer::{Camera, FrameBuffers, GlyphRamp, PixelFrame, RenderScratch, RenderStats};
use crate::scene::RenderConfig;

use super::super::stats::compute_gpu_render_stats;
#[cfg(feature = "gpu")]
pub fn render_frame_gpu(
    renderer_state: &mut GpuRendererState,
    frame: &mut FrameBuffers,
    config: &RenderConfig,
    scene: &crate::scene::SceneCpu,
    global_matrices: &[glam::Mat4],
    skin_matrices: &[Vec<glam::Mat4>],
    instance_morph_weights: &[Vec<f32>],
    glyph_ramp: &GlyphRamp,
    _scratch: &mut RenderScratch,
    camera: Camera,
    model_rotation_y: f32,
) -> Result<RenderStats, crate::render::gpu::GpuError> {
    let width = u32::from(frame.width).max(1);
    let height = u32::from(frame.height).max(1);

    let pixel_frame = {
        let renderer = renderer_state.renderer_mut()?;
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

    Ok(compute_gpu_render_stats(
        &pixel_frame,
        config,
        scene,
        global_matrices,
        skin_matrices,
        instance_morph_weights,
        camera,
        model_rotation_y,
    ))
}

#[cfg(not(feature = "gpu"))]
pub fn render_frame_gpu(
    _renderer_state: &mut GpuRendererState,
    _frame: &mut FrameBuffers,
    _config: &RenderConfig,
    _scene: &crate::scene::SceneCpu,
    _global_matrices: &[glam::Mat4],
    _skin_matrices: &[Vec<glam::Mat4>],
    _instance_morph_weights: &[Vec<f32>],
    _glyph_ramp: &GlyphRamp,
    _scratch: &mut RenderScratch,
    _camera: Camera,
    _model_rotation_y: f32,
) -> Result<RenderStats, crate::render::gpu::GpuError> {
    Err(crate::render::gpu::GpuError::NotImplemented)
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
