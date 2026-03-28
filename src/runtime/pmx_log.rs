use std::{fs, fs::OpenOptions, io::Write, path::Path, path::PathBuf};

const PMX_LOG_RELATIVE_PATH: &str = "log/pmx_Log.txt";

pub fn info(message: impl AsRef<str>) {
    let _ = append_line(message.as_ref());
}

pub fn warn(message: impl AsRef<str>) {
    eprintln!("{}", message.as_ref());
    let _ = append_line(message.as_ref());
}

pub fn error(message: impl AsRef<str>) {
    eprintln!("{}", message.as_ref());
    let _ = append_line(message.as_ref());
}

pub fn append_line_at(root: impl AsRef<Path>, message: impl AsRef<str>) -> std::io::Result<()> {
    let log_path = root.as_ref().join(PMX_LOG_RELATIVE_PATH);
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    writeln!(file, "{}", message.as_ref())
}

fn append_line(message: &str) -> std::io::Result<()> {
    append_line_at(PathBuf::from("."), message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn append_line_at_creates_log_directory_and_appends() {
        let temp = tempdir().expect("tempdir");
        append_line_at(temp.path(), "first line").expect("write first line");
        append_line_at(temp.path(), "second line").expect("write second line");

        let log_path = temp.path().join(PMX_LOG_RELATIVE_PATH);
        let contents = fs::read_to_string(log_path).expect("read log file");
        assert_eq!(contents, "first line\nsecond line\n");
    }
}
