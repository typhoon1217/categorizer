use egui::Modifiers;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KeyBind {
    pub key: egui::Key,
    #[serde(default)]
    pub modifiers: Modifiers,
    pub label: String,
}

/// Global keymap — stored at ~/.config/categorizer/keymap.json
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Keymap {
    pub skip: KeyBind,
    pub undo: KeyBind,
    pub new_folder: KeyBind,
    pub open_folder: KeyBind,
    pub toggle_keybindings: KeyBind,
}

/// Which binding the user is currently remapping.
#[derive(Clone, Debug, PartialEq)]
pub enum BindTarget {
    Category(String),
    Skip,
    Undo,
    NewFolder,
    OpenFolder,
    ToggleKeybindings,
}

/// Per-folder key bindings — stored at <folder>/.categorizer-keys.json
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FolderBindings(pub HashMap<String, KeyBind>);

impl Default for Keymap {
    fn default() -> Self {
        Self {
            skip: KeyBind {
                key: egui::Key::S,
                modifiers: Modifiers::NONE,
                label: "S".to_string(),
            },
            undo: KeyBind {
                key: egui::Key::Z,
                modifiers: Modifiers::CTRL,
                label: "Ctrl+Z".to_string(),
            },
            new_folder: KeyBind {
                key: egui::Key::N,
                modifiers: Modifiers::CTRL,
                label: "Ctrl+N".to_string(),
            },
            open_folder: KeyBind {
                key: egui::Key::O,
                modifiers: Modifiers::CTRL,
                label: "Ctrl+O".to_string(),
            },
            toggle_keybindings: KeyBind {
                key: egui::Key::K,
                modifiers: Modifiers::CTRL,
                label: "Ctrl+K".to_string(),
            },
        }
    }
}

impl Keymap {
    fn config_path() -> Option<PathBuf> {
        let dir = dirs::config_dir()?.join("categorizer");
        Some(dir.join("keymap.json"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        let Ok(data) = std::fs::read_to_string(&path) else {
            return Self::default();
        };
        serde_json::from_str(&data).unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let Some(path) = Self::config_path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config dir: {e}"))?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize keymap: {e}"))?;
        std::fs::write(&path, json).map_err(|e| format!("Failed to write keymap: {e}"))
    }

    /// Check if a key+modifiers combo conflicts with a global binding.
    /// Returns the action name if a conflict is found.
    pub fn has_conflict(
        &self,
        key: egui::Key,
        modifiers: Modifiers,
        exclude: Option<&BindTarget>,
        folder_bindings: &FolderBindings,
    ) -> Option<String> {
        // Check folder bindings
        for (name, bind) in &folder_bindings.0 {
            if bind.key == key && mods_equal(bind.modifiers, modifiers) {
                if exclude == Some(&BindTarget::Category(name.clone())) {
                    continue;
                }
                return Some(format!("Folder '{name}'"));
            }
        }
        // Check skip
        if self.skip.key == key
            && mods_equal(self.skip.modifiers, modifiers)
            && exclude != Some(&BindTarget::Skip)
        {
            return Some("Skip".to_string());
        }
        // Check undo
        if self.undo.key == key
            && mods_equal(self.undo.modifiers, modifiers)
            && exclude != Some(&BindTarget::Undo)
        {
            return Some("Undo".to_string());
        }
        // Check new_folder
        if self.new_folder.key == key
            && mods_equal(self.new_folder.modifiers, modifiers)
            && exclude != Some(&BindTarget::NewFolder)
        {
            return Some("New folder".to_string());
        }
        // Check open_folder
        if self.open_folder.key == key
            && mods_equal(self.open_folder.modifiers, modifiers)
            && exclude != Some(&BindTarget::OpenFolder)
        {
            return Some("Open folder".to_string());
        }
        // Check toggle_keybindings
        if self.toggle_keybindings.key == key
            && mods_equal(self.toggle_keybindings.modifiers, modifiers)
            && exclude != Some(&BindTarget::ToggleKeybindings)
        {
            return Some("Toggle keybindings".to_string());
        }
        None
    }

