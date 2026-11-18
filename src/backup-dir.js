// backup-dir.js
// カスタム保存先フォルダの使い方(モード)の解決を、ここ1箇所に集約する。
//
// タブの状態:
//   tab.backupDir      ユーザーが指定したフォルダそのもの(従来通り)
//   tab.backupDirMode  "parent" | "direct" | undefined
//     "parent"  指定フォルダを親として、中の wbt_backup_<対象名> に保存する
//     "direct"  指定フォルダ自体に直接保存する(従来の動作)
//     undefined フィールドが無い旧 session.json のタブ。後方互換のため
//               "direct" と同じ扱いにし、既存のバックアップを見失わない。
//
// バックエンドのコマンド群は従来通り「実効保存先」を受け取るので、
// 呼び出し側は resolveEffectiveBackupDir(tab) で解決してから渡すこと。
import {
  GetConfig,
  ListDirEntries,
  DirExists,
  FileExists,
} from "./tauri_exports";
import { askBackupDirMode } from "./ui/backup-dir-dialog.js";

function trimTrailingSep(p) {
  return (p || "").replace(/[/\\]+$/, "");
}

function pathSep(p) {
  return p && p.includes("\\") ? "\\" : "/";
}

export function baseNameOf(p) {
  const t = trimTrailingSep(p);
  const parts = t.split(/[/\\]/);
  return parts[parts.length - 1] || "";
}

export function dirNameOf(p) {
  const t = trimTrailingSep(p);
  const idx = Math.max(t.lastIndexOf("/"), t.lastIndexOf("\\"));
  return idx >= 0 ? t.slice(0, idx) : "";
}

export function joinPath(dir, name) {
  const sep = pathSep(dir);
  const t = trimTrailingSep(dir);
  if (!t) return sep + name;
  return t + sep + name;
}

// Rust の Path::file_stem 相当(最後の拡張子だけを取り除く)
function stemOf(full) {
  const idx = full.lastIndexOf(".");
  return idx > 0 ? full.slice(0, idx) : full;
}

/**
 * Rust の backup_entry_name (core/utils.rs) と同じ規則で、
 * `wbt_backup_<名前>` の <名前> を決める。
 *   フォルダ: 名前そのまま / ファイル: 拡張子なし(stem)
 *   存在しないときは、隣に `wbt_backup_<名前そのまま>` が実在すればそちら。
 */
export async function backupEntryName(workPath) {
  if (!workPath) return "";
  const full = baseNameOf(workPath);
  const stem = stemOf(full);
  if (await DirExists(workPath).catch(() => false)) return full;
  if ((await FileExists(workPath).catch(() => false)) || full === stem) {
    return stem;
  }
  const sibling = joinPath(dirNameOf(workPath), `wbt_backup_${full}`);
  if (await DirExists(sibling).catch(() => false)) return full;
  return stem;
}

/** タブの実効保存先を返す。未指定なら ""。 */
export async function resolveEffectiveBackupDir(tab) {
  const raw = tab?.backupDir || "";
  if (!raw) return "";
  if (tab.backupDirMode !== "parent") return raw;
  if (!tab.workFile) return raw;
  const entry = await backupEntryName(tab.workFile);
  if (!entry) return raw;
  const sub = `wbt_backup_${entry}`;
  // ユーザーが wbt_backup_<対象名> 自体を指定していた場合は二重にネストしない
  if (baseNameOf(raw) === sub) return raw;
  return joinPath(raw, sub);
}

// 世代フォルダ名(base<数字>_...)かどうか
const GENERATION_DIR_RE = /^base\d+_/;

/**
 * 指定されたフォルダの中身から、推奨モードを決める。
 *   - フォルダ名自体が wbt_backup_ で始まる → "direct"(それ自体が保存先)
 *   - 中に wbt_backup_<対象名> がある → "parent"(新しい形で使われている)
 *   - 直下に .base / *.diff / 世代フォルダがある → "direct"(レガシー)
 *   - どれも無ければ useSubfolderDefault(設定の既定動作)に従う
 */
export async function detectBackupDirMode(dir, workFile, useSubfolderDefault) {
  if (baseNameOf(dir).startsWith("wbt_backup_")) return "direct";

  let names = [];
  try {
    names = (await ListDirEntries(dir)) || [];
  } catch {
    names = [];
  }

  const entry = workFile ? await backupEntryName(workFile) : "";
  if (entry && names.includes(`wbt_backup_${entry}`)) return "parent";
  // 作業対象が未設定で対象名が分からないときは、wbt_backup_* があれば親とみなす
  if (!entry && names.some((n) => n.startsWith("wbt_backup_"))) return "parent";

  const hasLegacyTrace = names.some(
    (n) =>
      n.endsWith(".base") || n.endsWith(".diff") || GENERATION_DIR_RE.test(n),
  );
  if (hasLegacyTrace) return "direct";

  return useSubfolderDefault ? "parent" : "direct";
}

/**
 * 保存先が DnD / 手動選択で指定されたときのモード決定フロー。
 * 設定でポップアップを出さない場合は推奨モードをそのまま返し、
 * 出す場合はポップアップの初期選択を推奨モードにして結果を返す。
 * @returns {Promise<"parent"|"direct"|null>} キャンセル時は null(保存先は変更しない)
 */
export async function chooseBackupDirMode(dir, workFile) {
  let config = {};
  try {
    config = (await GetConfig()) || {};
  } catch {
    config = {};
  }
  // キーが無い(旧バックエンド等)場合は、組み込み既定値と同じ true として扱う
  const useSubfolderDefault = config.customBackupDirUseSubfolder !== false;
  const hidePrompt = config.customBackupDirHideModePrompt === true;

  const suggested = await detectBackupDirMode(
    dir,
    workFile,
    useSubfolderDefault,
  );
  if (hidePrompt) return suggested;

  const entry = workFile ? await backupEntryName(workFile) : "";
  return await askBackupDirMode({
    dir,
    initialMode: suggested,
    subfolderName: entry ? `wbt_backup_${entry}` : "wbt_backup_...",
  });
}
