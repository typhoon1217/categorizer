use std::path::{Path, PathBuf};
use egui::TextureHandle;
use crate::history::UndoStack;
use crate::keymap::{BindTarget, Keymap};

const UNDO_CAPACITY: usize = 20;

#[derive(Default)]
pub enum FileView {
    #[default]
    Loading,
    Image(TextureHandle),
    Text(String),
    Other { icon: &'static str, size: u64 },
}

pub struct App {
    pub folder: PathBuf,
    pub files: Vec<PathBuf>,
    pub skipped: Vec<PathBuf>,
    pub current_idx: usize,
    pub subdirs: Vec<PathBuf>,
    pub history: UndoStack,
    pub texture_cache: Option<(PathBuf, TextureHandle)>,
    pub file_view: FileView,
    pub status_message: Option<String>, // transient error/info display
    pub keymap: Keymap,
    pub show_keymap_editor: bool,
    pub listening_bind: Option<BindTarget>,
    pub show_new_folder_popup: bool,
    pub new_folder_name: String,
    pub new_folder_error: Option<String>,
}

impl App {
    pub fn new(folder: PathBuf) -> std::io::Result<Self> {
        let files = crate::files::scan_files(&folder)?;
        let subdirs = crate::files::scan_subdirs(&folder)?;
        Ok(Self {
            folder,
            files,
            skipped: Vec::new(),
            current_idx: 0,
            subdirs,
            history: UndoStack::new(UNDO_CAPACITY),
            texture_cache: None,
            file_view: FileView::Loading,
            status_message: None,
            keymap: Keymap::load(),
            show_keymap_editor: false,
            listening_bind: None,
            show_new_folder_popup: false,
            new_folder_name: String::new(),
            new_folder_error: None,
        })
    }

    /// The file currently being viewed. None if queue is empty.
    pub fn current_file(&self) -> Option<&PathBuf> {
        self.files.get(self.current_idx)
    }

    /// Total files remaining (including current).
    pub fn remaining(&self) -> usize {
        self.files.len()
    }

    /// Remove current file from queue and advance. Handles end-of-queue.
    /// Returns the removed file path.
    pub fn remove_current(&mut self) -> Option<PathBuf> {
        if self.files.is_empty() {
            return None;
        }
        let removed = self.files.remove(self.current_idx);
        // If we just removed the last element, clamp idx to 0
        if self.current_idx >= self.files.len() {
            self.current_idx = 0;
        }
        // If main queue is empty, drain skipped back in
        if self.files.is_empty() && !self.skipped.is_empty() {
            self.files = std::mem::take(&mut self.skipped);
            self.current_idx = 0;
        }
        self.file_view = FileView::Loading;
        Some(removed)
    }

    /// Move current file to dest_subdir. Records in undo history.
    pub fn move_current(&mut self, dest_dir: &Path) -> Result<(), String> {
        let src = match self.current_file() {
            Some(f) => f.clone(),
            None => return Err("No file to move".into()),
        };
        let new_path = crate::files::move_file(&src, dest_dir)
            .map_err(|e| format!("Move failed: {e}"))?;
        self.history.push(crate::history::MoveOp {
            from: src.clone(),
            to: new_path,
        });
        self.remove_current();
        self.status_message = None;
        Ok(())
    }

    /// Skip current file — send to end of queue. Not undoable.
    pub fn skip_current(&mut self) {
        if let Some(file) = self.current_file().cloned() {
            self.skipped.push(file);
            self.remove_current();
        }
    }

    /// Undo last move operation.
    pub fn undo(&mut self) -> Result<(), String> {
        let op = match self.history.pop() {
            Some(op) => op,
            None => return Err("Nothing to undo".into()),
        };
        crate::files::move_file(&op.to, op.from.parent().unwrap_or_else(|| &self.folder))
            .map_err(|e| format!("Undo failed: {e}"))?;
        // Reinsert the file at current position
        self.files.insert(self.current_idx, op.from);
        self.file_view = FileView::Loading;
        self.status_message = None;
        Ok(())
    }

    /// Create a new subfolder in the current folder.
    pub fn create_subfolder(&mut self, name: &str) -> Result<(), String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err("Folder name cannot be empty".into());
        }
        let path = self.folder.join(trimmed);
        std::fs::create_dir(&path).map_err(|e| format!("Failed to create folder: {e}"))?;
        self.subdirs =
            crate::files::scan_subdirs(&self.folder).unwrap_or_else(|_| self.subdirs.clone());
        Ok(())
    }

    /// Open a new folder. Resets file state but preserves keymap settings.
    pub fn open_folder(&mut self, new_folder: PathBuf) {
        let keymap = std::mem::take(&mut self.keymap);
        let show_keymap_editor = self.show_keymap_editor;
        let listening_bind = self.listening_bind.take();
        match Self::new(new_folder) {
            Ok(mut new_app) => {
                new_app.keymap = keymap;
                new_app.show_keymap_editor = show_keymap_editor;
                new_app.listening_bind = listening_bind;
                *self = new_app;
            }
            Err(e) => {
                self.keymap = keymap;
                self.show_keymap_editor = show_keymap_editor;
                self.listening_bind = listening_bind;
                self.status_message = Some(format!("Failed to open folder: {e}"));
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        crate::ui::render(self, ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_app() -> (TempDir, App) {
        let tmp = TempDir::new().unwrap();
        // Create a dummy file so the app has something to work with
        std::fs::write(tmp.path().join("test.txt"), "hello").unwrap();
        let app = App::new(tmp.path().to_path_buf()).unwrap();
        (tmp, app)
    }

    #[test]
    fn test_create_subfolder() {
        let (tmp, mut app) = setup_app();
        assert!(app.subdirs.is_empty());

        app.create_subfolder("photos").unwrap();

        assert!(tmp.path().join("photos").is_dir());
        assert_eq!(app.subdirs.len(), 1);
        assert_eq!(
            app.subdirs[0].file_name().unwrap().to_str().unwrap(),
            "photos"
        );
    }

    #[test]
    fn test_create_subfolder_duplicate() {
        let (_tmp, mut app) = setup_app();
        app.create_subfolder("photos").unwrap();

        let result = app.create_subfolder("photos");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to create folder"));
    }

    #[test]
    fn test_create_subfolder_empty_name() {
        let (_tmp, mut app) = setup_app();
        let result = app.create_subfolder("   ");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Folder name cannot be empty");
    }

    #[test]
    fn test_create_subfolder_sorted() {
        let (_tmp, mut app) = setup_app();
        app.create_subfolder("zebra").unwrap();
        app.create_subfolder("alpha").unwrap();
        app.create_subfolder("middle").unwrap();

        let names: Vec<_> = app
            .subdirs
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        assert_eq!(names, vec!["alpha", "middle", "zebra"]);
    }
}
