# categorizer

Fast manual file categorizer with image preview. Open a folder, see each file
one at a time, press a number key to move it into a subcategory.

## Features

- Image preview (jpg, jpeg, png, gif, bmp, webp, tiff, ico)
- Text preview for .txt and .md files
- Category buttons for every subdirectory in the current folder
- Keyboard shortcuts: `1`–`9` to categorize, `S` to skip, `Ctrl+Z` to undo
- 20-level undo history
- Skip queue: skipped files reappear after all others are processed
- Cross-platform (Linux, macOS, Windows)

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
3. Press `1`–`9` to move the file to the corresponding subdirectory
4. Press `S` to skip a file (it will reappear after all others)
5. Press `Ctrl+Z` to undo the last move
6. When all files are categorized, a completion screen is shown

## Layout

```
┌─────────────────────────────────────────────────────┐
│ 📂 Open folder…                    file 12/47       │
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
