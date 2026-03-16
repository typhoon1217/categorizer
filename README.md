# categorizer

[![CI](https://github.com/typhoon1217/categorizer/actions/workflows/ci.yml/badge.svg)](https://github.com/typhoon1217/categorizer/actions/workflows/ci.yml)

Fast manual file categorizer with image preview. Open a folder, see each file
one at a time, press a key to move it into a subcategory.

## Download

Pre-built binaries for Linux, macOS (Intel + Apple Silicon), and Windows are available on the [Releases](https://github.com/typhoon1217/categorizer/releases) page.

## Features

- Image preview (jpg, jpeg, png, gif, bmp, webp, tiff, ico)
- Text preview for .txt and .md files
- Category buttons for every subdirectory in the current folder
- **Named keybindings**: each folder gets its own persistent key — adding a new folder never shifts existing bindings
- **Two-tier keymap**: global shortcuts stored in `~/.config/categorizer/keymap.json`, per-folder bindings stored in `<folder>/.categorizer-keys.json`
- Fully remappable keyboard shortcuts via the built-in keybinding editor
- Create new subfolders on-the-fly with `Ctrl+N`
- **History panel**: resizable bottom bar with scrollable thumbnail strip showing moved files and their destination folders (`Ctrl+H`)
- **Image border**: yellow outline around previewed images to distinguish edges from app background
- 20-level undo history (also reflected in history panel)
- Skip queue: skipped files reappear after all others are processed
- Multilingual support (Korean, Chinese, Japanese) via automatic system font detection
- Cross-platform (Linux, macOS, Windows)

## Default Keybindings

| Action | Default Key |
|--------|------------|
| Categorize to folder | `1`–`9`, `A`–`Y` (auto-assigned per folder) |
| Skip | `S` |
| Undo | `Ctrl+Z` |
| New folder | `Ctrl+N` |
| Open folder | `Ctrl+O` |
| Toggle keybinding editor | `Ctrl+K` |
| Toggle history panel | `Ctrl+H` |

All bindings can be remapped via the keybinding editor (`Ctrl+K`).

## Usage

```bash
# Categorize files in a specific directory
categorizer /path/to/photos

# Categorize files in the current directory
categorizer
```

## Install

```bash
cargo install --path .
```

## Build from source

```bash
cargo build --release
./target/release/categorizer /path/to/photos
```

## How it works

1. Run `categorizer` pointing at a folder that contains files and subdirectories
2. The app shows each file one at a time
3. Press the assigned key to move the file to the corresponding subdirectory
4. Press `S` to skip a file (it will reappear after all others)
5. Press `Ctrl+Z` to undo the last move
6. Press `Ctrl+N` to create a new subfolder (it gets the next available key automatically)
7. Press `Ctrl+O` to open a different folder
8. Press `Ctrl+K` to open the keybinding editor
9. Press `Ctrl+H` to toggle the history panel (shows thumbnails of moved files)
10. When all files are categorized, a completion screen is shown

## Layout

```
┌──────────────────────────────────────────────────────────────┐
│ [Ctrl+O] 📂 Open  [Ctrl+K] ⚙  [Ctrl+H] 📜  file 12/47     │
├───────────────────────────────────┬──────────────────────────┤
│                                   │ photo_001.jpg            │
│                                   │ 2.3 MB                   │
│    ┌─────────────────────────┐    │ ─────────────────        │
│    │ Image / text preview    │    │ [1] 📁 animals           │
│    │ (yellow border outline) │    │ [2] 📁 cars              │
│    └─────────────────────────┘    │ [3] 📁 food              │
│                                   │ [4] 📁 people            │
│                                   │ ─────────────────        │
│                                   │ [S]  ⏭ Skip              │
│                                   │ [Ctrl+Z] ↩ Undo          │
├───────────────────────────────────┴──────────────────────────┤
│ 📜 History (3)                                               │
│ [thumb1] [thumb2] [thumb3]  ← horizontal scroll, resizable  │
└──────────────────────────────────────────────────────────────┘
```
