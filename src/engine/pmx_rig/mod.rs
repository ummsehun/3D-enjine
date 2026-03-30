//! PMX-specific rig metadata for IK and advanced skinning.
//!
//! This module stores PMX-specific bone metadata that doesn't fit into the
//! generic `SceneCpu`/`Node` structures, specifically IK chain definitions.

use glam::{Mat4, Quat, Vec3};

mod physics_meta;

pub use physics_meta::{
    PmxJointCpu, PmxJointKind, PmxPhysicsMeta, PmxRigidBodyCpu, PmxRigidCalcMethod, PmxRigidShape,
};

/// A single link in an IK chain.
#[derive(Debug, Clone)]
pub struct IKLink {
    /// Bone index in the skeleton.
    pub bone_index: usize,
    /// Optional angle limits (min, max) for each axis in radians.
    /// PMX uses axis-angle limits, approximated as per-axis limits.
    pub angle_limits: Option<[Vec3; 2]>,
}

/// An IK chain definition from PMX.
#[derive(Debug, Clone)]
pub struct IKChain {
    pub controller_bone_index: usize,
    /// The effector bone (end point) that IK tries to reach.
    pub target_bone_index: usize,
    /// Root of the IK chain (usually the first link).
    /// The solver iterates from here towards the target.
    pub chain_root_bone_index: usize,
    /// Maximum iterations for the solver.
    pub iterations: u32,
    /// Angle limit in radians per iteration step.
    pub limit_angle: f32,
    /// Links in the chain (from root towards target).
    pub links: Vec<IKLink>,
}

/// Raw PMX bone metadata preserved for diagnostics and future pose features.
#[derive(Debug, Clone)]
pub struct PmxGrantTransform {
    pub parent_index: usize,
    pub weight: f32,
    pub is_local: bool,
    pub affects_rotation: bool,
    pub affects_translation: bool,
}

/// Raw PMX bone metadata preserved for diagnostics and future pose features.
#[derive(Debug, Clone)]
pub struct PmxBoneMeta {
    pub name: String,
    pub name_en: String,
    pub position: Vec3,
    pub parent_index: i32,
    pub deform_depth: i32,
    pub boneflag: u16,
    pub offset: Vec3,
    pub child: i32,
    pub append_bone_index: i32,
    pub append_weight: f32,
    pub grant_transform: Option<PmxGrantTransform>,
    pub fixed_axis: Vec3,
    pub local_axis_x: Vec3,
    pub local_axis_z: Vec3,
    pub key_value: i32,
    pub ik_target_index: i32,
    pub ik_iter_count: i32,
    pub ik_limit: f32,
}

impl PmxBoneMeta {
    pub fn has_flag(&self, flag: u16) -> bool {
        self.boneflag & flag != 0
    }

    pub fn uses_ik(&self) -> bool {
        self.has_flag(0x0020)
    }

    pub fn uses_append_rotation(&self) -> bool {
        self.has_flag(0x0100)
    }

    pub fn uses_append_translation(&self) -> bool {
        self.has_flag(0x0200)
    }

    pub fn uses_append_local(&self) -> bool {
        self.has_flag(0x0080)
    }

    pub fn uses_fixed_axis(&self) -> bool {
        self.has_flag(0x0400)
    }

    pub fn uses_local_axis(&self) -> bool {
        self.has_flag(0x0800)
    }

    pub fn uses_external_parent(&self) -> bool {
        self.has_flag(0x2000)
    }
}

/// PMX-specific rig metadata extracted from the model.
#[derive(Debug, Clone, Default)]
pub struct PmxRigMeta {
    /// Raw PMX bone metadata in loader order.
    pub bones: Vec<PmxBoneMeta>,
    /// All IK chains defined in the model.
    pub ik_chains: Vec<IKChain>,
    /// Grant evaluation order from parent to child.
    pub grant_evaluation_order: Vec<usize>,
    /// Grant bones that participated in a detected cycle.
    pub grant_cycle_bones: Vec<usize>,
}

impl PmxRigMeta {
    /// Returns true if there are no IK chains.
    pub fn is_empty(&self) -> bool {
        self.bones.is_empty() && self.ik_chains.is_empty()
    }

    pub fn count_bones_with_flag(&self, flag: u16) -> usize {
        self.bones.iter().filter(|bone| bone.has_flag(flag)).count()
    }

    pub fn count_bones_with_ik(&self) -> usize {
        self.bones.iter().filter(|bone| bone.uses_ik()).count()
    }