    /// All global bindings as a list for conflict checking.
    fn global_keys(&self) -> Vec<(egui::Key, Modifiers)> {
        vec![
            (self.skip.key, self.skip.modifiers),
            (self.undo.key, self.undo.modifiers),
            (self.new_folder.key, self.new_folder.modifiers),
            (self.open_folder.key, self.open_folder.modifiers),
            (self.toggle_keybindings.key, self.toggle_keybindings.modifiers),
        ]
    }
}

impl FolderBindings {
    fn file_path(folder: &Path) -> PathBuf {
        folder.join(".categorizer-keys.json")
    }

    pub fn load(folder: &Path) -> Self {
        let path = Self::file_path(folder);
        let Ok(data) = std::fs::read_to_string(&path) else {
            return Self::default();
        };
        serde_json::from_str(&data).unwrap_or_default()
    }

    pub fn save(&self, folder: &Path) -> Result<(), String> {
        let path = Self::file_path(folder);
        let json = serde_json::to_string_pretty(&self)
            .map_err(|e| format!("Failed to serialize folder bindings: {e}"))?;
        std::fs::write(&path, json)
            .map_err(|e| format!("Failed to write folder bindings: {e}"))
    }

    pub fn delete(folder: &Path) {
        let path = Self::file_path(folder);
        let _ = std::fs::remove_file(path);
    }

    pub fn get(&self, name: &str) -> Option<&KeyBind> {
        self.0.get(name)
    }

    /// Auto-assign keys to any unbound subdirs from the default pool,
    /// skipping keys already used by the global keymap or other folder bindings.
    pub fn ensure_bound(&mut self, subdirs: &[PathBuf], keymap: &Keymap) {
        let pool = default_key_pool();

        for subdir in subdirs {
            let name = subdir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            if self.0.contains_key(&name) {
                continue;
            }
            // Find next available key
            if let Some(bind) = self.next_free_key(&pool, keymap) {
                self.0.insert(name, bind);
            }
        }
    }

    /// Assign a key to a single new folder name.
    pub fn assign_key(&mut self, name: &str, keymap: &Keymap) {
        if self.0.contains_key(name) {
            return;
        }
        let pool = default_key_pool();
        if let Some(bind) = self.next_free_key(&pool, keymap) {
            self.0.insert(name.to_string(), bind);
        }
    }

    fn next_free_key(&self, pool: &[KeyBind], keymap: &Keymap) -> Option<KeyBind> {
        let global_keys = keymap.global_keys();
        for candidate in pool {
            // Skip if used by global keymap
            if global_keys
                .iter()
                .any(|(k, m)| *k == candidate.key && mods_equal(*m, candidate.modifiers))
            {
                continue;
            }
            // Skip if already used by another folder binding
            if self
                .0
                .values()
                .any(|b| b.key == candidate.key && mods_equal(b.modifiers, candidate.modifiers))
            {
                continue;
            }
            return Some(candidate.clone());
        }
        None
    }
}

