use glam::Mat4;

use crate::scene::MeshInstance;

use super::super::{constants::MAX_JOINTS, device::GpuContext, pipeline::GpuPipeline};

pub(super) fn upload_joint_matrices(
    ctx: &GpuContext,
    pipeline: &GpuPipeline,
    skin_matrices: &[Vec<Mat4>],
    instance: &MeshInstance,
) -> bool {
    let mut joint_matrix_data = vec![0.0f32; MAX_JOINTS * 16];
    let has_skin = if let Some(skin_idx) = instance.skin_index {
        if let Some(joints) = skin_matrices.get(skin_idx) {
            for (i, mat) in joints.iter().take(MAX_JOINTS).enumerate() {
                let offset = i * 16;
                joint_matrix_data[offset..offset + 16].copy_from_slice(&mat.to_cols_array());
            }
            !joints.is_empty()
        } else {
            false
        }
    } else {
        false
    };

    ctx.queue.write_buffer(
        &pipeline.joint_matrix_buffer,
        0,
        bytemuck::cast_slice(&joint_matrix_data),
    );

    has_skin
}
