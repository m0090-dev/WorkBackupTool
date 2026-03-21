# WorkBackupTool

A simple, lightweight backup utility built with Tauri (Rust), specifically designed for managing 2DCG project files and work-in-progress versions.

This tool was created by the author for personal use to ensure quick and reliable versioning during creative workflows.

## 🔎 Key Features

- **Streamlined UI**: A single-window interface focused on "Backup" and "Restore."
- **Backup Modes**:
  - **Full Copy**: Creates a standard mirror of your files.
  - **Archive**: Compresses data into ZIP/TAR formats with optional password protection.
  - **Differential (Smart)**: Saves disk space by backing up only modified parts (using hdiff).
- **Generation Management**: Automatically creates a new base and rotates generations when the diff size exceeds a configurable threshold.
- **Note System**: Attach memos with priority marks and preset tags to any backup entry.
- **Quick Restore**: Browse your backup history and revert to a specific point in time with one click.
- **Tab Management**: Manage multiple projects simultaneously in a single window.
- **Compact & Tray Mode**: Minimal window or system tray operation for unobtrusive background use.
- **Multi-language**: English and Japanese support.

## ✅ Implementation Status

### Core Backup Functions
- [x] **Full Copy**: Simple duplication of workspace files.
- [x] **Archive**: ZIP and tar.gz formats with optional password protection.
- [x] **Differential (hdiff)**: Supports zstd / lzma2 / none compression methods.
- [x] **Generation Management**: Auto base rotation based on configurable diff size threshold.

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

## 🚀 How to Use

1. **Target**: Select the file you want to back up.
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
