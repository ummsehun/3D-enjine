use crate::interfaces::tui::helpers::{
    detect_terminal_cell_aspect, inspect_audio_duration, inspect_clip_duration,
};

use super::wizard::StartWizardState;

impl StartWizardState {
    pub(crate) fn refresh_runtime_metrics(&mut self, anim_selector: Option<&str>) {
        self.detected_cell_aspect = detect_terminal_cell_aspect();

        let model_path = self
            .model_entries
            .get(self.model_index)
            .map(|entry| entry.path.clone());
        if let Some(path) = model_path {
            self.clip_duration_cache
                .entry(path.clone())
                .or_insert_with(|| inspect_clip_duration(&path, anim_selector));
        }

        let music_path = self.selected_music_path().cloned();
        if let Some(path) = music_path {
            self.audio_duration_cache
                .entry(path.clone())
                .or_insert_with(|| inspect_audio_duration(&path));
        }
    }
}
