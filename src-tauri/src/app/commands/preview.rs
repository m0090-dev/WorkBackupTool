// 履歴プレビュー(サムネイル)機能。
//
// 履歴の各エントリが表す「その時点の作業ファイル」を一時的に取り出し、
// プレビュー画像を抽出して、縮小した JPEG をキャッシュに保存する。
// バックアップデータ自体へは一切書き込まない(読み取り専用)。
//
// 対応形式:
//   - .clip : 内蔵 SQLite の CanvasPreview から埋め込みプレビューを取り出す
//   - .psd : psd クレートでレイヤー合成(合成画像が無い場合は統合画像)する
//   - .aseprite / .ase : aseprite-loader クレートでフレーム0を合成する
//   - 画像ファイル(png / jpg / jpeg / webp / bmp / gif): そのまま縮小する
// 差分(.diff)は同じ世代の .base と一時合成してから取り出す。
// アーカイブ(.zip / .tar.gz)はファイル単体のアーカイブだけを対象に、
// 一時フォルダへ展開してから取り出す(フォルダのアーカイブと
// パスワード付き ZIP は対象外で、プレビュー無しになる)。
//
// 設定 `previewEnabled` がオフのときは、一時ファイルもキャッシュも作らない。

// 標準ライブラリ
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

// 外部クレート
use base64::Engine;
use tauri::{AppHandle, Manager};

// 内部モジュール (自作)
use crate::app::state::AppState;
use crate::core::ext::hdiff_common;
use crate::core::utils;

/// サムネイルの最大サイズ(長辺 px)
const THUMB_MAX: u32 = 512;
/// サムネイルの JPEG 品質
const THUMB_QUALITY: u8 = 82;

#[derive(Clone, Copy)]
struct PreviewCfg {
    enabled: bool,
    use_same_dir: bool,
    strict_hash: bool,
    strict_match: bool,
}

fn read_cfg(app: &AppHandle) -> PreviewCfg {
    let state = app.state::<AppState>();
    let cfg = state.config.lock().unwrap();
    PreviewCfg {
        enabled: cfg.preview_enabled,
        use_same_dir: cfg.use_same_dir_for_temp,
        strict_hash: cfg.hdiff_strict_hash_check,
        strict_match: cfg.strict_file_name_match,
    }
}

/// エントリのパスのハッシュ(キャッシュのファイル名・作業フォルダ名に使う)
fn path_hash(p: &Path) -> String {
    let mut s = DefaultHasher::new();
    p.to_string_lossy().hash(&mut s);
    format!("{:x}", s.finish())
}

/// サムネイルのキャッシュパス。
/// 元エントリのサイズと更新時刻もキーに含めるので、
/// エントリが差し替われば古いサムネイルは使われなくなる。
fn thumb_path(cache_root: &Path, item_path: &Path) -> Option<PathBuf> {
    let meta = fs::metadata(item_path).ok()?;
    let mtime = meta
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs();
    let mut s = DefaultHasher::new();
    item_path.to_string_lossy().hash(&mut s);
    meta.len().hash(&mut s);
    mtime.hash(&mut s);
    Some(
        cache_root
            .join("previews")
            .join(format!("{:x}.jpg", s.finish())),
    )
}

/// エントリが表す作業ファイル(単一ファイル)を一時フォルダへ用意する。
/// 用意できたらそのパスを返す。フォルダ対象などは None。
async fn materialize_entry(
    app: &AppHandle,
    cfg: &PreviewCfg,
    work_file: &str,
    item_path: &Path,
    work_dir: &Path,
) -> Option<PathBuf> {
    let name = item_path.file_name()?.to_string_lossy().to_lowercase();

    if name.ends_with(".diff") {
        // --- 差分: 同じ世代の .base と一時合成する ---
        if hdiff_common::restore_target_is_folder(work_file, &item_path.to_string_lossy()) {
            return None; // フォルダ対象の差分は対象外
        }
        let out_name = if !work_file.is_empty() {
            Path::new(work_file)
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "preview_out".to_string())
        } else {
            hdiff_common::diff_entry_name(&item_path.to_string_lossy())
                .unwrap_or_else(|| "preview_out".to_string())
        };
        let tmp_out = work_dir.join(out_name);
        let (base_full, out_path) = hdiff_common::resolve_apply_paths(
            work_file,
            &item_path.to_string_lossy(),
            tmp_out.to_string_lossy().into_owned(),
        )
        .ok()?;
        if Path::new(&base_full).is_dir() {
            return None;
        }
        crate::app::hdiff::apply_hdiff(
            app.clone(),
            &base_full,
            &item_path.to_string_lossy(),
            &out_path,
            cfg.strict_hash,
        )
        .await
        .ok()?;
        let out = PathBuf::from(out_path);
        if out.is_file() {
            Some(out)
        } else {
            None
        }
    } else if name.ends_with(".zip") || name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        // --- アーカイブ: ファイル単体のものだけ展開する ---
        let item_str = item_path.to_string_lossy().into_owned();
        let (target_name, is_folder) =
            crate::core::backup::archive::archive_target_info(&item_str).ok()?;
        if is_folder {
            return None;
        }
        let out = work_dir.join(&target_name);
        crate::core::backup::archive::restore_archive_with_password(
            &item_str,
            &out.to_string_lossy(),
            None,
        )
        .ok()?;
        if out.is_file() {
            Some(out)
        } else {
            None
        }
    } else if item_path.is_file() {
        // --- キャッシュ内などに実体があるファイル ---
        Some(item_path.to_path_buf())
    } else {
        None
    }
}