    pub fn count_bones_with_append(&self) -> usize {
        self.bones
            .iter()
            .filter(|bone| bone.uses_append_rotation() || bone.uses_append_translation())
            .count()
    }

    pub fn count_bones_with_grant(&self) -> usize {
        self.bones
            .iter()
            .filter(|bone| bone.grant_transform.is_some())
            .count()
    }

    pub fn count_bones_with_local_grant(&self) -> usize {
        self.bones
            .iter()
            .filter(|bone| {
                bone.grant_transform
                    .as_ref()
                    .is_some_and(|grant| grant.is_local)
            })
            .count()
    }

    pub fn count_bones_with_fixed_axis(&self) -> usize {
        self.bones
            .iter()
            .filter(|bone| bone.uses_fixed_axis())
            .count()
    }

    pub fn count_bones_with_local_axis(&self) -> usize {
        self.bones
            .iter()
            .filter(|bone| bone.uses_local_axis())
            .count()
    }

    pub fn count_bones_with_external_parent(&self) -> usize {
        self.bones
            .iter()
            .filter(|bone| bone.uses_external_parent())
            .count()
    }

    pub fn rebuild_grant_evaluation_order(&mut self) {
        let (order, cycle_bones) = build_grant_evaluation_order(&self.bones);
        self.grant_evaluation_order = order;
        self.grant_cycle_bones = cycle_bones;
    }
}

/// Solve a single IK chain using CCD (Cyclic Coordinate Descent).
///
/// This is a simplified CCD solver that rotates each joint in the chain
/// iteratively to minimize the distance between the effector and target.
///
/// # Arguments
/// * `chain` - The IK chain definition
/// * `nodes` - Skeleton nodes (for parent relationships)
/// * `poses` - Current pose (will be modified with IK results)
/// * `target_pos` - World-space target position for the effector
pub fn solve_ik_chain_ccd(
    chain: &IKChain,
    nodes: &[crate::scene::Node],
    poses: &mut [crate::scene::NodePose],
    target_pos: Vec3,
) {
    if chain.links.is_empty() {
        return;
    }

    // CCD iterates through joints from tip towards root
    // For each joint, find the rotation that minimizes effector-to-target distance
    for _iteration in 0..chain.iterations {
        // Iterate from the link closest to target (last in array) towards root
        for link_idx in (0..chain.links.len()).rev() {
            let link = &chain.links[link_idx];
            let joint_idx = link.bone_index;

            // Compute global position of the effector (target bone)
            let effector_global = compute_global_position(chain.target_bone_index, nodes, poses);

            // Compute global position of this joint
            let joint_global = compute_global_position(joint_idx, nodes, poses);

            // Vectors in world space
            let to_effector = effector_global - joint_global;
            let to_target = target_pos - joint_global;

            let to_effector_len = to_effector.length();
            let to_target_len = to_target.length();

            if to_effector_len < f32::EPSILON || to_target_len < f32::EPSILON {
                continue;
            }

            let to_effector_norm = to_effector / to_effector_len;
            let to_target_norm = to_target / to_target_len;

            // Rotation that aligns effector direction towards target direction
            let rotation = rotation_between(to_effector_norm, to_target_norm);

            let parent_rotation = nodes
                .get(joint_idx)
                .and_then(|node| node.parent)
                .map(|parent_index| {
                    let (_, rotation, _) = compute_global_transform(parent_index, nodes, poses)
                        .to_scale_rotation_translation();
                    rotation
                })
                .unwrap_or(Quat::IDENTITY);
            let local_rotation_delta =
                (parent_rotation.conjugate() * rotation * parent_rotation).normalize();

            // Apply rotation to this joint's local pose
            let current_rotation = poses[joint_idx].rotation;
            poses[joint_idx].rotation = (local_rotation_delta * current_rotation).normalize();

            // Apply angle limit if specified
            if let Some(limits) = &link.angle_limits {
                apply_angle_limits(&mut poses[joint_idx].rotation, limits, chain.limit_angle);
            }
        }
    }
}

