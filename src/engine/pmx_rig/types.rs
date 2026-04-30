use glam::Vec3;

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
#[derive(Debug, Clone, Copy)]
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