/// .clip ファイルから埋め込みプレビュー画像(PNG)のバイト列を取り出す。
/// コンテナ(CSFCHUNK)のチャンクを先頭から辿って CHNKSQLi(SQLite) を探し、
/// CanvasPreview テーブルの最大の ImageData を返す。
fn extract_clip_preview(clip_path: &Path, work_dir: &Path) -> Option<Vec<u8>> {
    let mut file = fs::File::open(clip_path).ok()?;
    let file_len = file.metadata().ok()?.len();

    // ルートヘッダ: "CSFCHUNK"(8) + ファイルサイズ(8, BE) + 先頭チャンクのオフセット(8, BE)
    let mut root = [0u8; 24];
    file.read_exact(&mut root).ok()?;
    if &root[..8] != b"CSFCHUNK" {
        return None;
    }
    let mut pos = u64::from_be_bytes(root[16..24].try_into().ok()?);
    if pos == 0 || pos >= file_len {
        pos = 24;
    }

    // チャンクを辿る: タグ(8) + ペイロードサイズ(8, BE) + ペイロード
    let mut sqlite_range: Option<(u64, u64)> = None;
    for _ in 0..64 {
        if pos + 16 > file_len {
            break;
        }
        file.seek(SeekFrom::Start(pos)).ok()?;
        let mut head = [0u8; 16];
        file.read_exact(&mut head).ok()?;
        let size = u64::from_be_bytes(head[8..16].try_into().ok()?);
        let payload_start = pos + 16;
        if payload_start + size > file_len {
            break;
        }
        if &head[..8] == b"CHNKSQLi" {
            sqlite_range = Some((payload_start, size));
            break;
        }
        if &head[..8] == b"CHNKFoot" {
            break;
        }
        pos = payload_start + size;
    }
    let (offset, size) = sqlite_range?;

    // SQLite の実体を一時ファイルに書き出して、読み取り専用で開く
    file.seek(SeekFrom::Start(offset)).ok()?;
    let mut db_bytes = vec![0u8; size as usize];
    file.read_exact(&mut db_bytes).ok()?;
    if !db_bytes.starts_with(b"SQLite format 3\0") {
        return None;
    }
    let db_path = work_dir.join("preview_db.sqlite");
    fs::write(&db_path, &db_bytes).ok()?;

    let conn = rusqlite::Connection::open_with_flags(
        &db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .ok()?;
    let data: Vec<u8> = conn
        .query_row(
            "SELECT ImageData FROM CanvasPreview \
             WHERE ImageData IS NOT NULL AND length(ImageData) > 0 \
             ORDER BY (COALESCE(ImageWidth, 0) * COALESCE(ImageHeight, 0)) DESC \
             LIMIT 1",
            [],
            |row| row.get(0),
        )
        .ok()?;
    if data.is_empty() {
        None
    } else {
        Some(data)
    }
}

/// .psd ファイルを合成して画像にする。
/// レイヤーがある場合はレイヤー合成、無い場合は統合画像(互換性最大で保存された合成)を使う。
fn extract_psd_image(source: &Path) -> Option<image::DynamicImage> {
    let bytes = fs::read(source).ok()?;
    let doc = psd::Psd::from_bytes(&bytes).ok()?;
    let (w, h) = (doc.width(), doc.height());
    if w == 0 || h == 0 {
        return None;
    }
    let rgba = if doc.layers().is_empty() {
        doc.rgba()
    } else {
        doc.flatten_layers_rgba(&|_| true).ok()?
    };
    let buf = image::RgbaImage::from_raw(w, h, rgba)?;
    Some(image::DynamicImage::ImageRgba8(buf))
}

/// .aseprite / .ase ファイルのフレーム0を表示レイヤーで合成して画像にする。
fn extract_aseprite_image(source: &Path) -> Option<image::DynamicImage> {
    use aseprite_loader::loader::{AsepriteFile, LayerSelection};
    let bytes = fs::read(source).ok()?;
    let file = AsepriteFile::load(&bytes).ok()?;
    let (w, h) = file.size();
    if w == 0 || h == 0 || file.frames().is_empty() {
        return None;
    }
    let mut target = vec![0u8; w as usize * h as usize * 4];
    file.render_frame(0, &mut target, &LayerSelection::Visible)
        .ok()?;
    let buf = image::RgbaImage::from_raw(w as u32, h as u32, target)?;
    Some(image::DynamicImage::ImageRgba8(buf))
}

/// 用意した作業ファイルから、プレビュー画像を取り出す。
fn extract_preview_image(source: &Path, work_dir: &Path) -> Option<image::DynamicImage> {
    let ext = source
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "clip" => {
            let png = extract_clip_preview(source, work_dir)?;
            image::load_from_memory(&png).ok()
        }
        "psd" => extract_psd_image(source),
        "aseprite" | "ase" => extract_aseprite_image(source),
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif" => {
            image::load_from_memory(&fs::read(source).ok()?).ok()
        }
        _ => {
            // 拡張子で判断できないときは、先頭バイトで画像かどうかだけ見る
            let bytes = fs::read(source).ok()?;
            if bytes.starts_with(b"\x89PNG\r\n\x1a\n") || bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
                image::load_from_memory(&bytes).ok()
            } else {
                None
            }
        }
    }
}

