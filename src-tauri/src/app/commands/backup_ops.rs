// 標準ライブラリ
use std::fs;
use std::path::{Path, PathBuf};

// 外部クレート
use chrono::{DateTime, Local};
use tauri::{AppHandle, LogicalSize, Manager, Size, State, WebviewWindow, Window};

// Tauriプラグイン
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_shell::ShellExt;

// 内部モジュール (自作)
use crate::app::hdiff::*;
use crate::app::state::AppState;
use crate::core::ext::hdiff_common::*;
use crate::core::types::BackupItem;
use crate::core::types::*;
use crate::core::{backup::auto_generation, utils};
use flate2::read::GzDecoder;
use regex::Regex;
use std::collections::HashMap;
use std::fs::File;
use tar::Archive;
use zip::ZipArchive;
