use crate::scene::{MaterialCpu, MaterialMorphCpu, MaterialMorphFormula, MaterialMorphOp};

pub fn apply_material_morphs(
    base: &MaterialCpu,
    morphs: &[MaterialMorphCpu],
    weights: &[f32],
) -> MaterialCpu {
    if morphs.is_empty() || weights.is_empty() {
        return base.clone();
    }

    let mut result = base.clone();

    for (morph_index, morph) in morphs.iter().enumerate() {
        let weight = weights.get(morph_index).copied().unwrap_or(0.0);
        if weight.abs() < 1e-6 {
            continue;
        }

        for op in &morph.operations {
            let target_index = op.target_material_index;
            if target_index >= 0 {
                continue;
            }

            apply_morph_op(&mut result, op, weight);
        }
    }

    result
}

pub fn apply_material_morph_to_index(
    base: &MaterialCpu,
    material_index: usize,
    morphs: &[MaterialMorphCpu],
    weights: &[f32],
) -> MaterialCpu {
    if morphs.is_empty() || weights.is_empty() {
        return base.clone();
    }

    let mut result = base.clone();

    for (morph_index, morph) in morphs.iter().enumerate() {
        let weight = weights.get(morph_index).copied().unwrap_or(0.0);
        if weight.abs() < 1e-6 {
            continue;
        }

        for op in &morph.operations {
            if op.target_material_index >= 0 && op.target_material_index as usize != material_index
            {
                continue;
            }

            apply_morph_op(&mut result, op, weight);
        }
    }

    result
}

fn apply_morph_op(material: &mut MaterialCpu, op: &MaterialMorphOp, weight: f32) {
    match op.formula {
        MaterialMorphFormula::Multiply => {
            for i in 0..4 {
                material.base_color_factor[i] *= 1.0 + op.diffuse[i] * weight;
            }
            for i in 0..3 {
                material.emissive_factor[i] += op.ambient[i] * weight;
            }
        }
        MaterialMorphFormula::Add => {
            for i in 0..4 {
                material.base_color_factor[i] += op.diffuse[i] * weight;
            }
            for i in 0..3 {
                material.emissive_factor[i] += op.ambient[i] * weight;
            }
        }
    }

    for i in 0..4 {
        material.base_color_factor[i] = material.base_color_factor[i].clamp(0.0, 1.0);
    }
    for i in 0..3 {
        material.emissive_factor[i] = material.emissive_factor[i].clamp(0.0, 1.0);
    }
}
