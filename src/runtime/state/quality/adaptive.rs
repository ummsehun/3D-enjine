use crate::scene::PerfProfile;

#[derive(Debug, Clone, Copy)]
pub(crate) struct RuntimeAdaptiveQuality {
    pub(crate) target_frame_ms: f32,
    pub(crate) ema_frame_ms: f32,
    pub(crate) lod_level: usize,
    pub(crate) overload_streak: u32,
    pub(crate) underload_streak: u32,
}

impl RuntimeAdaptiveQuality {
    pub(crate) fn new(profile: PerfProfile) -> Self {
        Self {
            target_frame_ms: target_frame_ms(profile),
            ema_frame_ms: target_frame_ms(profile),
            lod_level: 0,
            overload_streak: 0,
            underload_streak: 0,
        }
    }

    pub(crate) fn observe(&mut self, frame_ms: f32) -> bool {
        self.ema_frame_ms += (frame_ms - self.ema_frame_ms) * 0.12;
        let high = self.target_frame_ms * 1.18;
        let low = self.target_frame_ms * 0.82;
        let mut changed = false;

        if self.ema_frame_ms > high {
            self.overload_streak = self.overload_streak.saturating_add(1);
            self.underload_streak = 0;
            if self.overload_streak >= 20 && self.lod_level < 2 {
                self.lod_level += 1;
                self.overload_streak = 0;
                changed = true;
            }
        } else if self.ema_frame_ms < low {
            self.underload_streak = self.underload_streak.saturating_add(1);
            self.overload_streak = 0;
            if self.underload_streak >= 60 && self.lod_level > 0 {
                self.lod_level -= 1;
                self.underload_streak = 0;
                changed = true;
            }
        } else {
            self.overload_streak = 0;
            self.underload_streak = 0;
        }
        changed
    }
}

pub(crate) fn target_frame_ms(profile: PerfProfile) -> f32 {
    match profile {
        PerfProfile::Balanced => 33.3,
        PerfProfile::Cinematic => 50.0,
        PerfProfile::Smooth => 22.2,
    }
}