/// The default pool of keys for auto-assigning to folders: 1-9, then A-Z (skipping S, Z).
pub fn default_key_pool() -> Vec<KeyBind> {
    let mut pool = Vec::with_capacity(33);

    let num_keys = [
        egui::Key::Num1,
        egui::Key::Num2,
        egui::Key::Num3,
        egui::Key::Num4,
        egui::Key::Num5,
        egui::Key::Num6,
        egui::Key::Num7,
        egui::Key::Num8,
        egui::Key::Num9,
    ];
    for (i, &key) in num_keys.iter().enumerate() {
        pool.push(KeyBind {
            key,
            modifiers: Modifiers::NONE,
            label: format!("{}", i + 1),
        });
    }

    let letter_keys = [
        (egui::Key::A, "A"),
        (egui::Key::B, "B"),
        (egui::Key::C, "C"),
        (egui::Key::D, "D"),
        (egui::Key::E, "E"),
        (egui::Key::F, "F"),
        (egui::Key::G, "G"),
        (egui::Key::H, "H"),
        (egui::Key::I, "I"),
        (egui::Key::J, "J"),
        (egui::Key::K, "K"),
        (egui::Key::L, "L"),
        (egui::Key::M, "M"),
        (egui::Key::N, "N"),
        (egui::Key::O, "O"),
        (egui::Key::P, "P"),
        (egui::Key::Q, "Q"),
        (egui::Key::R, "R"),
        // S skipped (default skip key)
        (egui::Key::T, "T"),
        (egui::Key::U, "U"),
        (egui::Key::V, "V"),
        (egui::Key::W, "W"),
        (egui::Key::X, "X"),
        (egui::Key::Y, "Y"),
        // Z skipped (default undo with Ctrl)
    ];
    for (key, label) in letter_keys {
        pool.push(KeyBind {
            key,
            modifiers: Modifiers::NONE,
            label: label.to_string(),
        });
    }

    pool
}

fn mods_equal(a: Modifiers, b: Modifiers) -> bool {
    a.ctrl == b.ctrl && a.shift == b.shift && a.alt == b.alt
}

/// Build a label string from key + modifiers.
pub fn format_key_label(key: egui::Key, modifiers: Modifiers) -> String {
    let mut parts = Vec::new();
    if modifiers.ctrl {
        parts.push("Ctrl");
    }
    if modifiers.alt {
        parts.push("Alt");
    }
    if modifiers.shift {
        parts.push("Shift");
    }
    parts.push(key_name(key));
    parts.join("+")
}

