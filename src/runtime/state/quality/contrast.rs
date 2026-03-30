use crate::scene::{ContrastProfile, RenderConfig};

use super::super::RuntimeContrastPreset;

pub(crate) fn apply_runtime_contrast_preset(
    config: &mut RenderConfig,
    preset: RuntimeContrastPreset,
) {
    match preset {
        RuntimeContrastPreset::AdaptiveLow => {
            config.contrast_profile = ContrastProfile::Adaptive;
            config.contrast_floor = 0.08;
            config.contrast_gamma = 1.00;
            config.fog_scale = 1.00;
        }
        RuntimeContrastPreset::AdaptiveNormal => {
            config.contrast_profile = ContrastProfile::Adaptive;
            config.contrast_floor = 0.10;
            config.contrast_gamma = 0.90;
            config.fog_scale = 1.00;
        }
        RuntimeContrastPreset::AdaptiveHigh => {
            config.contrast_profile = ContrastProfile::Adaptive;
            config.contrast_floor = 0.14;
            config.contrast_gamma = 0.78;
            config.fog_scale = 0.80;
        }
        RuntimeContrastPreset::Fixed => {}
    }
}
