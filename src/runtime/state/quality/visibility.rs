use super::limits::{
    LOW_VIS_EXPOSURE_RECOVER_FRAMES, LOW_VIS_EXPOSURE_RECOVER_THRESHOLD,
    LOW_VIS_EXPOSURE_THRESHOLD, LOW_VIS_EXPOSURE_TRIGGER_FRAMES, MIN_VISIBLE_HEIGHT_RATIO,
    MIN_VISIBLE_HEIGHT_RECOVER_FRAMES, MIN_VISIBLE_HEIGHT_RECOVER_RATIO,
    MIN_VISIBLE_HEIGHT_TRIGGER_FRAMES, VISIBILITY_LOW_FRAMES_TO_RECOVER, VISIBILITY_LOW_THRESHOLD,
};

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct VisibilityWatchdog {
    pub(crate) low_visible_streak: u32,
}

impl VisibilityWatchdog {
    pub(crate) fn observe(&mut self, visible_ratio: f32) -> bool {
        if visible_ratio < VISIBILITY_LOW_THRESHOLD {
            self.low_visible_streak = self.low_visible_streak.saturating_add(1);
        } else {
            self.low_visible_streak = 0;
        }
        self.low_visible_streak >= VISIBILITY_LOW_FRAMES_TO_RECOVER
    }

    pub(crate) fn reset(&mut self) {
        self.low_visible_streak = 0;
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct ExposureAutoBoost {
    pub(crate) low_streak: u32,
    pub(crate) high_streak: u32,
    pub(crate) boost: f32,
}

impl ExposureAutoBoost {
    pub(crate) fn on_resize(&mut self) {
        self.low_streak = 0;
        self.high_streak = 0;
        self.boost = 0.0;
    }

    pub(crate) fn update(&mut self, visible_ratio: f32) {
        if visible_ratio < LOW_VIS_EXPOSURE_THRESHOLD {
            self.low_streak = self.low_streak.saturating_add(1);
            self.high_streak = 0;
            if self.low_streak >= LOW_VIS_EXPOSURE_TRIGGER_FRAMES {
                self.boost = (self.boost + 0.06).clamp(0.0, 0.45);
                self.low_streak = 0;
            }
            return;
        }

        if visible_ratio > LOW_VIS_EXPOSURE_RECOVER_THRESHOLD {
            self.high_streak = self.high_streak.saturating_add(1);
            self.low_streak = 0;
            if self.high_streak >= LOW_VIS_EXPOSURE_RECOVER_FRAMES {
                self.boost = (self.boost - 0.03).clamp(0.0, 0.45);
                self.high_streak = 0;
            }
            return;
        }

        self.low_streak = 0;
        self.high_streak = 0;
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct AutoRadiusGuard {
    pub(crate) low_height_streak: u32,
    pub(crate) recover_streak: u32,
    pub(crate) shrink_ratio: f32,
}

impl AutoRadiusGuard {
    pub(crate) fn update(&mut self, height_ratio: f32, enabled: bool) -> f32 {
        if !enabled {
            self.low_height_streak = 0;
            self.recover_streak = 0;
            self.shrink_ratio = 0.0;
            return 0.0;
        }

        if height_ratio < MIN_VISIBLE_HEIGHT_RATIO {
            self.low_height_streak = self.low_height_streak.saturating_add(1);
            self.recover_streak = 0;
            if self.low_height_streak >= MIN_VISIBLE_HEIGHT_TRIGGER_FRAMES {
                self.shrink_ratio = (self.shrink_ratio + 0.02).clamp(0.0, 0.12);
                self.low_height_streak = 0;
            }
        } else if height_ratio > MIN_VISIBLE_HEIGHT_RECOVER_RATIO {
            self.recover_streak = self.recover_streak.saturating_add(1);
            self.low_height_streak = 0;
            if self.recover_streak >= MIN_VISIBLE_HEIGHT_RECOVER_FRAMES {
                self.shrink_ratio = (self.shrink_ratio - 0.02).clamp(0.0, 0.12);
                self.recover_streak = 0;
            }
        } else {
            self.low_height_streak = 0;
            self.recover_streak = 0;
        }
        self.shrink_ratio
    }
}