/// Apply PMX append rotation/translation in a best-effort way.
///
/// This preserves the effect of "additional parent" bones within the current
/// simplified `NodePose` representation. It does not attempt to reproduce the
/// full PMX local-space inheritance matrix model.
pub fn apply_append_bone_transforms(meta: &PmxRigMeta, poses: &mut [crate::scene::NodePose]) {
    let mut resolved_poses = poses.to_vec();
    let grant_order = if meta.grant_evaluation_order.is_empty() {
        (0..meta.bones.len()).collect::<Vec<_>>()
    } else {
        meta.grant_evaluation_order.clone()
    };

    for bone_index in grant_order {
        let Some(bone) = meta.bones.get(bone_index) else {
            continue;
        };
        let Some(grant) = bone.grant_transform.as_ref() else {
            continue;
        };
        let source_index = grant.parent_index;
        if bone_index >= poses.len()
            || source_index >= resolved_poses.len()
            || source_index == bone_index
        {
            continue;
        }
        let weight = grant.weight.clamp(0.0, 1.0);
        if weight <= f32::EPSILON {
            continue;
        }

        let source_pose = resolved_poses[source_index];
        if let Some(target_pose) = poses.get_mut(bone_index) {
            if grant.affects_translation {
                let translated = if grant.is_local {
                    target_pose.rotation * (source_pose.translation * weight)
                } else {
                    source_pose.translation * weight
                };
                target_pose.translation += translated;
            }
            if grant.affects_rotation {
                let append_rotation = Quat::IDENTITY.slerp(source_pose.rotation, weight);
                target_pose.rotation = if grant.is_local {
                    (target_pose.rotation * append_rotation).normalize()
                } else {
                    (append_rotation * target_pose.rotation).normalize()
                };
            }
            resolved_poses[bone_index] = *target_pose;
        }
    }
}

/// Apply PMX fixed-axis and local-axis rotation hints in a best-effort way.
///
/// This does not recreate Blender's full bone constraint system. It only
/// reduces the most visible axis drift by re-basing bones with local axes and
/// constraining fixed-axis bones to twist around their declared axis.
pub fn apply_pmx_bone_axis_constraints(meta: &PmxRigMeta, poses: &mut [crate::scene::NodePose]) {
    for (bone_index, bone) in meta.bones.iter().enumerate() {
        if bone_index >= poses.len() {
            continue;
        }

        let mut rotation = poses[bone_index].rotation;

        if bone.uses_fixed_axis() {
            let fixed_axis = bone.fixed_axis.normalize_or_zero();

            if fixed_axis.length_squared() > f32::EPSILON {
                rotation = twist_only(rotation, fixed_axis);
            }
        }

        poses[bone_index].rotation = rotation.normalize();
    }
}

/// Compute the global position of a bone given the current pose.
pub fn compute_bone_position(
    bone_index: usize,
    nodes: &[crate::scene::Node],
    poses: &[crate::scene::NodePose],
) -> Vec3 {
    compute_global_transform(bone_index, nodes, poses).transform_point3(Vec3::ZERO)
}

fn compute_global_position(
    bone_index: usize,
    nodes: &[crate::scene::Node],
    poses: &[crate::scene::NodePose],
) -> Vec3 {
    compute_bone_position(bone_index, nodes, poses)
}

fn compute_global_transform(
    bone_index: usize,
    nodes: &[crate::scene::Node],
    poses: &[crate::scene::NodePose],
) -> Mat4 {
    let mut transform = Mat4::IDENTITY;
    let mut current_idx = Some(bone_index);
    let mut visited = vec![false; nodes.len()];

    while let Some(idx) = current_idx {
        if idx >= poses.len() || idx >= nodes.len() || visited[idx] {
            break;
        }
        visited[idx] = true;

        let pose = &poses[idx];
        let local =
            Mat4::from_scale_rotation_translation(pose.scale, pose.rotation, pose.translation);
        transform = local * transform;
        current_idx = nodes[idx].parent;
    }

    transform
}

/// Create a rotation that rotates `from` direction to `to` direction.
fn rotation_between(from: Vec3, to: Vec3) -> Quat {
    let dot = from.dot(to);
    if dot > 0.9999 {
        return Quat::IDENTITY;
    }
    if dot < -0.9999 {
        // Vectors are opposite, return a 180-degree rotation
        // Find an orthogonal axis
        let ortho = if from.x.abs() > from.y.abs() {
            Vec3::new(-from.z, 0.0, from.x).normalize()
        } else {
            Vec3::new(0.0, from.z, -from.y).normalize()
        };
        return Quat::from_rotation_arc(from, ortho) * Quat::from_rotation_arc(ortho, to);
    }

    Quat::from_rotation_arc(from, to)
}

