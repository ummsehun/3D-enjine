use glam::Mat4;

use crate::engine::pmx_rig::{
    apply_append_bone_transforms, apply_pmx_bone_axis_constraints, compute_bone_position,
    solve_ik_chain_ccd,
};
use crate::{animation::ChannelTarget, scene::NodePose};
use crate::{
    animation::{
        compute_global_matrices_in_place, compute_skin_matrices_in_place, reset_poses_from_nodes,
    },
    scene::SceneCpu,
};

pub(crate) trait PhysicsStepper {
    fn step_physics(
        &mut self,
        scene: &SceneCpu,
        poses: &mut [NodePose],
        pre_physics_globals: &[Mat4],
        dt: f32,
    );
}

pub struct FramePipeline {
    poses: Vec<NodePose>,
    node_morph_weights: Vec<Vec<f32>>,
    instance_morph_weights: Vec<Vec<f32>>,
    material_morph_weights: Vec<f32>,
    globals: Vec<Mat4>,
    globals_visited: Vec<bool>,
    skin_matrices: Vec<Vec<Mat4>>,
    text_buffer: String,
}

impl FramePipeline {
    pub fn new(scene: &SceneCpu) -> Self {
        Self {
            poses: Vec::with_capacity(scene.nodes.len()),
            node_morph_weights: Vec::with_capacity(scene.nodes.len()),
            instance_morph_weights: Vec::with_capacity(scene.mesh_instances.len()),
            material_morph_weights: vec![0.0; scene.material_morphs.len()],
            globals: Vec::with_capacity(scene.nodes.len()),
            globals_visited: Vec::with_capacity(scene.nodes.len()),
            skin_matrices: Vec::with_capacity(scene.skins.len()),
            text_buffer: String::new(),
        }
    }

    pub(crate) fn prepare_frame(
        &mut self,
        scene: &SceneCpu,
        elapsed_seconds: f32,
        anim_index: Option<usize>,
        mut physics_state: Option<&mut dyn PhysicsStepper>,
        physics_dt: f32,
    ) {
        reset_poses_from_nodes(&scene.nodes, &mut self.poses);
        seed_node_morph_weights(scene, &mut self.node_morph_weights);
        self.material_morph_weights.fill(0.0);
        let mut primary_normalized_time = None;
        if let Some(index) = anim_index {
            if let Some(clip) = scene.animations.get(index) {
                clip.sample_into_with_morph(
                    elapsed_seconds,
                    &mut self.poses,
                    &mut self.node_morph_weights,
                    &mut self.material_morph_weights,
                );
                primary_normalized_time =
                    Some(normalized_clip_time(elapsed_seconds, clip.duration));
            }
        }
        for (index, clip) in scene.animations.iter().enumerate() {
            if Some(index) == anim_index || !is_morph_only_clip(clip) {
                continue;
            }
            let sample_time = match primary_normalized_time {
                Some(primary_t) if clip.duration > f32::EPSILON => primary_t * clip.duration,
                _ => elapsed_seconds,
            };
            clip.sample_into_with_morph(
                sample_time,
                &mut self.poses,
                &mut self.node_morph_weights,
                &mut self.material_morph_weights,
            );
        }

        apply_pmx_pose_stack(scene, &mut self.poses, physics_state.is_some());

        compute_global_matrices_in_place(
            &scene.nodes,
            &self.poses,
            &mut self.globals,
            &mut self.globals_visited,
        );
        if let Some(physics) = physics_state.as_deref_mut() {
            physics.step_physics(scene, &mut self.poses, &self.globals, physics_dt);
            apply_pmx_pose_stack(scene, &mut self.poses, true);
            compute_global_matrices_in_place(
                &scene.nodes,
                &self.poses,
                &mut self.globals,
                &mut self.globals_visited,
            );
        }
        compute_skin_matrices_in_place(scene, &self.globals, &mut self.skin_matrices);
        resolve_instance_morph_weights(
            scene,
            &self.node_morph_weights,
            &mut self.instance_morph_weights,
        );
    }

