use glam::Mat4;

use crate::scene::SceneCpu;

mod frame;
mod helpers;
#[cfg(test)]
mod tests;

pub use frame::FramePipeline;

pub(crate) trait PhysicsStepper {
    fn step_physics(
        &mut self,
        scene: &SceneCpu,
        poses: &mut [NodePose],
        pre_physics_globals: &[Mat4],
        dt: f32,
    );
}

use crate::scene::NodePose;