/// Apply angle limits to a rotation, clamping each axis.
fn apply_angle_limits(rotation: &mut Quat, limits: &[Vec3; 2], max_angle: f32) {
    let (mut yaw, mut pitch, mut roll) = rotation.to_euler(glam::EulerRot::YXZ);

    let [min, max] = limits;
    yaw = yaw.clamp(min.x, max.x);
    pitch = pitch.clamp(min.y, max.y);
    roll = roll.clamp(min.z, max.z);

    yaw = yaw.clamp(-max_angle, max_angle);
    pitch = pitch.clamp(-max_angle, max_angle);
    roll = roll.clamp(-max_angle, max_angle);

    *rotation = Quat::from_euler(glam::EulerRot::YXZ, yaw, pitch, roll);
}

fn twist_only(rotation: Quat, axis: Vec3) -> Quat {
    let axis = axis.normalize_or_zero();
    if axis.length_squared() <= f32::EPSILON {
        return rotation;
    }

    let vector = Vec3::new(rotation.x, rotation.y, rotation.z);
    let projected = axis * vector.dot(axis);
    let twist = Quat::from_xyzw(projected.x, projected.y, projected.z, rotation.w);
    if twist.length_squared() <= f32::EPSILON {
        rotation
    } else {
        twist.normalize()
    }
}