    pub fn globals(&self) -> &[Mat4] {
        &self.globals
    }

    pub fn skin_matrices(&self) -> &[Vec<Mat4>] {
        &self.skin_matrices
    }

    pub fn morph_weights_by_instance(&self) -> &[Vec<f32>] {
        &self.instance_morph_weights
    }

    pub fn material_morph_weights(&self) -> &[f32] {
        &self.material_morph_weights
    }

    pub fn text_buffer_mut(&mut self) -> &mut String {
        &mut self.text_buffer
    }
}

fn is_morph_only_clip(clip: &crate::animation::AnimationClip) -> bool {
    !clip.channels.is_empty()
        && clip
            .channels
            .iter()
            .all(|channel| channel.target == ChannelTarget::MorphWeights)
}

fn normalized_clip_time(elapsed_seconds: f32, duration: f32) -> f32 {
    if duration <= f32::EPSILON {
        return 0.0;
    }
    elapsed_seconds.rem_euclid(duration) / duration
}

fn solve_pmx_ik_chains(scene: &SceneCpu, poses: &mut [NodePose], physics_active: bool) {
    let Some(rig_meta) = &scene.pmx_rig_meta else {
        return;
    };

    for chain in &rig_meta.ik_chains {
        if physics_active && ik_chain_conflicts_with_physics(scene, chain) {
            continue;
        }
        let target_pos = compute_bone_position(chain.controller_bone_index, &scene.nodes, poses);
        solve_ik_chain_ccd(chain, &scene.nodes, poses, target_pos);
    }
}

fn apply_pmx_pose_stack(scene: &SceneCpu, poses: &mut [NodePose], physics_active: bool) {
    let Some(rig_meta) = &scene.pmx_rig_meta else {
        return;
    };

    apply_append_bone_transforms(rig_meta, poses);
    solve_pmx_ik_chains(scene, poses, physics_active);
    apply_pmx_bone_axis_constraints(rig_meta, poses);
}