fn key_name(key: egui::Key) -> &'static str {
    match key {
        egui::Key::Num0 => "0",
        egui::Key::Num1 => "1",
        egui::Key::Num2 => "2",
        egui::Key::Num3 => "3",
        egui::Key::Num4 => "4",
        egui::Key::Num5 => "5",
        egui::Key::Num6 => "6",
        egui::Key::Num7 => "7",
        egui::Key::Num8 => "8",
        egui::Key::Num9 => "9",
        egui::Key::A => "A",
        egui::Key::B => "B",
        egui::Key::C => "C",
        egui::Key::D => "D",
        egui::Key::E => "E",
        egui::Key::F => "F",
        egui::Key::G => "G",
        egui::Key::H => "H",
        egui::Key::I => "I",
        egui::Key::J => "J",
        egui::Key::K => "K",
        egui::Key::L => "L",
        egui::Key::M => "M",
        egui::Key::N => "N",
        egui::Key::O => "O",
        egui::Key::P => "P",
        egui::Key::Q => "Q",
        egui::Key::R => "R",
        egui::Key::S => "S",
        egui::Key::T => "T",
        egui::Key::U => "U",
        egui::Key::V => "V",
        egui::Key::W => "W",
        egui::Key::X => "X",
        egui::Key::Y => "Y",
        egui::Key::Z => "Z",
        egui::Key::Space => "Space",
        egui::Key::Enter => "Enter",
        egui::Key::Tab => "Tab",
        egui::Key::Backspace => "Backspace",
        egui::Key::Delete => "Delete",
        egui::Key::ArrowUp => "Up",
        egui::Key::ArrowDown => "Down",
        egui::Key::ArrowLeft => "Left",
        egui::Key::ArrowRight => "Right",
        egui::Key::Home => "Home",
        egui::Key::End => "End",
        egui::Key::PageUp => "PageUp",
        egui::Key::PageDown => "PageDown",
        egui::Key::F1 => "F1",
        egui::Key::F2 => "F2",
        egui::Key::F3 => "F3",
        egui::Key::F4 => "F4",
        egui::Key::F5 => "F5",
        egui::Key::F6 => "F6",
        egui::Key::F7 => "F7",
        egui::Key::F8 => "F8",
        egui::Key::F9 => "F9",
        egui::Key::F10 => "F10",
        egui::Key::F11 => "F11",
        egui::Key::F12 => "F12",
        egui::Key::Minus => "-",
        egui::Key::Plus => "+",
        egui::Key::Equals => "=",
        egui::Key::OpenBracket => "[",
        egui::Key::CloseBracket => "]",
        egui::Key::Backslash => "\\",
        egui::Key::Semicolon => ";",
        egui::Key::Colon => ":",
        egui::Key::Comma => ",",
        egui::Key::Period => ".",
        egui::Key::Slash => "/",
        egui::Key::Pipe => "|",
        egui::Key::Questionmark => "?",
        egui::Key::Quote => "'",
        egui::Key::Backtick => "`",
        _ => "?",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_keymap_structure() {
        let km = Keymap::default();
        assert_eq!(km.skip.key, egui::Key::S);
        assert_eq!(km.skip.modifiers, Modifiers::NONE);
        assert_eq!(km.undo.key, egui::Key::Z);
        assert!(km.undo.modifiers.ctrl);
        assert_eq!(km.new_folder.key, egui::Key::N);
        assert!(km.new_folder.modifiers.ctrl);
        assert_eq!(km.open_folder.key, egui::Key::O);
        assert!(km.open_folder.modifiers.ctrl);
        assert_eq!(km.toggle_keybindings.key, egui::Key::K);
        assert!(km.toggle_keybindings.modifiers.ctrl);
    }

    #[test]
    fn test_round_trip_serialization() {
        let km = Keymap::default();
        let json = serde_json::to_string(&km).unwrap();
        let km2: Keymap = serde_json::from_str(&json).unwrap();
        assert_eq!(km, km2);
    }

    #[test]
    fn test_folder_bindings_round_trip() {
        let mut fb = FolderBindings::default();
        fb.0.insert(
            "photos".to_string(),
            KeyBind {
                key: egui::Key::Num1,
                modifiers: Modifiers::NONE,
                label: "1".to_string(),
            },
        );
        let json = serde_json::to_string(&fb).unwrap();
        let fb2: FolderBindings = serde_json::from_str(&json).unwrap();
        assert_eq!(fb.0.len(), fb2.0.len());
        assert_eq!(fb.0["photos"], fb2.0["photos"]);
    }

    #[test]
    fn test_ensure_bound_assigns_keys() {
        let km = Keymap::default();
        let mut fb = FolderBindings::default();
        let subdirs = vec![
            PathBuf::from("/tmp/cats"),
            PathBuf::from("/tmp/dogs"),
            PathBuf::from("/tmp/birds"),
        ];
        fb.ensure_bound(&subdirs, &km);
        assert_eq!(fb.0.len(), 3);
        assert!(fb.0.contains_key("cats"));
        assert!(fb.0.contains_key("dogs"));
        assert!(fb.0.contains_key("birds"));
        // First three should be 1, 2, 3
        assert_eq!(fb.0["cats"].key, egui::Key::Num1);
        assert_eq!(fb.0["dogs"].key, egui::Key::Num2);
        assert_eq!(fb.0["birds"].key, egui::Key::Num3);
    }

    #[test]
    fn test_ensure_bound_skips_already_bound() {
        let km = Keymap::default();
        let mut fb = FolderBindings::default();
        fb.0.insert(
            "cats".to_string(),
            KeyBind {
                key: egui::Key::Q,
                modifiers: Modifiers::NONE,
                label: "Q".to_string(),
            },
        );
        let subdirs = vec![
            PathBuf::from("/tmp/cats"),
            PathBuf::from("/tmp/dogs"),
        ];
        fb.ensure_bound(&subdirs, &km);
        // cats keeps its Q binding
        assert_eq!(fb.0["cats"].key, egui::Key::Q);
        // dogs gets 1 (first available)
        assert_eq!(fb.0["dogs"].key, egui::Key::Num1);
    }

    #[test]
    fn test_ensure_bound_skips_global_conflicts() {
        // S is used by skip (no modifier), so it should be skipped in the pool
        // The default pool already skips S, so let's test with a custom global
        let mut km = Keymap::default();
        km.skip.key = egui::Key::Num1; // override skip to use key 1
        km.skip.modifiers = Modifiers::NONE;
        let mut fb = FolderBindings::default();
        let subdirs = vec![PathBuf::from("/tmp/cats")];
        fb.ensure_bound(&subdirs, &km);
        // Should skip 1 (global conflict) and assign 2
        assert_eq!(fb.0["cats"].key, egui::Key::Num2);
    }

    #[test]
    fn test_new_subfolder_gets_binding_without_shifting() {
        let km = Keymap::default();
        let mut fb = FolderBindings::default();
        let subdirs = vec![
            PathBuf::from("/tmp/alpha"),
            PathBuf::from("/tmp/beta"),
        ];
        fb.ensure_bound(&subdirs, &km);
        let alpha_key = fb.0["alpha"].key;
        let beta_key = fb.0["beta"].key;

        // Now add a new folder
        fb.assign_key("gamma", &km);
        // Existing bindings unchanged
        assert_eq!(fb.0["alpha"].key, alpha_key);
        assert_eq!(fb.0["beta"].key, beta_key);
        // New folder gets next key
        assert!(fb.0.contains_key("gamma"));
        assert_eq!(fb.0["gamma"].key, egui::Key::Num3);
    }

    #[test]
    fn test_conflict_detection_global() {
        let km = Keymap::default();
        let fb = FolderBindings::default();

        // S is bound to skip
        let conflict = km.has_conflict(egui::Key::S, Modifiers::NONE, None, &fb);
        assert_eq!(conflict, Some("Skip".to_string()));

        // Ctrl+Z is bound to undo
        let conflict = km.has_conflict(egui::Key::Z, Modifiers::CTRL, None, &fb);
        assert_eq!(conflict, Some("Undo".to_string()));

        // Ctrl+O is bound to open_folder
        let conflict = km.has_conflict(egui::Key::O, Modifiers::CTRL, None, &fb);
        assert_eq!(conflict, Some("Open folder".to_string()));

        // Ctrl+K is bound to toggle_keybindings
        let conflict = km.has_conflict(egui::Key::K, Modifiers::CTRL, None, &fb);
        assert_eq!(conflict, Some("Toggle keybindings".to_string()));

        // F1 has no conflict
        let conflict = km.has_conflict(egui::Key::F1, Modifiers::NONE, None, &fb);
        assert_eq!(conflict, None);
    }

    #[test]
    fn test_conflict_detection_folder_bindings() {
        let km = Keymap::default();
        let mut fb = FolderBindings::default();
        fb.0.insert(
            "photos".to_string(),
            KeyBind {
                key: egui::Key::Num1,
                modifiers: Modifiers::NONE,
                label: "1".to_string(),
            },
        );

        let conflict = km.has_conflict(egui::Key::Num1, Modifiers::NONE, None, &fb);
        assert_eq!(conflict, Some("Folder 'photos'".to_string()));

        // Excluding the category should pass
        let conflict = km.has_conflict(
            egui::Key::Num1,
            Modifiers::NONE,
            Some(&BindTarget::Category("photos".to_string())),
            &fb,
        );
        assert_eq!(conflict, None);
    }

    #[test]
    fn test_corrupt_json_fallback() {
        let result: Result<Keymap, _> = serde_json::from_str("not valid json{{{");
        assert!(result.is_err());
        let km = result.unwrap_or_default();
        assert_eq!(km.skip.key, egui::Key::S);
    }

    #[test]
    fn test_default_key_pool_size() {
        let pool = default_key_pool();
        assert_eq!(pool.len(), 33); // 9 nums + 24 letters (minus S, Z)
    }
}
