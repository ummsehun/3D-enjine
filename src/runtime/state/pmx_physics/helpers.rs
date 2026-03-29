use glam::{EulerRot, Mat4, Quat, Vec3};

use crate::engine::pmx_rig::PmxRigidShape;
use crate::scene::SceneCpu;

use super::{JointRuntime, JointRuntimeKind, RigidBodyRuntime};

pub(super) fn apply_joint_limits(
    joint: &JointRuntime,
    body_a: &mut RigidBodyRuntime,
    body_b: &mut RigidBodyRuntime,
    _strength: f32,
    position_limit_gain: f32,
    rotation_limit_gain: f32,
) {
    let position_limit_strength = position_limit_gain.clamp(0.75, 1.0);
    let rotation_limit_strength = rotation_limit_gain.clamp(0.35, 0.95);
    let joint_inv = joint.joint_rotation.conjugate();
    let a_local_pos = joint_inv * (body_a.position - joint.joint_position);
    let b_local_pos = joint_inv * (body_b.position - joint.joint_position);
    let rel_local_pos = b_local_pos - a_local_pos;
    let mut position_correction_local = Vec3::ZERO;

    match joint.kind {
        JointRuntimeKind::Spring6Dof | JointRuntimeKind::SixDof => {
            if let (Some(min), Some(max)) = (joint.move_limit_down, joint.move_limit_up) {
                let clamped = clamp_vec3(rel_local_pos, min, max);
                position_correction_local = rel_local_pos - clamped;
            }
        }
        JointRuntimeKind::Slider => {
            if let (Some(min), Some(max)) = (joint.lower_linear_limit, joint.upper_linear_limit) {
                let x = rel_local_pos.x.clamp(min, max);
                position_correction_local = Vec3::new(rel_local_pos.x - x, 0.0, 0.0);
            }
        }
        _ => {}
    }

    if position_correction_local.length_squared() > f32::EPSILON {
        let correction_world = joint.joint_rotation * position_correction_local;
        let inv_a = body_a.inverse_mass;
        let inv_b = body_b.inverse_mass;
        let total = inv_a + inv_b;
        if total > f32::EPSILON {
            body_a.position += correction_world * (inv_a / total) * position_limit_strength;
            body_b.position -= correction_world * (inv_b / total) * position_limit_strength;
        }
    }

    let a_local_rot = joint_inv * body_a.rotation;
    let b_local_rot = joint_inv * body_b.rotation;
    let rel_local_rot = a_local_rot.conjugate() * b_local_rot;

    let desired_rel = match joint.kind {
        JointRuntimeKind::Spring6Dof | JointRuntimeKind::SixDof => {
            match (joint.rotation_limit_down, joint.rotation_limit_up) {
                (Some(min), Some(max)) => Some(clamp_local_rotation(rel_local_rot, min, max)),
                _ => None,
            }
        }
        JointRuntimeKind::Slider => {
            joint
                .lower_angle_limit
                .zip(joint.upper_angle_limit)
                .map(|(min, max)| {
                    let (y, mut x, z) = rel_local_rot.to_euler(EulerRot::YXZ);
                    x = x.clamp(min, max);
                    Quat::from_euler(EulerRot::YXZ, y, x, z)
                })
        }
        JointRuntimeKind::Hinge => {
            joint
                .lower_angle_limit
                .zip(joint.upper_angle_limit)
                .map(|(min, max)| {
                    let (y, mut x, z) = rel_local_rot.to_euler(EulerRot::YXZ);
                    x = x.clamp(min, max);
                    Quat::from_euler(EulerRot::YXZ, y, x, z)
                })
        }
        JointRuntimeKind::ConeTwist => match (
            joint.cone_swing_span1,
            joint.cone_swing_span2,
            joint.cone_twist_span,
        ) {
            (Some(swing1), Some(swing2), Some(twist)) => {
                let (mut y, mut x, mut z) = rel_local_rot.to_euler(EulerRot::YXZ);
                y = y.clamp(-swing1, swing1);
                x = x.clamp(-twist, twist);
                z = z.clamp(-swing2, swing2);
                Some(Quat::from_euler(EulerRot::YXZ, y, x, z))
            }
            _ => None,
        },
        JointRuntimeKind::P2P => None,
    };

    if let Some(desired_rel) = desired_rel {
        let desired_b_world = joint.joint_rotation * (a_local_rot * desired_rel);
        let desired_a_world = joint.joint_rotation * (b_local_rot * desired_rel.conjugate());
        let inv_a = body_a.inverse_mass;
        let inv_b = body_b.inverse_mass;
        let total = inv_a + inv_b;
        let a_share = if total > f32::EPSILON {
            inv_a / total
        } else {
            0.5
        };
        let b_share = if total > f32::EPSILON {
            inv_b / total
        } else {
            0.5
        };
        let rot_alpha = rotation_limit_strength.clamp(0.0, 0.95);
        if inv_a > f32::EPSILON {
            body_a.rotation = body_a
                .rotation
                .slerp(desired_a_world, rot_alpha * a_share)
                .normalize();
        }
        if inv_b > f32::EPSILON {
            body_b.rotation = body_b
                .rotation
                .slerp(desired_b_world, rot_alpha * b_share)
                .normalize();
        }
    }
}