/// 画像を縮小し、白背景に合成して JPEG 化する(サムネイル用)。
fn make_thumbnail_jpeg(img: &image::DynamicImage) -> Option<Vec<u8>> {
    let thumb = img.thumbnail(THUMB_MAX, THUMB_MAX);
    let rgba = thumb.to_rgba8();
    let (w, h) = rgba.dimensions();

    // 透過部分は白で合成する(そのまま RGB 化すると黒ずむため)
    let mut rgb = image::RgbImage::new(w, h);
    for (x, y, p) in rgba.enumerate_pixels() {
        let a = p[3] as u32;
        let blend = |c: u8| ((c as u32 * a + 255 * (255 - a)) / 255) as u8;
        rgb.put_pixel(x, y, image::Rgb([blend(p[0]), blend(p[1]), blend(p[2])]));
    }

    let mut out = std::io::Cursor::new(Vec::new());
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, THUMB_QUALITY);
    encoder
        .encode(rgb.as_raw(), w, h, image::ExtendedColorType::Rgb8)
        .ok()?;
    Some(out.into_inner())
}

/// エントリのサムネイルを用意し、キャッシュのパスを返す(生成済みならそれを返す)。
/// プレビューが作れないエントリは None。
async fn ensure_thumbnail(
    app: &AppHandle,
    cfg: &PreviewCfg,
    work_file: &str,
    backup_dir: &str,
    item_path: &Path,
) -> Option<PathBuf> {
    let cache_root = utils::get_cache_root(cfg.use_same_dir, backup_dir, work_file);
    let thumb = thumb_path(&cache_root, item_path)?;
    if thumb.exists() {
        return Some(thumb);
    }

    // 一時作業フォルダ(合成・展開・DB 書き出し用)
    let work_dir = cache_root
        .join("preview_work")
        .join(path_hash(item_path));
    let _ = fs::remove_dir_all(&work_dir);
    fs::create_dir_all(&work_dir).ok()?;

    let result = async {
        let source = materialize_entry(app, cfg, work_file, item_path, &work_dir).await?;
        let image = extract_preview_image(&source, &work_dir)?;
        let jpeg = make_thumbnail_jpeg(&image)?;
        if let Some(parent) = thumb.parent() {
            fs::create_dir_all(parent).ok()?;
        }
        fs::write(&thumb, &jpeg).ok()?;
        Some(thumb.clone())
    }
    .await;

    // 一時ファイルは残さない
    let _ = fs::remove_dir_all(&work_dir);
    result
}

/// 履歴エントリのプレビュー(サムネイル)を data URL で返す。
/// 設定がオフ、またはプレビューが作れないエントリでは None。
#[tauri::command]
pub async fn get_history_preview(
    app: AppHandle,
    work_file: String,
    backup_dir: String,
    file_path: String,
) -> Result<Option<String>, String> {
    let cfg = read_cfg(&app);
    if !cfg.enabled {
        return Ok(None);
    }
    let item_path = PathBuf::from(&file_path);
    let Some(thumb) = ensure_thumbnail(&app, &cfg, &work_file, &backup_dir, &item_path).await
    else {
        return Ok(None);
    };
    let bytes = fs::read(&thumb).map_err(|e| e.to_string())?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(Some(format!("data:image/jpeg;base64,{}", encoded)))
}

/// 起動時などに、新しい順に上限件数までサムネイルを先行生成する。
/// (上限 0 は無制限。既にキャッシュがあるものも件数に数える)
#[tauri::command]
pub async fn pregenerate_previews(
    app: AppHandle,
    work_file: String,
    backup_dir: String,
    limit: usize,
) -> Result<(), String> {
    let cfg = read_cfg(&app);
    if !cfg.enabled {
        return Ok(());
    }
    let mut items = crate::core::backup::scanner::scan_backups(
        &work_file,
        &backup_dir,
        cfg.strict_match,
        cfg.use_same_dir,
    );
    // タイムスタンプ文字列("YYYY-MM-DD HH:MM:SS")の降順 = 新しい順
    items.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    let mut processed = 0usize;
    for item in items {
        if limit > 0 && processed >= limit {
            break;
        }
        processed += 1;
        let item_path = PathBuf::from(&item.file_path);
        let _ = ensure_thumbnail(&app, &cfg, &work_file, &backup_dir, &item_path).await;
    }
    Ok(())
}
