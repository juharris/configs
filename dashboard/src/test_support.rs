use std::fs;
use std::path::Path;

use tempfile::TempDir;

/// Grants the owner permission to run a fake executable written by a test.
#[cfg(unix)]
pub fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(path, permissions).unwrap();
}

#[cfg(not(unix))]
pub fn make_executable(_path: &Path) {
    panic!("Personal Dashboard requires Bash and a Unix process model");
}

/// Creates a temporary directory inside the crate's `target` directory.
pub fn workspace_tempdir() -> TempDir {
    tempfile::tempdir_in(Path::new(env!("CARGO_MANIFEST_DIR")).join("target")).unwrap()
}
