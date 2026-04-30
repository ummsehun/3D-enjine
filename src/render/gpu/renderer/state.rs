use std::collections::HashMap;

use super::{
    super::{
        device::{GpuContext, GpuError},
        pipeline::GpuPipeline,
        resources::{GpuMesh, GpuTexture},
        texture::GpuTexture as RenderTarget,
    },
    cache::{SceneSignature, TextureBindingKey},
};

pub struct GpuRenderer {
    pub(crate) ctx: GpuContext,
    pub(crate) pipeline: Option<GpuPipeline>,
    pub(crate) mesh_cache: HashMap<usize, GpuMesh>,
    pub(crate) morph_mesh_cache: HashMap<usize, GpuMesh>,
    pub(crate) texture_cache: HashMap<usize, GpuTexture>,
    pub(crate) texture_bind_groups: HashMap<TextureBindingKey, wgpu::BindGroup>,
    pub(crate) default_texture: Option<GpuTexture>,
    pub(crate) default_bind_group: Option<wgpu::BindGroup>,
    pub(crate) cached_render_target: Option<RenderTarget>,
    pub(crate) cached_render_target_size: Option<(u32, u32)>,
    pub(crate) cached_scene_sig: Option<SceneSignature>,
}

impl GpuRenderer {
    pub fn new() -> Result<Self, GpuError> {
        let ctx = GpuContext::new()?;
        Ok(Self {
            ctx,
            pipeline: None,
            mesh_cache: HashMap::new(),
            morph_mesh_cache: HashMap::new(),
            texture_cache: HashMap::new(),
            texture_bind_groups: HashMap::new(),
            default_texture: None,
            default_bind_group: None,
            cached_render_target: None,
            cached_render_target_size: None,
            cached_scene_sig: None,
        })
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

    pub(super) fn ensure_pipeline(&mut self) -> Result<(), GpuError> {
        if self.pipeline.is_none() {
            self.pipeline = Some(GpuPipeline::new(
                &self.ctx,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )?);
        }

        if self.default_texture.is_none() {
            let default_tex = GpuTexture::placeholder(&self.ctx);
            self.default_bind_group = Some(self.create_texture_bind_group(&default_tex)?);
            self.default_texture = Some(default_tex);
        }

        Ok(())
    }
}
