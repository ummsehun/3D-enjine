use super::*;
use crate::engine::pipeline::FramePipeline;
use crate::scene::{
    MeshInstance, MeshLayer, MorphTargetCpu, Node, RenderBackend, RenderConfig, RenderMode,
    SceneCpu, cube_scene,
};
use glam::{Quat, Vec3};

fn build_mixed_morph_scene() -> SceneCpu {
    let mut scene = cube_scene();
    scene.meshes[0].morph_targets = vec![MorphTargetCpu {
        name: Some("stretch".to_owned()),
        position_deltas: scene.meshes[0]
            .positions
            .iter()
            .map(|p| Vec3::new(0.0, if p.y > 0.0 { 0.45 } else { 0.0 }, 0.0))
            .collect(),
        normal_deltas: vec![Vec3::ZERO; scene.meshes[0].positions.len()],
        uv0_deltas: None,
        uv1_deltas: None,
    }];
    scene.nodes = vec![
        Node {
            name: Some("left".to_owned()),
            name_en: None,
            parent: None,
            children: Vec::new(),
            base_translation: Vec3::new(-0.55, 0.0, 0.0),
            base_rotation: Quat::IDENTITY,
            base_scale: Vec3::ONE,
        },
        Node {
            name: Some("right".to_owned()),
            name_en: None,
            parent: None,
            children: Vec::new(),
            base_translation: Vec3::new(0.55, 0.0, 0.0),
            base_rotation: Quat::IDENTITY,
            base_scale: Vec3::ONE,
        },
    ];
    scene.mesh_instances = vec![
        MeshInstance {
            mesh_index: 0,
            node_index: 0,
            skin_index: None,
            default_morph_weights: Vec::new(),
            layer: MeshLayer::Subject,
        },
        MeshInstance {
            mesh_index: 0,
            node_index: 1,
            skin_index: None,
            default_morph_weights: vec![1.0],
            layer: MeshLayer::Subject,
        },
    ];
    scene.root_center_node = Some(0);
    scene
}

#[test]
fn mixed_morph_instances_populate_separate_gpu_caches() {
    if !GpuRenderer::is_available() {
        eprintln!("gpu unavailable; skipping cache split capture");
        return;
    }

    let scene = build_mixed_morph_scene();
    let mut renderer = GpuRenderer::new().expect("gpu renderer");
    let mut config = RenderConfig::default();
    config.backend = RenderBackend::Gpu;
    config.mode = RenderMode::Ascii;

    let mut pipeline = FramePipeline::new(&scene);
    pipeline.prepare_frame(&scene, 0.0, None, None, 0.0);

    let _ = renderer
        .render(
            &config,
            &scene,
            pipeline.globals(),
            pipeline.skin_matrices(),
            pipeline.morph_weights_by_instance(),
            Camera::default(),
            0.0,
            88,
            50,
        )
        .expect("gpu render");

    assert_eq!(renderer.mesh_cache.len(), 1);
    assert_eq!(renderer.morph_mesh_cache.len(), 1);
    assert!(renderer.mesh_cache.contains_key(&0));
    assert!(renderer.morph_mesh_cache.contains_key(&0));
}