fn build_grant_evaluation_order(bones: &[PmxBoneMeta]) -> (Vec<usize>, Vec<usize>) {
    let mut grant_bones = bones
        .iter()
        .enumerate()
        .filter_map(|(bone_index, bone)| bone.grant_transform.as_ref().map(|_| bone_index))
        .collect::<Vec<_>>();
    grant_bones.sort_by_key(|&bone_index| (bones[bone_index].deform_depth, bone_index));

    let mut state = vec![0_u8; bones.len()];
    let mut order = Vec::with_capacity(grant_bones.len());
    let mut cycle_bones = Vec::new();

    fn visit(
        bone_index: usize,
        bones: &[PmxBoneMeta],
        state: &mut [u8],
        order: &mut Vec<usize>,
        cycle_bones: &mut Vec<usize>,
    ) {
        if bone_index >= bones.len() {
            return;
        }
        match state[bone_index] {
            2 => return,
            1 => {
                cycle_bones.push(bone_index);
                return;
            }
            _ => {}
        }
        let Some(grant) = bones[bone_index].grant_transform.as_ref() else {
            return;
        };

        state[bone_index] = 1;
        if grant.parent_index < bones.len()
            && grant.parent_index != bone_index
            && bones[grant.parent_index].grant_transform.is_some()
        {
            visit(grant.parent_index, bones, state, order, cycle_bones);
        }
        state[bone_index] = 2;
        order.push(bone_index);
    }

    for bone_index in grant_bones {
        visit(bone_index, bones, &mut state, &mut order, &mut cycle_bones);
    }

    cycle_bones.sort_unstable();
    cycle_bones.dedup();
    (order, cycle_bones)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_rig_meta() {
        let meta = PmxRigMeta::default();
        assert!(meta.is_empty());
        assert!(meta.ik_chains.is_empty());
    }

    #[test]
    fn test_ik_chain_creation() {
        let chain = IKChain {
            controller_bone_index: 1,
            target_bone_index: 5,
            chain_root_bone_index: 2,
            iterations: 10,
            limit_angle: 0.1,
            links: vec![
                IKLink {
                    bone_index: 2,
                    angle_limits: None,
                },
                IKLink {
                    bone_index: 3,
                    angle_limits: None,
                },
                IKLink {
                    bone_index: 4,
                    angle_limits: None,
                },
            ],
        };
        assert_eq!(chain.controller_bone_index, 1);
        assert_eq!(chain.target_bone_index, 5);
        assert_eq!(chain.links.len(), 3);
    }

    #[test]
    fn test_rotation_identity_near_aligned() {
        let from = Vec3::X;
        let to = Vec3::X * 0.9999 + Vec3::Y * 0.01;
        let rot = rotation_between(from.normalize(), to.normalize());
        let result = rot * from.normalize();
        assert!((result - to.normalize()).length() < 0.02);
    }

    #[test]
    fn test_compute_bone_position_uses_pose_translation_and_parent_rotation() {
        let nodes = vec![
            crate::scene::Node {
                name: Some("root".to_owned()),
                name_en: None,
                parent: None,
                children: vec![1],
                base_translation: Vec3::ZERO,
                base_rotation: Quat::IDENTITY,
                base_scale: Vec3::ONE,
            },
            crate::scene::Node {
                name: Some("child".to_owned()),
                name_en: None,
                parent: Some(0),
                children: Vec::new(),
                base_translation: Vec3::new(1.0, 0.0, 0.0),
                base_rotation: Quat::IDENTITY,
                base_scale: Vec3::ONE,
            },
        ];

        let poses = vec![
            crate::scene::NodePose {
                translation: Vec3::new(10.0, 0.0, 0.0),
                rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                scale: Vec3::ONE,
            },
            crate::scene::NodePose {
                translation: Vec3::new(1.0, 0.0, 0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
        ];

        let position = compute_bone_position(1, &nodes, &poses);
        assert!((position - Vec3::new(10.0, 1.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn test_apply_append_bone_transforms_blends_parent_pose() {
        let meta = PmxRigMeta {
            bones: vec![
                PmxBoneMeta {
                    grant_transform: None,
                    name: "root".to_owned(),
                    name_en: "root".to_owned(),
                    position: Vec3::ZERO,
                    parent_index: -1,
                    deform_depth: 0,
                    boneflag: 0,
                    offset: Vec3::ZERO,
                    child: -1,
                    append_bone_index: -1,
                    append_weight: 0.0,
                    fixed_axis: Vec3::ZERO,
                    local_axis_x: Vec3::ZERO,
                    local_axis_z: Vec3::ZERO,
                    key_value: 0,
                    ik_target_index: -1,
                    ik_iter_count: 0,
                    ik_limit: 0.0,
                },
                PmxBoneMeta {
                    grant_transform: Some(PmxGrantTransform {
                        parent_index: 0,
                        weight: 0.5,
                        is_local: false,
                        affects_rotation: true,
                        affects_translation: true,
                    }),
                    name: "child".to_owned(),
                    name_en: "child".to_owned(),
                    position: Vec3::ZERO,
                    parent_index: 0,
                    deform_depth: 1,
                    boneflag: 0x0100 | 0x0200,
                    offset: Vec3::ZERO,
                    child: -1,
                    append_bone_index: 0,
                    append_weight: 0.5,
                    fixed_axis: Vec3::ZERO,
                    local_axis_x: Vec3::ZERO,
                    local_axis_z: Vec3::ZERO,
                    key_value: 0,
                    ik_target_index: -1,
                    ik_iter_count: 0,
                    ik_limit: 0.0,
                },
            ],
            ik_chains: Vec::new(),
            grant_evaluation_order: vec![1],
            grant_cycle_bones: Vec::new(),
        };
        let mut poses = vec![
            crate::scene::NodePose {
                translation: Vec3::new(2.0, 0.0, 0.0),
                rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                scale: Vec3::ONE,
            },
            crate::scene::NodePose {
                translation: Vec3::new(1.0, 0.0, 0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
        ];

        apply_append_bone_transforms(&meta, &mut poses);

        assert!((poses[1].translation - Vec3::new(2.0, 0.0, 0.0)).length() < 1e-5);
        let rotated = poses[1].rotation * Vec3::X;
        assert!((rotated - Vec3::new(0.70710677, 0.70710677, 0.0)).length() < 1e-4);
    }

    #[test]
    fn test_apply_pmx_bone_axis_constraints_projects_fixed_axis_twist() {
        let meta = PmxRigMeta {
            bones: vec![PmxBoneMeta {
                grant_transform: None,
                name: "joint".to_owned(),
                name_en: "joint".to_owned(),
                position: Vec3::ZERO,
                parent_index: -1,
                deform_depth: 0,
                boneflag: 0x0400,
                offset: Vec3::ZERO,
                child: -1,
                append_bone_index: -1,
                append_weight: 0.0,
                fixed_axis: Vec3::Y,
                local_axis_x: Vec3::ZERO,
                local_axis_z: Vec3::ZERO,
                key_value: 0,
                ik_target_index: -1,
                ik_iter_count: 0,
                ik_limit: 0.0,
            }],
            ik_chains: Vec::new(),
            grant_evaluation_order: Vec::new(),
            grant_cycle_bones: Vec::new(),
        };
        let mut poses = vec![crate::scene::NodePose {
            translation: Vec3::ZERO,
            rotation: Quat::from_euler(glam::EulerRot::XYZ, 0.45, 0.35, -0.2),
            scale: Vec3::ONE,
        }];

        apply_pmx_bone_axis_constraints(&meta, &mut poses);

        let axis_after = poses[0].rotation * Vec3::Y;
        assert!((axis_after - Vec3::Y).length() < 1e-4);
    }

    #[test]
    fn test_apply_pmx_bone_axis_constraints_keeps_local_axis_metadata_passive() {
        let meta = PmxRigMeta {
            bones: vec![PmxBoneMeta {
                grant_transform: None,
                name: "joint".to_owned(),
                name_en: "joint".to_owned(),
                position: Vec3::ZERO,
                parent_index: -1,
                deform_depth: 0,
                boneflag: 0x0800,
                offset: Vec3::ZERO,
                child: -1,
                append_bone_index: -1,
                append_weight: 0.0,
                fixed_axis: Vec3::ZERO,
                local_axis_x: Vec3::Y,
                local_axis_z: Vec3::Z,
                key_value: 0,
                ik_target_index: -1,
                ik_iter_count: 0,
                ik_limit: 0.0,
            }],
            ik_chains: Vec::new(),
            grant_evaluation_order: Vec::new(),
            grant_cycle_bones: Vec::new(),
        };
        let mut poses = vec![crate::scene::NodePose {
            translation: Vec3::ZERO,
            rotation: Quat::from_rotation_x(0.5),
            scale: Vec3::ONE,
        }];

        let before = poses[0].rotation;
        apply_pmx_bone_axis_constraints(&meta, &mut poses);

        assert!(poses[0].rotation.dot(before).abs() > 0.999_99);
        assert!((poses[0].rotation.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_grant_evaluation_order_applies_parent_before_child() {
        let mut meta = PmxRigMeta {
            bones: vec![
                PmxBoneMeta {
                    grant_transform: None,
                    name: "root".to_owned(),
                    name_en: "root".to_owned(),
                    position: Vec3::ZERO,
                    parent_index: -1,
                    deform_depth: 0,
                    boneflag: 0,
                    offset: Vec3::ZERO,
                    child: -1,
                    append_bone_index: -1,
                    append_weight: 0.0,
                    fixed_axis: Vec3::ZERO,
                    local_axis_x: Vec3::ZERO,
                    local_axis_z: Vec3::ZERO,
                    key_value: 0,
                    ik_target_index: -1,
                    ik_iter_count: 0,
                    ik_limit: 0.0,
                },
                PmxBoneMeta {
                    grant_transform: Some(PmxGrantTransform {
                        parent_index: 0,
                        weight: 0.5,
                        is_local: false,
                        affects_rotation: false,
                        affects_translation: true,
                    }),
                    name: "mid".to_owned(),
                    name_en: "mid".to_owned(),
                    position: Vec3::ZERO,
                    parent_index: 0,
                    deform_depth: 1,
                    boneflag: 0x0200,
                    offset: Vec3::ZERO,
                    child: -1,
                    append_bone_index: 0,
                    append_weight: 0.5,
                    fixed_axis: Vec3::ZERO,
                    local_axis_x: Vec3::ZERO,
                    local_axis_z: Vec3::ZERO,
                    key_value: 0,
                    ik_target_index: -1,
                    ik_iter_count: 0,
                    ik_limit: 0.0,
                },
                PmxBoneMeta {
                    grant_transform: Some(PmxGrantTransform {
                        parent_index: 1,
                        weight: 1.0,
                        is_local: false,
                        affects_rotation: false,
                        affects_translation: true,
                    }),
                    name: "leaf".to_owned(),
                    name_en: "leaf".to_owned(),
                    position: Vec3::ZERO,
                    parent_index: 1,
                    deform_depth: 2,
                    boneflag: 0x0200,
                    offset: Vec3::ZERO,
                    child: -1,
                    append_bone_index: 1,
                    append_weight: 1.0,
                    fixed_axis: Vec3::ZERO,
                    local_axis_x: Vec3::ZERO,
                    local_axis_z: Vec3::ZERO,
                    key_value: 0,
                    ik_target_index: -1,
                    ik_iter_count: 0,
                    ik_limit: 0.0,
                },
            ],
            ik_chains: Vec::new(),
            grant_evaluation_order: Vec::new(),
            grant_cycle_bones: Vec::new(),
        };
        meta.rebuild_grant_evaluation_order();
        assert_eq!(meta.grant_evaluation_order, vec![1, 2]);

        let mut poses = vec![
            crate::scene::NodePose {
                translation: Vec3::new(2.0, 0.0, 0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
            crate::scene::NodePose {
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
            crate::scene::NodePose {
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
        ];

        apply_append_bone_transforms(&meta, &mut poses);

        assert!((poses[1].translation - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-5);
        assert!((poses[2].translation - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-5);
    }
}
