use std::path::{Path, PathBuf};
use std::fs;

/// Image file extensions this app can preview.
const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif", "ico",
];

/// Text file extensions this app shows as text preview.
const TEXT_EXTENSIONS: &[&str] = &["txt", "md"];

/// Returns true if the file should be shown as an image.
pub fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Returns true if the file should be shown as a text preview.
pub fn is_text(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| TEXT_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Scans `dir` for top-level regular files only.
/// Excludes: directories, dotfiles (name starts with '.'), symlinks.
/// Returns files sorted alphabetically by filename.
pub fn scan_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let ft = e.file_type().ok();
            let is_file = ft.map(|t| t.is_file()).unwrap_or(false);
            let name = e.file_name();
            let not_hidden = !name.to_string_lossy().starts_with('.');
            is_file && not_hidden
        })
        .map(|e| e.path())
        .collect();
    files.sort_by(|a, b| {
        a.file_name()
            .unwrap_or_default()
            .cmp(b.file_name().unwrap_or_default())
    });
    Ok(files)
}

/// Scans `dir` for top-level subdirectories only.
/// Excludes dotfiles. Returns sorted alphabetically by name.
pub fn scan_subdirs(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let ft = e.file_type().ok();
            let is_dir = ft.map(|t| t.is_dir()).unwrap_or(false);
            let name = e.file_name();
            let not_hidden = !name.to_string_lossy().starts_with('.');
            is_dir && not_hidden
        })
        .map(|e| e.path())
        .collect();
    dirs.sort_by(|a, b| {
        a.file_name()
            .unwrap_or_default()
            .cmp(b.file_name().unwrap_or_default())
    });
    Ok(dirs)
}

/// Moves `src` into `dest_dir`, preserving the filename.
/// Returns the new path on success.
pub fn move_file(src: &Path, dest_dir: &Path) -> std::io::Result<PathBuf> {
    let filename = src
        .file_name()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "no filename"))?;
    let dest = dest_dir.join(filename);
    fs::rename(src, &dest)?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_temp() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn test_is_image_common_extensions() {
        assert!(is_image(Path::new("photo.jpg")));
        assert!(is_image(Path::new("photo.JPEG")));
        assert!(is_image(Path::new("img.PNG")));
        assert!(is_image(Path::new("anim.gif")));
        assert!(is_image(Path::new("icon.ico")));
    }

    #[test]
    fn test_is_image_non_image() {
        assert!(!is_image(Path::new("doc.pdf")));
        assert!(!is_image(Path::new("notes.txt")));
        assert!(!is_image(Path::new("archive.zip")));
        assert!(!is_image(Path::new("noextension")));
    }

    #[test]
    fn test_is_text() {
        assert!(is_text(Path::new("notes.txt")));
        assert!(is_text(Path::new("README.md")));
        assert!(!is_text(Path::new("photo.jpg")));
    }

    #[test]
    fn test_scan_files_returns_only_regular_files() {
        let dir = make_temp();
        fs::write(dir.path().join("file_a.jpg"), b"").unwrap();
        fs::write(dir.path().join("file_b.png"), b"").unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();

        let files = scan_files(dir.path()).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files[0].file_name().unwrap() == "file_a.jpg");
        assert!(files[1].file_name().unwrap() == "file_b.png");
    }

    #[test]
    fn test_scan_files_excludes_dotfiles() {
        let dir = make_temp();
        fs::write(dir.path().join("visible.jpg"), b"").unwrap();
        fs::write(dir.path().join(".hidden"), b"").unwrap();

        let files = scan_files(dir.path()).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].file_name().unwrap() == "visible.jpg");
    }

    #[test]
    fn test_scan_files_sorted_alphabetically() {
        let dir = make_temp();
        fs::write(dir.path().join("zebra.jpg"), b"").unwrap();
        fs::write(dir.path().join("apple.jpg"), b"").unwrap();
        fs::write(dir.path().join("mango.jpg"), b"").unwrap();

        let files = scan_files(dir.path()).unwrap();
        let names: Vec<_> = files.iter().map(|f| f.file_name().unwrap().to_str().unwrap()).collect();
        assert_eq!(names, vec!["apple.jpg", "mango.jpg", "zebra.jpg"]);
    }

    #[test]
    fn test_scan_subdirs() {
        let dir = make_temp();
        fs::create_dir(dir.path().join("cats")).unwrap();
        fs::create_dir(dir.path().join("dogs")).unwrap();
        fs::write(dir.path().join("file.jpg"), b"").unwrap();

        let dirs = scan_subdirs(dir.path()).unwrap();
        assert_eq!(dirs.len(), 2);
        assert!(dirs[0].file_name().unwrap() == "cats");
        assert!(dirs[1].file_name().unwrap() == "dogs");
    }

    #[test]
    fn test_move_file() {
        let dir = make_temp();
        let dest = dir.path().join("subdir");
        fs::create_dir(&dest).unwrap();
        let src = dir.path().join("photo.jpg");
        fs::write(&src, b"fake image data").unwrap();

        let new_path = move_file(&src, &dest).unwrap();

        assert!(!src.exists());
        assert!(new_path.exists());
        assert_eq!(new_path, dest.join("photo.jpg"));
    }
}
