use glam::Mat4;

use crate::renderer::{render_frame, Camera, FrameBuffers, GlyphRamp, RenderScratch, RenderStats};
use crate::scene::{RenderConfig, SceneCpu};

pub fn render_frame_cpu(
    frame: &mut FrameBuffers,
    config: &RenderConfig,
    scene: &SceneCpu,
    global_matrices: &[Mat4],
    skin_matrices: &[Vec<Mat4>],
    instance_morph_weights: &[Vec<f32>],
    material_morph_weights: &[f32],
    glyph_ramp: &GlyphRamp,
    scratch: &mut RenderScratch,
    camera: Camera,
    model_rotation_y: f32,
) -> RenderStats {
    render_frame(
        frame,
        config,
        scene,
        global_matrices,
        skin_matrices,
        instance_morph_weights,
        material_morph_weights,
        glyph_ramp,
        scratch,
        camera,
        model_rotation_y,
    )
}
