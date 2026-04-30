use std::{fs, path::Path, path::PathBuf};

use crate::runtime::start_ui_helpers::format_mib;

#[derive(Debug, Clone)]
pub(crate) struct StartEntry {
    pub(crate) path: PathBuf,
    pub(crate) name: String,
    pub(crate) bytes: u64,
}

impl StartEntry {
    pub(crate) fn from_path(path: &Path) -> Self {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<invalid>")
            .to_owned();
        let bytes = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        Self {
            path: path.to_path_buf(),
            name,
            bytes,
        }
    }

    pub(crate) fn label(&self) -> String {
        format!("{} ({})", self.name, format_mib(self.bytes))
    }
}