fn ik_chain_conflicts_with_physics(
    scene: &SceneCpu,
    chain: &crate::engine::pmx_rig::IKChain,
) -> bool {
    let Some(physics_meta) = scene.pmx_physics_meta.as_ref() else {
        return false;
    };

    chain.links.iter().any(|link| {
        physics_meta.rigid_bodies.iter().any(|rigid| {
            rigid.bone_index >= 0
                && rigid.bone_index as usize == link.bone_index
                && !matches!(
                    rigid.calc_method,
                    crate::engine::pmx_rig::PmxRigidCalcMethod::Static
                )
        })
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::animation::{AnimationChannel, AnimationClip, ChannelValues, Interpolation};
    use crate::assets::vmd_motion::parse_vmd_motion;
    use crate::loader;
    use crate::runtime::state::PmxPhysicsState;
    use crate::scene::{MeshCpu, MeshInstance, MeshLayer, MorphTargetCpu, Node, SceneCpu};
    use glam::{Quat, Vec3};

    #[test]
    fn normalized_clip_time_wraps() {
        let t = normalized_clip_time(3.5, 2.0);
        assert!((t - 0.75).abs() < 1e-6);
    }

    #[test]
    fn morph_only_clip_detection() {
        let clip = AnimationClip {
            name: Some("facial".to_owned()),
            channels: vec![AnimationChannel {
                node_index: 0,
                target: ChannelTarget::MorphWeights,
                interpolation: Interpolation::Linear,
                inputs: vec![0.0, 1.0],
                outputs: ChannelValues::MorphWeights {
                    values: vec![0.0, 1.0],
                    weights_per_key: 1,
                },
            }],
            duration: 1.0,
            looping: true,
        };
        assert!(is_morph_only_clip(&clip));
    }

    #[test]
    fn prepare_frame_applies_secondary_morph_clip_with_primary_timeline() {
        let node = Node {
            name: Some("root".to_owned()),
            name_en: None,
            parent: None,
            children: Vec::new(),
            base_translation: Vec3::ZERO,
            base_rotation: Quat::IDENTITY,
            base_scale: Vec3::ONE,
        };
        let mesh = MeshCpu {
            positions: vec![Vec3::ZERO],
            normals: vec![Vec3::Y],
            uv0: None,
            uv1: None,
            colors_rgba: None,
            material_index: None,
            indices: vec![[0, 0, 0]],
            joints4: None,
            weights4: None,
            sdef_vertices: None,
            morph_targets: vec![MorphTargetCpu {
                name: Some("smile".to_owned()),
                position_deltas: vec![Vec3::new(0.0, 1.0, 0.0)],
                normal_deltas: vec![Vec3::ZERO],
                uv0_deltas: None,
                uv1_deltas: None,
            }],
        };
        let primary = AnimationClip {
            name: Some("bone".to_owned()),
            channels: vec![AnimationChannel {
                node_index: 0,
                target: ChannelTarget::Translation,
                interpolation: Interpolation::Linear,
                inputs: vec![0.0, 2.0],
                outputs: ChannelValues::Vec3(vec![Vec3::ZERO, Vec3::new(0.0, 2.0, 0.0)]),
            }],
            duration: 2.0,
            looping: true,
        };
        let facial = AnimationClip {
            name: Some("facial".to_owned()),
            channels: vec![AnimationChannel {
                node_index: 0,
                target: ChannelTarget::MorphWeights,
                interpolation: Interpolation::Linear,
                inputs: vec![0.0, 1.0],
                outputs: ChannelValues::MorphWeights {
                    values: vec![0.0, 1.0],
                    weights_per_key: 1,
                },
            }],
            duration: 1.0,
            looping: true,
        };
        let scene = SceneCpu {
            meshes: vec![mesh],
            materials: Vec::new(),
            textures: Vec::new(),
            skins: Vec::new(),
            nodes: vec![node],
            mesh_instances: vec![MeshInstance {
                mesh_index: 0,
                node_index: 0,
                skin_index: None,
                default_morph_weights: vec![0.0],
                layer: MeshLayer::Subject,
            }],
            animations: vec![primary, facial],
            root_center_node: Some(0),
            pmx_rig_meta: None,
            pmx_physics_meta: None,
            material_morphs: Vec::new(),
        };

        let mut pipeline = FramePipeline::new(&scene);
        pipeline.prepare_frame(&scene, 1.0, Some(0), None, 0.0);
        let applied = pipeline.morph_weights_by_instance()[0][0];
        assert!((applied - 0.5).abs() < 1e-5);
    }

    #[test]
    fn prepare_frame_applies_pmx_physics_before_skinning() {
        let scene = SceneCpu {
            meshes: Vec::new(),
            materials: Vec::new(),
            textures: Vec::new(),
            skins: Vec::new(),
            nodes: vec![Node {
                name: Some("root".to_owned()),
                name_en: None,
                parent: None,
                children: Vec::new(),
                base_translation: Vec3::ZERO,
                base_rotation: Quat::IDENTITY,
                base_scale: Vec3::ONE,
            }],
            mesh_instances: Vec::new(),
            animations: Vec::new(),
            root_center_node: Some(0),
            pmx_rig_meta: None,
            pmx_physics_meta: Some(crate::engine::pmx_rig::PmxPhysicsMeta {
                rigid_bodies: vec![crate::engine::pmx_rig::PmxRigidBodyCpu {
                    name: "rb".to_owned(),
                    name_en: "rb".to_owned(),
                    bone_index: 0,
                    group: 0,
                    un_collision_group_flag: 0,
                    form: crate::engine::pmx_rig::PmxRigidShape::Sphere,
                    size: Vec3::splat(0.1),
                    position: Vec3::new(0.0, 1.0, 0.0),
                    rotation: Vec3::ZERO,
                    mass: 1.0,
                    move_resist: 0.0,
                    rotation_resist: 0.0,
                    repulsion: 0.0,
                    friction: 0.0,
                    calc_method: crate::engine::pmx_rig::PmxRigidCalcMethod::Dynamic,
                }],
                joints: Vec::new(),
            }),
            material_morphs: Vec::new(),
        };

        let mut pipeline = FramePipeline::new(&scene);
        let mut physics = PmxPhysicsState::from_scene(
            &scene,
            crate::runtime::state::RuntimePmxSettings::default(),
        )
        .expect("physics state");
        pipeline.prepare_frame(&scene, 0.0, None, Some(&mut physics), 0.2);

        let root_y = pipeline.globals()[0].transform_point3(Vec3::ZERO).y;
        assert!(root_y < 1.0);
    }

    #[test]
    fn prepare_frame_applies_ik_using_controller_target() {
        let scene = SceneCpu {
            meshes: Vec::new(),
            materials: Vec::new(),
            textures: Vec::new(),
            skins: Vec::new(),
            nodes: vec![
                Node {
                    name: Some("controller".to_owned()),
                    name_en: None,
                    parent: None,
                    children: Vec::new(),
                    base_translation: Vec3::new(0.0, 1.0, 0.0),
                    base_rotation: Quat::IDENTITY,
                    base_scale: Vec3::ONE,
                },
                Node {
                    name: Some("joint".to_owned()),
                    name_en: None,
                    parent: None,
                    children: vec![2],
                    base_translation: Vec3::ZERO,
                    base_rotation: Quat::IDENTITY,
                    base_scale: Vec3::ONE,
                },
                Node {
                    name: Some("effector".to_owned()),
                    name_en: None,
                    parent: Some(1),
                    children: Vec::new(),
                    base_translation: Vec3::new(1.0, 0.0, 0.0),
                    base_rotation: Quat::IDENTITY,
                    base_scale: Vec3::ONE,
                },
            ],
            mesh_instances: Vec::new(),
            animations: Vec::new(),
            root_center_node: Some(0),
            pmx_rig_meta: Some(crate::engine::pmx_rig::PmxRigMeta {
                bones: Vec::new(),
                ik_chains: vec![crate::engine::pmx_rig::IKChain {
                    controller_bone_index: 0,
                    target_bone_index: 2,
                    chain_root_bone_index: 1,
                    iterations: 8,
                    limit_angle: 1.0,
                    links: vec![crate::engine::pmx_rig::IKLink {
                        bone_index: 1,
                        angle_limits: None,
                    }],
                }],
                grant_evaluation_order: Vec::new(),
                grant_cycle_bones: Vec::new(),
            }),
            pmx_physics_meta: None,
            material_morphs: Vec::new(),
        };

        let mut pipeline = FramePipeline::new(&scene);
        pipeline.prepare_frame(&scene, 0.0, None, None, 0.0);

        let effector_world = pipeline.globals()[2].transform_point3(Vec3::ZERO);
        assert!(effector_world.y > 0.5);
    }

    #[test]
    fn real_rabbit_pmx_vmd_pipeline_stays_finite() {
        let pmx_path = Path::new("assets/pmx/miku/rabbit.pmx");
        let vmd_path = Path::new("assets/vmd/rabbit.vmd");
        if !pmx_path.exists() || !vmd_path.exists() {
            return;
        }

        let mut scene = loader::load_pmx(pmx_path).expect("load rabbit pmx");
        let vmd = parse_vmd_motion(vmd_path).expect("parse rabbit vmd");
        scene.animations.push(vmd.to_clip_for_scene(&scene));
        let animation_index = scene.animations.len().checked_sub(1);

        let mut pipeline = FramePipeline::new(&scene);
        let mut physics = PmxPhysicsState::from_scene(
            &scene,
            crate::runtime::state::RuntimePmxSettings::default(),
        )
        .expect("physics state");

        for sample_time in [0.0_f32, 1.0 / 60.0, 0.5, 1.0] {
            pipeline.prepare_frame(
                &scene,
                sample_time,
                animation_index,
                Some(&mut physics),
                1.0 / 60.0,
            );
            assert!(
                pipeline
                    .globals()
                    .iter()
                    .all(|matrix| matrix.to_cols_array().iter().all(|value| value.is_finite())),
                "non-finite matrix detected at sample_time={sample_time}"
            );
        }
    }

    #[test]
    #[ignore = "debug PMX inspection"]
    fn debug_rabbit_pmx_vmd_outlier_bones() {
        let pmx_path = Path::new("assets/pmx/miku/rabbit.pmx");
        let vmd_path = Path::new("assets/vmd/rabbit.vmd");
        if !pmx_path.exists() || !vmd_path.exists() {
            return;
        }

        let mut scene = loader::load_pmx(pmx_path).expect("load rabbit pmx");
        let vmd = parse_vmd_motion(vmd_path).expect("parse rabbit vmd");
        scene.animations.push(vmd.to_clip_for_scene(&scene));
        let animation_index = scene.animations.len().checked_sub(1);

        let mut poses = Vec::new();
        reset_poses_from_nodes(&scene.nodes, &mut poses);
        let mut node_morph_weights = Vec::new();
        seed_node_morph_weights(&scene, &mut node_morph_weights);
        let mut material_morph_weights = vec![0.0; scene.material_morphs.len()];
        if let Some(index) = animation_index {
            scene.animations[index].sample_into_with_morph(
                0.5,
                &mut poses,
                &mut node_morph_weights,
                &mut material_morph_weights,
            );
        }

        apply_pmx_pose_stack(&scene, &mut poses, true);
        let mut pre_physics_globals = Vec::new();
        let mut pre_physics_visited = Vec::new();
        compute_global_matrices_in_place(
            &scene.nodes,
            &poses,
            &mut pre_physics_globals,
            &mut pre_physics_visited,
        );
        debug_rabbit_focus("pre_physics", &scene, &pre_physics_globals);

        let mut physics = PmxPhysicsState::from_scene(
            &scene,
            crate::runtime::state::RuntimePmxSettings::default(),
        )
        .expect("physics state");
        physics.step(&scene, &mut poses, &pre_physics_globals, 1.0 / 60.0);

        let mut post_physics_globals = Vec::new();
        let mut post_physics_visited = Vec::new();
        compute_global_matrices_in_place(
            &scene.nodes,
            &poses,
            &mut post_physics_globals,
            &mut post_physics_visited,
        );
        debug_rabbit_focus("post_physics", &scene, &post_physics_globals);

        apply_pmx_pose_stack(&scene, &mut poses, true);
        let mut final_globals = Vec::new();
        let mut final_visited = Vec::new();
        compute_global_matrices_in_place(
            &scene.nodes,
            &poses,
            &mut final_globals,
            &mut final_visited,
        );
        debug_rabbit_focus("final", &scene, &final_globals);

        let mut outliers = scene
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| {
                let position = final_globals[index].transform_point3(Vec3::ZERO);
                (
                    position.length(),
                    index,
                    node.name.clone().unwrap_or_else(|| "<unnamed>".to_owned()),
                    position,
                )
            })
            .collect::<Vec<_>>();
        outliers.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        for (distance, index, name, position) in outliers.into_iter().take(20) {
            eprintln!(
                "rabbit_outlier index={} name={} distance={:.3} pos=({:.3},{:.3},{:.3})",
                index, name, distance, position.x, position.y, position.z
            );
        }

        if let Some(rig_meta) = scene.pmx_rig_meta.as_ref() {
            for chain in &rig_meta.ik_chains {
                let controller = scene.nodes[chain.controller_bone_index]
                    .name
                    .as_deref()
                    .unwrap_or("<unnamed>");
                let target = scene.nodes[chain.target_bone_index]
                    .name
                    .as_deref()
                    .unwrap_or("<unnamed>");
                let links = chain
                    .links
                    .iter()
                    .map(|link| {
                        scene.nodes[link.bone_index]
                            .name
                            .clone()
                            .unwrap_or_else(|| format!("#{}", link.bone_index))
                    })
                    .collect::<Vec<_>>();
                eprintln!(
                    "rabbit_ik controller={} target={} links={:?}",
                    controller, target, links
                );
            }
        }
    }

    fn debug_rabbit_focus(label: &str, scene: &SceneCpu, globals: &[Mat4]) {
        for (index, node) in scene.nodes.iter().enumerate() {
            let name = node.name.as_deref().unwrap_or("");
            if !(name.contains("スカート")
                || name.contains("足")
                || name.contains("ひざ")
                || name.contains("髪"))
            {
                continue;
            }
            let position = globals[index].transform_point3(Vec3::ZERO);
            eprintln!(
                "rabbit_focus stage={} index={} name={} pos=({:.3},{:.3},{:.3})",
                label, index, name, position.x, position.y, position.z
            );
            if let Some(rig_meta) = scene.pmx_rig_meta.as_ref() {
                if let Some(bone) = rig_meta.bones.get(index) {
                    if let Some(grant) = bone.grant_transform.as_ref() {
                        let parent_name = scene.nodes[grant.parent_index]
                            .name
                            .as_deref()
                            .unwrap_or("<unnamed>");
                        eprintln!(
                            "rabbit_focus_grant stage={} index={} name={} parent={} weight={:.3} local={} rot={} pos={} local_axis={} fixed_axis={}",
                            label,
                            index,
                            name,
                            parent_name,
                            grant.weight,
                            grant.is_local,
                            grant.affects_rotation,
                            grant.affects_translation,
                            bone.uses_local_axis(),
                            bone.uses_fixed_axis(),
                        );
                    } else {
                        eprintln!(
                            "rabbit_focus_meta stage={} index={} name={} local_axis={} fixed_axis={} append_rot={} append_pos={}",
                            label,
                            index,
                            name,
                            bone.uses_local_axis(),
                            bone.uses_fixed_axis(),
                            bone.uses_append_rotation(),
                            bone.uses_append_translation(),
                        );
                    }
                }
            }
        }
    }
}

fn seed_node_morph_weights(scene: &SceneCpu, node_morph_weights: &mut Vec<Vec<f32>>) {
    node_morph_weights.resize_with(scene.nodes.len(), Vec::new);
    for weights in node_morph_weights.iter_mut() {
        weights.clear();
    }
    for instance in &scene.mesh_instances {
        let Some(node_weights) = node_morph_weights.get_mut(instance.node_index) else {
            continue;
        };
        if node_weights.len() < instance.default_morph_weights.len() {
            node_weights.resize(instance.default_morph_weights.len(), 0.0);
        }
        for (i, value) in instance.default_morph_weights.iter().enumerate() {
            node_weights[i] = *value;
        }
    }
}

fn resolve_instance_morph_weights(
    scene: &SceneCpu,
    node_morph_weights: &[Vec<f32>],
    instance_morph_weights: &mut Vec<Vec<f32>>,
) {
    instance_morph_weights.resize_with(scene.mesh_instances.len(), Vec::new);
    for (instance_index, instance) in scene.mesh_instances.iter().enumerate() {
        let dst = &mut instance_morph_weights[instance_index];
        dst.clear();
        if let Some(node_weights) = node_morph_weights.get(instance.node_index) {
            if !node_weights.is_empty() {
                dst.extend_from_slice(node_weights);
                continue;
            }
        }
        if !instance.default_morph_weights.is_empty() {
            dst.extend_from_slice(&instance.default_morph_weights);
        }
    }
}
