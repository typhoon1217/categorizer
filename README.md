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
- 20-level undo history
- Skip queue: skipped files reappear after all others are processed
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
9. When all files are categorized, a completion screen is shown

## Layout

```
┌─────────────────────────────────────────────────────┐
│ [Ctrl+O] 📂 Open folder…  [Ctrl+K] ⚙  file 12/47  │
├──────────────────────────────┬──────────────────────┤
│                              │ photo_001.jpg        │
│                              │ 2.3 MB               │
│      Image / text preview    │ ─────────────────    │
│      (scales to fit)         │ [1] 📁 animals       │
│                              │ [2] 📁 cars           │
│                              │ [3] 📁 food           │
│                              │ [4] 📁 people         │
│                              │ ─────────────────    │
│                              │ [S]  ⏭ Skip          │
│                              │ [Ctrl+Z] ↩ Undo      │
└──────────────────────────────┴──────────────────────┘
```
