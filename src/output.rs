use std::io::{self, Write};
use std::path::Path;

use tempfile::Builder;

/// Stream to an owner-only temporary file and replace the destination only
/// after the entire operation succeeds. Failure drops the tempfile, leaving
/// the previous destination untouched.
pub fn with_private_output<T>(
    path: &Path,
    write: impl FnOnce(&mut std::fs::File) -> Result<T, String>,
) -> Result<T, String> {
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = Builder::new()
        .prefix(".medea-json-")
        .tempfile_in(directory)
        .map_err(|err| format!("Could not save crypto trades: {err}"))?;
    let result = write(temporary.as_file_mut())?;
    temporary
        .flush()
        .map_err(|err| format!("Could not save crypto trades: {err}"))?;
    temporary
        .persist(path)
        .map_err(|err| format!("Could not save crypto trades: {}", err.error))?;
    Ok(result)
}

pub fn save_private_json(path: &Path, bytes: &[u8]) -> io::Result<()> {
    // tempfile uses an owner-only file on Unix, in the destination directory.
    // Persist renames it over the destination, so a preexisting public file
    // cannot accidentally retain its old permissions.
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = Builder::new()
        .prefix(".medea-json-")
        .tempfile_in(directory)?;
    temporary.write_all(bytes)?;
    temporary.flush()?;
    temporary.persist(path).map_err(|err| err.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::save_private_json;
    use std::fs;

    #[test]
    fn saves_exact_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("account.json");
        let response = br#"{"status":"ACTIVE","extra":123}"#;
        save_private_json(&path, response).unwrap();
        assert_eq!(fs::read(&path).unwrap(), response);
    }

    #[test]
    fn replaces_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("account.json");
        fs::write(&path, "old").unwrap();
        save_private_json(&path, b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
    }

    #[test]
    fn missing_directory_reports_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does-not-exist").join("account.json");
        assert!(save_private_json(&path, b"test").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn replaces_a_public_file_with_private_owner_only_file() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("account.json");
        fs::write(&path, b"old").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        save_private_json(&path, b"new").unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}
