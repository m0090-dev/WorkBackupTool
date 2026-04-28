# WorkBackupTool

A simple, lightweight backup utility built with Tauri (Rust), specifically designed for managing 2DCG project files and work-in-progress versions.

This tool was created by the author for personal use to ensure quick and reliable versioning during creative workflows.

## 🔎 Key Features

- **Streamlined UI**: A single-window interface focused on "Backup" and "Restore."
- **Backup Modes**:
  - **Full Copy**: Creates a standard mirror of your file or folder.
  - **Archive**: Compresses data into ZIP/TAR formats with optional password protection.
  - **Differential (Smart)**: Saves disk space by backing up only modified parts (using hdiff). Supports both single files and folders.
- **Generation Management**: Manages backup history in generations. Each generation consists of a **base** (a full copy of the file or folder at that point) plus a stack of **diff files** accumulated on top of it. When the total diff size exceeds a configurable threshold, a new generation is created: a fresh base is made from the current state, and diff files start accumulating again from scratch.
- **Per-Tab Ignore List**: Specify file patterns (e.g. `*.tmp`) to exclude from differential backups, configured independently per project tab.
- **Note System**: Attach memos with priority marks and preset tags to any backup entry.
- **Quick Restore**: Browse your backup history and revert to a specific point in time with one click.
- **Tab Management**: Manage multiple projects simultaneously in a single window.
- **Compact & Tray Mode**: Minimal window or system tray operation for unobtrusive background use.
- **Multi-language**: English and Japanese support.

## ✅ Implementation Status

### Core Backup Functions
- [x] **Full Copy**: Simple duplication of workspace files or folders.
- [x] **Archive**: ZIP and tar.gz formats with optional password protection.
- [x] **Differential (hdiff)**: Supports zstd / lzma2 / none compression methods. Supports both files and folders.
- [x] **Generation Management**: Auto base rotation based on configurable diff size threshold.
- [x] **Per-Tab Ignore List**: Exclude files by pattern (e.g. `*.tmp`) from differential backups, stored per tab in session.json.

### User Interface (UI/UX)
- [x] **Tab Management**: Multiple project tabs with reordering and right-click menu.
- [x] **Backup History**: Browse history with metadata popup (path, note, priority, generation info).
- [x] **Note System**: Memos with priority marks (None / Low / Mid / High) and preset tags, stored as .note files.
- [x] **Search & Filter**: Filter history by filename or note content.
- [x] **Compact Mode**: Minimal window size for unobtrusive use during work.
- [x] **Tray Mode**: Minimize to system tray with quick-execute support.
- [x] **Window Options**: Always on Top and Restore Previous State.
- [x] **Multi-language**: English and Japanese.
- [x] **Advanced Settings**: Schema-driven settings panel with categorized pages.
- [x] **Strict Hash Check**: Optional `-C-all` flag for full checksum verification on restore (note: increases diff file size).

## 🗂 Generation Structure

Differential backups are organized into **generations**. Each generation is a folder containing:

```
wbt_backup_my_project/
└── base1_20260101_120000/       ← generation folder
    ├── my_project.base/         ← full copy of the file/folder at generation start
    ├── my_project.20260101_120500.hdiff.diff
    ├── my_project.20260101_130000.hdiff.diff
    └── my_project.20260101_150000.hdiff.diff
```

- **Base**: A full copy (not a diff) of the target at the time the generation was created.
- **Diff files**: Each backup run after the base produces one diff file, stacked on top of the base.
- **Generation rotation**: When the cumulative diff size exceeds the configured threshold, a new generation folder is created. A new base is made from the current state of the file/folder, and diff files start accumulating again from zero.

Restore always applies a single diff file on top of the base within the same generation.

## 🚀 How to Use

1. **Target**: Select the file or folder you want to back up.
2. **Location**: Set the destination folder (or use the default).
3. **Execute**: Choose your backup mode and click "Execute".
4. **Restore**: Select a previous version from the history list and click "Restore to selected point".

## 🛠 For Developers

This application is built using Tauri (https://github.com/tauri-apps/tauri).

### Prerequisites

- Rust
- Node.js
- Tauri CLI

### Commands

    # Run in development mode
    npm run tauri:dev

    # Build the application
    npm run tauri:build

### External Binaries

This tool uses hdiffpatch (https://github.com/sisong/HDiffPatch) for differential backup and restore.
The binaries are bundled as Tauri sidecar executables located in src-tauri/binaries/ and are automatically placed alongside the app executable upon installation.

## 📦 Distribution Notes

- **Licenses**: This software uses several open-source libraries. You can find the list in CREDITS.md and their full license texts in the licenses/ directory.

## License

This project is licensed under the MIT License - see the LICENSE.md file for details.
Copyright (c) 2024-2026 m0090-dev

For a complete list of third-party licenses and credits, please refer to CREDITS.md.
