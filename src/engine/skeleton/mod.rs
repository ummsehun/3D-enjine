use glam::{Mat4, Quat, Vec3};

use crate::scene::{Node, NodePose, SceneCpu};

pub fn default_poses(nodes: &[Node]) -> Vec<NodePose> {
    let mut poses = Vec::with_capacity(nodes.len());
    reset_poses_from_nodes(nodes, &mut poses);
    poses
}

pub fn compute_global_matrices(nodes: &[Node], poses: &[NodePose]) -> Vec<Mat4> {
    let mut globals = Vec::with_capacity(nodes.len());
    let mut visited = Vec::with_capacity(nodes.len());
    compute_global_matrices_in_place(nodes, poses, &mut globals, &mut visited);
    globals
}

fn compute_node_global(
    index: usize,
    nodes: &[Node],
    poses: &[NodePose],
    globals: &mut [Mat4],
    visited: &mut [bool],
) -> Mat4 {
    if visited[index] {
        return globals[index];
    }
    let local = poses
        .get(index)
        .copied()
        .unwrap_or(NodePose {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        })
        .to_mat4();
    let global = if let Some(parent) = nodes[index].parent {
        compute_node_global(parent, nodes, poses, globals, visited) * local
    } else {
        local
    };
    globals[index] = global;
    visited[index] = true;
    global
}

pub fn compute_skin_matrices(scene: &SceneCpu, global_matrices: &[Mat4]) -> Vec<Vec<Mat4>> {
    let mut skin_matrices = Vec::with_capacity(scene.skins.len());
    compute_skin_matrices_in_place(scene, global_matrices, &mut skin_matrices);
    skin_matrices
}

pub fn reset_poses_from_nodes(nodes: &[Node], poses: &mut Vec<NodePose>) {
    poses.clear();
    poses.extend(nodes.iter().map(NodePose::from));
}

pub fn compute_global_matrices_in_place(
    nodes: &[Node],
    poses: &[NodePose],
    globals: &mut Vec<Mat4>,
    visited: &mut Vec<bool>,
) {
    globals.resize(nodes.len(), Mat4::IDENTITY);
    visited.resize(nodes.len(), false);
    visited.fill(false);
    for index in 0..nodes.len() {
        compute_node_global(
            index,
            nodes,
            poses,
            globals.as_mut_slice(),
            visited.as_mut_slice(),
        );
    }
}

pub fn compute_skin_matrices_in_place(
    scene: &SceneCpu,
    global_matrices: &[Mat4],
    skin_matrices: &mut Vec<Vec<Mat4>>,
) {
    skin_matrices.resize_with(scene.skins.len(), Vec::new);
    for (skin_index, skin) in scene.skins.iter().enumerate() {
        let matrices = &mut skin_matrices[skin_index];
        matrices.resize(skin.joints.len(), Mat4::IDENTITY);
        for (joint_slot, joint_node) in skin.joints.iter().enumerate() {
            let joint_global = global_matrices
                .get(*joint_node)
                .copied()
                .unwrap_or(Mat4::IDENTITY);
            let inverse_bind = skin
                .inverse_bind_mats
                .get(joint_slot)
                .copied()
                .unwrap_or(Mat4::IDENTITY);
            matrices[joint_slot] = joint_global * inverse_bind;
        }
    }
}
