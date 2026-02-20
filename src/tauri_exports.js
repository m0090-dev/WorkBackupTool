import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/**
 * WailsのEventsOnと互換性のあるイベントリスナー
 */
export async function EventsOn(eventName, callback) {
  // listenはPromiseを返すので、awaitしてunlisten関数を取得できるようにします
  return await listen(eventName, (event) => {
    // event.payload に Rust から emit されたデータが入っています
    if (callback && typeof callback === "function") {
      callback(event.payload);
    }
  });
}

/**
 * Wails互換のOnFileDrop
 * @param {function} callback - (x, y, paths) => { ... } 形式の関数
 */
export async function OnFileDrop(callback) {
  // 'tauri://drag-drop' イベントを監視
  return await listen("tauri://drag-drop", (event) => {
    // event.payload の構造: { paths: string[], position: { x: number, y: number } }
    const { paths, position } = event.payload;

    if (callback && typeof callback === "function") {
      // Wailsの引数順 (x, y, paths) に合わせて実行
      callback(position.x, position.y, paths);
    }
  });
}

/**
 * リソース・設定取得系
 */
export async function GetI18N() {
  return await invoke("get_i18n");
}

export async function GetConfigDir() {
  return await invoke("get_config_dir");
}

export async function GetRestorePreviousState() {
  return await invoke("get_restore_previous_state");
}

export async function GetBsdiffMaxFileSize() {
  return await invoke("get_bsdiff_max_file_size");
}

/**
 * ファイル・フォルダ選択系
 */
export async function SelectAnyFile(title, filters) {
  return await invoke("select_any_file", { title, filters });
}

export async function SelectBackupFolder() {
  return await invoke("select_backup_folder");
}

/**
 * テキストファイル操作系
 */
export async function WriteTextFile(path, content) {
  return await invoke("write_text_file", { path, content });
}

export async function ReadTextFile(path) {
  return await invoke("read_text_file", { path });
}

/**
 * バックアップ・ファイル操作系
 */
export async function GetFileSize(path) {
  return await invoke("get_file_size", { path });
}

export async function DirExists(path) {
  return await invoke("dir_exists", { path });
}

export async function CopyBackupFile(src, backupDir) {
  return await invoke("copy_backup_file", { src, backupDir });
}

export async function RestoreBackup(path, workFile) {
  return await invoke("restore_backup", { path, workFile });
}

export async function ArchiveBackupFile(src, backupDir, format, password) {
  return await invoke("archive_backup_file", {
    src,
    backupDir,
    format,
    password,
  });
}

export async function BackupOrDiff(workFile, customDir, algo, compress) {
  return await invoke("backup_or_diff", {
    workFile,
    customDir,
    algo,
    compress,
  });
}

export async function GetBackupList(workFile, backupDir) {
  return await invoke("get_backup_list", { workFile, backupDir });
}

export async function ApplyMultiDiff(workFile, diffPaths) {
  return await invoke("apply_multi_diff", { workFile, diffPaths });
}

/**
 * 世代アーカイブ用：フォルダリストの取得
 */
export async function GetGenerationFolders(workFile, backupDir) {
  return await invoke("get_generation_folders", { workFile, backupDir });
}

/**
 * 世代アーカイブ用：圧縮実行
 */
export async function ArchiveGeneration(
  targetN,
  format,
  workFile,
  backupDir,
  password = null,
) {
  return await invoke("archive_generation", {
    targetN,
    format,
    workFile,
    backupDir,
    password,
  });
}

/**
 * キャッシュ関連コマンド
 */

// 全ての展開済みキャッシュを削除
export async function ClearAllCaches(backupDir, workFile) {
  return await invoke("clear_all_caches", { backupDir, workFile });
}

// 特定のアーカイブを展開してキャッシュを作成（パスを返す）
export async function PrepareArchiveCache(archivePath, password = null) {
  return await invoke("prepare_archive_cache", { archivePath, password });
}

// 指定ディレクトリ内のアーカイブを全てスキャンしてキャッシュを再構築
export async function RebuildArchiveCaches(workFile, backupDir) {
  return await invoke("rebuild_archive_caches", { workFile, backupDir });
}
