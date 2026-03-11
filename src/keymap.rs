use egui::Modifiers;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KeyBind {
    pub key: egui::Key,
    #[serde(default)]
    pub modifiers: Modifiers,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Keymap {
    pub category_keys: Vec<KeyBind>,
    pub skip: KeyBind,
    pub undo: KeyBind,
}

/// Which binding the user is currently remapping.
#[derive(Clone, Debug, PartialEq)]
pub enum BindTarget {
    Category(usize),
    Skip,
    Undo,
}

impl Default for Keymap {
    fn default() -> Self {
        let mut category_keys = Vec::with_capacity(33);

        // 1-9 for first 9 categories
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
            category_keys.push(KeyBind {
                key,
                modifiers: Modifiers::NONE,
                label: format!("{}", i + 1),
            });
        }

        // A-Z skipping S and Z for categories 10+
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
            // S skipped (skip action)
            (egui::Key::T, "T"),
            (egui::Key::U, "U"),
            (egui::Key::V, "V"),
            (egui::Key::W, "W"),
            (egui::Key::X, "X"),
            (egui::Key::Y, "Y"),
            // Z skipped (undo with Ctrl)
        ];
        for (key, label) in letter_keys {
            category_keys.push(KeyBind {
                key,
                modifiers: Modifiers::NONE,
                label: label.to_string(),
            });
        }

        Self {
            category_keys,
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
            return Ok(()); // No config dir, skip silently
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config dir: {e}"))?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize keymap: {e}"))?;
        std::fs::write(&path, json).map_err(|e| format!("Failed to write keymap: {e}"))
    }

    /// Check if a key+modifiers combo is already bound to something.
    /// Returns the action name if a conflict is found.
    /// `exclude` lets you skip the binding currently being edited.
    pub fn has_conflict(
        &self,
        key: egui::Key,
        modifiers: Modifiers,
        exclude: Option<&BindTarget>,
    ) -> Option<String> {
        // Check category keys
        for (i, bind) in self.category_keys.iter().enumerate() {
            if bind.key == key && mods_equal(bind.modifiers, modifiers) {
                if exclude == Some(&BindTarget::Category(i)) {
                    continue;
                }
                return Some(format!("Category {}", i + 1));
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
        None
    }
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
        assert_eq!(km.category_keys.len(), 33);
        assert_eq!(km.skip.key, egui::Key::S);
        assert_eq!(km.skip.modifiers, Modifiers::NONE);
        assert_eq!(km.undo.key, egui::Key::Z);
        assert!(km.undo.modifiers.ctrl);
    }

    #[test]
    fn test_round_trip_serialization() {
        let km = Keymap::default();
        let json = serde_json::to_string(&km).unwrap();
        let km2: Keymap = serde_json::from_str(&json).unwrap();
        assert_eq!(km, km2);
    }

    #[test]
    fn test_conflict_detection() {
        let km = Keymap::default();
        // Key "1" is bound to category 1
        let conflict = km.has_conflict(egui::Key::Num1, Modifiers::NONE, None);
        assert_eq!(conflict, Some("Category 1".to_string()));

        // Key "S" is bound to skip
        let conflict = km.has_conflict(egui::Key::S, Modifiers::NONE, None);
        assert_eq!(conflict, Some("Skip".to_string()));

        // Ctrl+Z is bound to undo
        let conflict = km.has_conflict(egui::Key::Z, Modifiers::CTRL, None);
        assert_eq!(conflict, Some("Undo".to_string()));

        // No conflict for unbound key
        let conflict = km.has_conflict(egui::Key::F1, Modifiers::NONE, None);
        assert_eq!(conflict, None);

        // Excluding the conflicting binding should return None
        let conflict =
            km.has_conflict(egui::Key::Num1, Modifiers::NONE, Some(&BindTarget::Category(0)));
        assert_eq!(conflict, None);
    }

    #[test]
    fn test_corrupt_json_fallback() {
        let result: Result<Keymap, _> = serde_json::from_str("not valid json{{{");
        assert!(result.is_err());
        // Our load() would fall back to default in this case
        let km = result.unwrap_or_default();
        assert_eq!(km.category_keys.len(), 33);
    }

    #[test]
    fn test_category_count_boundary() {
        let km = Keymap::default();
        // With 5 categories, only first 5 bindings are relevant
        assert!(km.category_keys.get(4).is_some());
        // With 40 categories, categories 34-40 (index 33-39) have no binding
        assert!(km.category_keys.get(33).is_none());
    }
}