pub(super) fn clamp_vec3(value: Vec3, min: Vec3, max: Vec3) -> Vec3 {
    Vec3::new(
        value.x.clamp(min.x, max.x),
        value.y.clamp(min.y, max.y),
        value.z.clamp(min.z, max.z),
    )
}

pub(super) fn clamp_local_rotation(rotation: Quat, min: Vec3, max: Vec3) -> Quat {
    let (mut y, mut x, mut z) = rotation.to_euler(EulerRot::YXZ);
    y = y.clamp(min.x, max.x);
    x = x.clamp(min.y, max.y);
    z = z.clamp(min.z, max.z);
    Quat::from_euler(EulerRot::YXZ, y, x, z)
}

pub(super) fn target_body_transform(
    scene: &SceneCpu,
    pre_physics_globals: &[Mat4],
    body: &RigidBodyRuntime,
) -> (Vec3, Quat) {
    let local = Mat4::from_scale_rotation_translation(
        Vec3::ONE,
        body.local_rotation,
        body.local_translation,
    );
    let target = body
        .bone_index
        .and_then(|bone_index| pre_physics_globals.get(bone_index).copied())
        .map(|bone_global| bone_global * local)
        .unwrap_or(local);
    let (_, rotation, translation) = target.to_scale_rotation_translation();
    if body.bone_index.is_some() && scene.nodes.is_empty() {
        (body.position, body.rotation)
    } else {
        (translation, rotation)
    }
}

pub(super) fn collision_pair_enabled(group: u8, mask: u16, other_group: u8) -> bool {
    let bit = 1u16.checked_shl(other_group as u32).unwrap_or(0);
    (mask & bit) == 0 && group < 16
}

pub(super) fn shape_bounding_radius(shape: PmxRigidShape, size: Vec3) -> f32 {
    let size = size.abs();
    match shape {
        PmxRigidShape::Sphere => size.x,
        PmxRigidShape::Box => size.length(),
        PmxRigidShape::Capsule => size.x + size.y,
    }
    .max(0.01)
}

pub(super) fn shape_support_radius(body: &RigidBodyRuntime, direction: Vec3) -> f32 {
    let direction = direction.normalize_or_zero();
    if direction.length_squared() <= f32::EPSILON {
        return body.radius;
    }

    let size = body.size.abs();
    match body.shape {
        PmxRigidShape::Sphere => size.x.max(0.01),
        PmxRigidShape::Box => {
            let axis_x = body.rotation * Vec3::X;
            let axis_y = body.rotation * Vec3::Y;
            let axis_z = body.rotation * Vec3::Z;
            size.x * direction.dot(axis_x).abs()
                + size.y * direction.dot(axis_y).abs()
                + size.z * direction.dot(axis_z).abs()
        }
        PmxRigidShape::Capsule => {
            let axis = body.rotation * Vec3::Y;
            size.x + size.y * direction.dot(axis).abs()
        }
    }
    .max(0.01)
}
