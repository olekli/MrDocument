pub fn file_exists(path: &std::path::PathBuf) -> bool {
    if let Ok(exists) = std::fs::exists(&path) {
        exists
    } else {
        false
    }
}
