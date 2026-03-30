use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PmxRigidShape {
    Sphere,
    Box,
    Capsule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PmxRigidCalcMethod {
    Static,
    Dynamic,
    DynamicWithBonePosition,
}

#[derive(Debug, Clone)]
pub struct PmxRigidBodyCpu {
    pub name: String,
    pub name_en: String,
    pub bone_index: i32,
    pub group: u8,
    pub un_collision_group_flag: u16,
    pub form: PmxRigidShape,
    pub size: Vec3,
    pub position: Vec3,
    pub rotation: Vec3,
    pub mass: f32,
    pub move_resist: f32,
    pub rotation_resist: f32,
    pub repulsion: f32,
    pub friction: f32,
    pub calc_method: PmxRigidCalcMethod,
}

#[derive(Debug, Clone)]
pub enum PmxJointKind {
    Spring6Dof {
        a_rigid_index: i32,
        b_rigid_index: i32,
        position: Vec3,
        rotation: Vec3,
        move_limit_down: Vec3,
        move_limit_up: Vec3,
        rotation_limit_down: Vec3,
        rotation_limit_up: Vec3,
        spring_const_move: Vec3,
        spring_const_rotation: Vec3,
    },
    SixDof {
        a_rigid_index: i32,
        b_rigid_index: i32,
        position: Vec3,
        rotation: Vec3,
        move_limit_down: Vec3,
        move_limit_up: Vec3,
        rotation_limit_down: Vec3,
        rotation_limit_up: Vec3,
    },
    P2P {
        a_rigid_index: i32,
        b_rigid_index: i32,
        position: Vec3,
        rotation: Vec3,
    },
    ConeTwist {
        a_rigid_index: i32,
        b_rigid_index: i32,
        swing_span1: f32,
        swing_span2: f32,
        twist_span: f32,
        softness: f32,
        bias_factor: f32,
        relaxation_factor: f32,
        damping: f32,
        fix_thresh: f32,
        enable_motor: bool,
        max_motor_impulse: f32,
        motor_target_in_constraint_space: Vec3,
    },
    Slider {
        a_rigid_index: i32,
        b_rigid_index: i32,
        lower_linear_limit: f32,
        upper_linear_limit: f32,
        lower_angle_limit: f32,
        upper_angle_limit: f32,
        power_linear_motor: bool,
        target_linear_motor_velocity: f32,
        max_linear_motor_force: f32,
        power_angler_motor: bool,
        target_angler_motor_velocity: f32,
        max_angler_motor_force: f32,
    },
    Hinge {
        a_rigid_index: i32,
        b_rigid_index: i32,
        low: f32,
        high: f32,
        softness: f32,
        bias_factor: f32,
        relaxation_factor: f32,
        enable_motor: bool,
        target_velocity: f32,
        max_motor_impulse: f32,
    },
}

#[derive(Debug, Clone)]
pub struct PmxJointCpu {
    pub name: String,
    pub name_en: String,
    pub kind: PmxJointKind,
}

#[derive(Debug, Clone, Default)]
pub struct PmxPhysicsMeta {
    pub rigid_bodies: Vec<PmxRigidBodyCpu>,
    pub joints: Vec<PmxJointCpu>,
}

impl PmxPhysicsMeta {
    pub fn is_empty(&self) -> bool {
        self.rigid_bodies.is_empty() && self.joints.is_empty()
    }
}
