use anyhow::{Context, Result};
use turbojpeg::{Decompressor, Image, PixelFormat, ScalingFactor};
use md5::Digest;
use rusqlite::{params, Connection};
use serde_json::Value;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

use crate::db;

const PHOTO_EXTS: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "bmp", "heic", "heif", "arw", "dng", "tiff", "tif"];
const VIDEO_EXTS: &[&str] = &["mp4", "mov", "avi", "m4v", "mpg", "mpeg", "3gp", "mkv", "wmv"];

pub struct Progress {
    pub total: Arc<AtomicU64>,
    pub done: Arc<AtomicU64>,
    pub errors: Arc<AtomicU64>,
    pub phase: Arc<std::sync::Mutex<String>>,
    pub running: Arc<AtomicBool>,
}

impl Progress {
    pub fn new() -> Self {
        Self {
            total: Arc::new(AtomicU64::new(0)),
            done: Arc::new(AtomicU64::new(0)),
            errors: Arc::new(AtomicU64::new(0)),
            phase: Arc::new(std::sync::Mutex::new("Idle".to_string())),
            running: Arc::new(AtomicBool::new(false)),
        }
    }
}

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub fn classify_media_type(ext: &str) -> Option<&'static str> {
    let lower = ext.to_lowercase();
    let lower = lower.as_str();
    if PHOTO_EXTS.contains(&lower) {
        Some("photo")
    } else if VIDEO_EXTS.contains(&lower) {
        Some("video")
    } else {
        None
    }
}

fn compute_md5(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = md5::Md5::new();
    let mut buf = vec![0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn find_sidecar(media_path: &Path) -> Option<PathBuf> {
    let file_name = media_path.file_name()?.to_string_lossy();
    let dir = media_path.parent()?;

    // Try candidates in priority order
    let candidates = [
        format!("{}.supplemental-metadata.json", file_name),
        format!("{}.supplemental-m.json", file_name),
        format!("{}.suppl.json", file_name),
        format!("{}.supplemental-metadata(1).json", file_name),
    ];
    for c in &candidates {
        let p = dir.join(c);
        if p.exists() {
            return Some(p);
        }
    }

    // Glob fallback: any file starting with the media name + ".json" or ending with ".json"
    if let Ok(pattern) = glob::glob(&format!("{}/{}*.json", dir.to_string_lossy(), file_name)) {
        for entry in pattern.flatten() {
            // Skip album-level metadata.json files
            if entry.file_name().map(|n| n != "metadata.json").unwrap_or(false) {
                return Some(entry);
            }
        }
    }
    None
}

#[derive(Default)]
pub struct Sidecar {
    pub title: Option<String>,
    pub description: Option<String>,
    pub photo_taken_ts: Option<i64>,
    pub creation_ts: Option<i64>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f64>,
    pub image_views: Option<i64>,
    pub google_url: Option<String>,
    pub origin_type: Option<String>,
    pub device_type: Option<String>,
    pub people: Vec<String>,
}

pub fn parse_sidecar(path: &Path) -> Result<Sidecar> {
    let raw = fs::read_to_string(path)?;
    let v: Value = serde_json::from_str(&raw)?;
    let mut s = Sidecar::default();

    s.title = v["title"].as_str().map(String::from).filter(|t| !t.is_empty());
    s.description = v["description"].as_str().map(String::from).filter(|d| !d.is_empty());

    if let Some(ts) = v["photoTakenTime"]["timestamp"].as_str() {
        s.photo_taken_ts = ts.parse::<i64>().ok();
    }
    if let Some(ts) = v["creationTime"]["timestamp"].as_str() {
        s.creation_ts = ts.parse::<i64>().ok();
    }

    // Prefer geoDataExif if non-zero, fall back to geoData
    let geo_exif = &v["geoDataExif"];
    let geo = &v["geoData"];
    let lat = geo_exif["latitude"].as_f64().filter(|&x| x != 0.0)
        .or_else(|| geo["latitude"].as_f64().filter(|&x| x != 0.0));
    let lon = geo_exif["longitude"].as_f64().filter(|&x| x != 0.0)
        .or_else(|| geo["longitude"].as_f64().filter(|&x| x != 0.0));
    let alt = geo_exif["altitude"].as_f64().filter(|&x| x != 0.0)
        .or_else(|| geo["altitude"].as_f64().filter(|&x| x != 0.0));
    s.latitude = lat;
    s.longitude = lon;
    s.altitude = alt;

    if let Some(views) = v["imageViews"].as_str() {
        s.image_views = views.parse::<i64>().ok();
    }
    s.google_url = v["url"].as_str().map(String::from).filter(|u| !u.is_empty());

    // origin type
    if v["googlePhotosOrigin"]["picasa"].is_object() {
        s.origin_type = Some("picasa".to_string());
    } else if v["googlePhotosOrigin"]["webUpload"].is_object() {
        s.origin_type = Some("webUpload".to_string());
    } else if let Some(mob) = v["googlePhotosOrigin"]["mobileUpload"].as_object() {
        s.origin_type = Some("mobileUpload".to_string());
        s.device_type = mob.get("deviceType").and_then(|d| d.as_str()).map(String::from);
    } else if v["googlePhotosOrigin"]["fromPartnerSharing"].is_object() {
        s.origin_type = Some("fromPartnerSharing".to_string());
    }

    if let Some(arr) = v["people"].as_array() {
        for p in arr {
            if let Some(name) = p["name"].as_str() {
                if !name.is_empty() {
                    s.people.push(name.to_string());
                }
            }
        }
    }

    Ok(s)
}

#[derive(Default)]
struct ExifData {
    width: Option<i64>,
    height: Option<i64>,
    camera_make: Option<String>,
    camera_model: Option<String>,
    date_ts: Option<i64>,
}

fn parse_exif(path: &Path) -> ExifData {
    let mut ed = ExifData::default();
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return ed,
    };
    let mut buf_reader = std::io::BufReader::new(file);
    let exif = match exif::Reader::new().read_from_container(&mut buf_reader) {
        Ok(e) => e,
        Err(_) => return ed,
    };

    use exif::{In, Tag};

    if let Some(f) = exif.get_field(Tag::PixelXDimension, In::PRIMARY) {
        ed.width = f.value.get_uint(0).map(|v| v as i64);
    }
    if let Some(f) = exif.get_field(Tag::PixelYDimension, In::PRIMARY) {
        ed.height = f.value.get_uint(0).map(|v| v as i64);
    }
    if let Some(f) = exif.get_field(Tag::Make, In::PRIMARY) {
        ed.camera_make = Some(f.display_value().to_string());
    }
    if let Some(f) = exif.get_field(Tag::Model, In::PRIMARY) {
        ed.camera_model = Some(f.display_value().to_string());
    }
    if let Some(f) = exif.get_field(Tag::DateTimeOriginal, In::PRIMARY) {
        let s = f.display_value().to_string();
        // Format: "2012-07-02 21:03:19"
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S") {
            ed.date_ts = Some(dt.and_utc().timestamp());
        }
    }
    ed
}

fn generate_thumbnail_jpeg(data: &[u8], thumb_path: &Path) -> Result<()> {
    let mut d = Decompressor::new()?;
    let header = d.read_header(data)?;

    // Pick the coarsest DCT scale that keeps both dimensions >= 256.
    // Lossless JPEGs cannot be DCT-scaled.
    let scale = if header.is_lossless {
        ScalingFactor::ONE
    } else {
        let short = header.width.min(header.height);
        if short >= 2048 { ScalingFactor::ONE_EIGHTH }
        else if short >= 1024 { ScalingFactor::ONE_QUARTER }
        else if short >= 512 { ScalingFactor::ONE_HALF }
        else { ScalingFactor::ONE }
    };

    d.set_scaling_factor(scale)?;
    let sh = header.scaled(scale);
    let pitch = sh.width * 3;
    let mut img = Image {
        pixels: vec![0u8; sh.height * pitch],
        width: sh.width,
        pitch,
        height: sh.height,
        format: PixelFormat::RGB,
    };
    d.decompress(data, img.as_deref_mut())?;

    let rgb = image::RgbImage::from_raw(sh.width as u32, sh.height as u32, img.pixels)
        .context("turbojpeg returned invalid pixel buffer")?;
    image::DynamicImage::ImageRgb8(rgb).thumbnail(256, 256).save(thumb_path)?;
    Ok(())
}

/// Decodes a HEIC/HEIF file into a DynamicImage. Used by both thumbnail generation
/// and the on-demand viewer decode command.
pub(crate) fn heic_to_dynamic_image(path: &Path) -> Result<image::DynamicImage> {
    use libheif_rs::{ColorSpace, HeifContext, LibHeif, RgbChroma};
    let lib = LibHeif::new();
    let ctx = HeifContext::read_from_file(path.to_str().context("non-UTF8 path")?)?;
    let handle = ctx.primary_image_handle()?;
    let img = lib.decode(&handle, ColorSpace::Rgb(RgbChroma::Rgb), None)?;
    let width = img.width() as usize;
    let height = img.height() as usize;
    let planes = img.planes();
    let plane = planes.interleaved.context("HEIC decode: no interleaved plane")?;
    let stride = plane.stride;
    let mut pixels = Vec::with_capacity(width * height * 3);
    for row in 0..height {
        let start = row * stride;
        pixels.extend_from_slice(&plane.data[start..start + width * 3]);
    }
    let rgb = image::RgbImage::from_raw(width as u32, height as u32, pixels)
        .context("HEIC decode: invalid pixel buffer")?;
    Ok(image::DynamicImage::ImageRgb8(rgb))
}

fn generate_thumbnail(media_path: &Path, hash: &str) -> Option<PathBuf> {
    let thumb_dir = db::data_dir().join("thumbs");
    let _ = fs::create_dir_all(&thumb_dir);
    let thumb_path = thumb_dir.join(format!("{}.jpg", hash));

    if thumb_path.exists() {
        return Some(thumb_path);
    }

    let ext = media_path.extension()?.to_string_lossy().to_lowercase();
    if VIDEO_EXTS.contains(&ext.as_str()) {
        return None;
    }
    if ext == "heic" || ext == "heif" {
        return heic_to_dynamic_image(media_path).ok()
            .and_then(|img| img.thumbnail(256, 256).save(&thumb_path).ok())
            .map(|_| thumb_path);
    }

    if ext == "jpg" || ext == "jpeg" {
        let data = fs::read(media_path).ok()?;
        generate_thumbnail_jpeg(&data, &thumb_path).ok()?;
    } else {
        let img = image::open(media_path).ok()?;
        img.thumbnail(256, 256).save(&thumb_path).ok()?;
    }
    Some(thumb_path)
}

/// Like `generate_thumbnail` but returns the path relative to `data_dir()` with
/// forward slashes so it is portable across Windows and macOS.
fn generate_thumbnail_rel(media_path: &Path, hash: &str) -> Option<String> {
    let abs = generate_thumbnail(media_path, hash)?;
    abs.strip_prefix(&db::data_dir()).ok()
        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
}

pub fn run_scan(library_path: String, progress: Arc<Progress>) -> Result<()> {
    progress.running.store(true, Ordering::SeqCst);
    *progress.phase.lock().unwrap() = "Scanning files".to_string();

    let mut conn = db::open()?;
    db::set_library_root(&conn, &library_path)?;

    // Collect all media files first
    let mut media_files: Vec<PathBuf> = Vec::new();
    let root = PathBuf::from(&library_path);

    for entry in WalkDir::new(&root).follow_links(false).into_iter().flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path().to_path_buf();
        let fname = path.file_name().unwrap_or_default().to_string_lossy();

        // Skip album-level metadata.json, sidecar JSONs, and hidden files
        if fname == "metadata.json" || fname.ends_with(".json") || fname.starts_with('.') {
            continue;
        }

        let ext = path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
        if classify_media_type(&ext).is_some() {
            media_files.push(path);
        }
    }

    let total = media_files.len() as u64;
    progress.total.store(total, Ordering::SeqCst);
    *progress.phase.lock().unwrap() = "Indexing media".to_string();

    // Index albums (scan dirs with metadata.json)
    for entry in WalkDir::new(&root).max_depth(2).follow_links(false).into_iter().flatten() {
        if entry.file_type().is_file() && entry.file_name() == "metadata.json" {
            let album_dir = entry.path().parent().unwrap_or(&root);
            if let Ok(raw) = fs::read_to_string(entry.path()) {
                if let Ok(v) = serde_json::from_str::<Value>(&raw) {
                    let title = v["title"].as_str().filter(|t| !t.is_empty())
                        .map(String::from)
                        .unwrap_or_else(|| {
                            album_dir.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default()
                        });
                    let description = v["description"].as_str().map(String::from);
                    let access = v["access"].as_str().map(String::from);
                    let date_ts = v["date"]["timestamp"].as_str().and_then(|t| t.parse::<i64>().ok());
                    let _ = db::upsert_album(
                        &conn,
                        &db::to_relative(&root, album_dir),
                        if title.is_empty() { None } else { Some(title.as_str()) },
                        description.as_deref(),
                        access.as_deref(),
                        date_ts,
                    );
                }
            }
        }
    }

    // Index each media file
    for path in &media_files {
        let result = index_one_file(&conn, path, &root);
        if result.is_err() {
            progress.errors.fetch_add(1, Ordering::Relaxed);
        }
        progress.done.fetch_add(1, Ordering::Relaxed);
    }

    // Prune records for files no longer on disk
    *progress.phase.lock().unwrap() = "Pruning deleted files".to_string();
    let mut stmt = conn.prepare("SELECT id, file_path FROM media")?;
    let all_db_paths: Vec<(i64, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt);
    for (id, rel_path) in all_db_paths {
        let abs_path = db::to_absolute(&library_path, &rel_path);
        if !Path::new(&abs_path).exists() {
            conn.execute("DELETE FROM media WHERE id = ?1", params![id])?;
        }
    }

    // Duplicate detection
    *progress.phase.lock().unwrap() = "Finding duplicates".to_string();
    detect_duplicates(&mut conn)?;

    *progress.phase.lock().unwrap() = "Done".to_string();
    progress.running.store(false, Ordering::SeqCst);
    Ok(())
}

fn index_one_file(conn: &Connection, path: &Path, root: &Path) -> Result<()> {
    let meta = fs::metadata(path).context("stat")?;
    let file_size = meta.len() as i64;
    let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
    let ext = path.extension().unwrap_or_default().to_string_lossy().to_lowercase().to_string();
    let media_type = classify_media_type(&ext).unwrap_or("photo").to_string();
    // Store as a portable relative path (forward slashes, relative to library root)
    let file_path = db::to_relative(root, path);

    // Check if already indexed and unchanged
    let existing: Option<(i64, Option<String>)> = conn.query_row(
        "SELECT indexed_at, file_hash FROM media WHERE file_path = ?1",
        params![file_path],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).ok();

    let file_hash = compute_md5(path)?;

    if let Some((_, Some(ref old_hash))) = existing {
        if old_hash == &file_hash {
            // File content unchanged — but if thumbnail is missing, generate it now.
            // This handles files that were indexed before thumbnail support was added
            // (e.g. HEIC files indexed before libheif was available).
            let thumb_missing: bool = conn.query_row(
                "SELECT thumbnail_path IS NULL FROM media WHERE file_path = ?1",
                params![file_path],
                |r| r.get(0),
            ).unwrap_or(false);

            if thumb_missing {
                if let Some(thumb_rel) = generate_thumbnail_rel(path, &file_hash) {
                    conn.execute(
                        "UPDATE media SET thumbnail_path = ?1 WHERE file_path = ?2",
                        params![thumb_rel, file_path],
                    )?;
                }
            }
            return Ok(());
        }
    }

    // Parse sidecar
    let sidecar = find_sidecar(path)
        .and_then(|sp| parse_sidecar(&sp).ok())
        .unwrap_or_default();

    // Parse EXIF (photos only)
    let exif = if media_type == "photo" { parse_exif(path) } else { ExifData::default() };

    // Thumbnail — stored relative to data_dir so it survives drive-letter changes
    let thumb_rel = generate_thumbnail_rel(path, &file_hash);

    // Resolve album_id from parent directory (also stored relative)
    let album_dir = db::to_relative(root, path.parent().unwrap_or(root));
    let album_id: Option<i64> = conn.query_row(
        "SELECT id FROM albums WHERE dir_path = ?1",
        params![album_dir],
        |r| r.get(0),
    ).ok();

    let now = now_epoch();

    conn.execute(
        "INSERT INTO media (
            file_path, file_name, file_size, file_hash, media_type, extension,
            title, description, photo_taken_ts, creation_ts,
            latitude, longitude, altitude, image_views, google_url, origin_type, device_type,
            exif_width, exif_height, exif_camera_make, exif_camera_model, exif_date_ts,
            album_id, thumbnail_path, indexed_at
         ) VALUES (
            ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25
         )
         ON CONFLICT(file_path) DO UPDATE SET
            file_size=excluded.file_size, file_hash=excluded.file_hash,
            media_type=excluded.media_type, extension=excluded.extension,
            title=excluded.title, description=excluded.description,
            photo_taken_ts=excluded.photo_taken_ts, creation_ts=excluded.creation_ts,
            latitude=excluded.latitude, longitude=excluded.longitude, altitude=excluded.altitude,
            image_views=excluded.image_views, google_url=excluded.google_url,
            origin_type=excluded.origin_type, device_type=excluded.device_type,
            exif_width=excluded.exif_width, exif_height=excluded.exif_height,
            exif_camera_make=excluded.exif_camera_make, exif_camera_model=excluded.exif_camera_model,
            exif_date_ts=excluded.exif_date_ts,
            album_id=excluded.album_id, thumbnail_path=excluded.thumbnail_path,
            indexed_at=excluded.indexed_at",
        params![
            file_path, file_name, file_size, file_hash, media_type, ext,
            sidecar.title, sidecar.description, sidecar.photo_taken_ts, sidecar.creation_ts,
            sidecar.latitude, sidecar.longitude, sidecar.altitude,
            sidecar.image_views, sidecar.google_url, sidecar.origin_type, sidecar.device_type,
            exif.width, exif.height, exif.camera_make, exif.camera_model, exif.date_ts,
            album_id, thumb_rel, now,
        ],
    )?;

    let media_id: i64 = conn.query_row(
        "SELECT id FROM media WHERE file_path = ?1",
        params![file_path],
        |r| r.get(0),
    )?;

    // Clear old people links and re-insert
    conn.execute("DELETE FROM media_people WHERE media_id = ?1", params![media_id])?;
    for name in &sidecar.people {
        if let Ok(person_id) = db::upsert_person(conn, name) {
            let _ = conn.execute(
                "INSERT OR IGNORE INTO media_people (media_id, person_id) VALUES (?1, ?2)",
                params![media_id, person_id],
            );
        }
    }

    Ok(())
}

fn detect_duplicates(conn: &mut Connection) -> Result<()> {
    db::clear_duplicate_groups(conn)?;
    let tx = conn.transaction()?;
    let mut group_id: i64 = 1;

    // Phase 1: exact hash duplicates
    {
        let mut stmt = tx.prepare(
            "SELECT file_hash, GROUP_CONCAT(id) as ids
             FROM media WHERE file_hash IS NOT NULL
             GROUP BY file_hash HAVING COUNT(*) > 1"
        )?;
        let rows: Vec<(String, String)> = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .flatten()
            .collect();

        for (_, ids_str) in rows {
            for id_str in ids_str.split(',') {
                if let Ok(mid) = id_str.trim().parse::<i64>() {
                    tx.execute(
                        "INSERT OR IGNORE INTO duplicate_groups (group_id, media_id, match_type) VALUES (?1, ?2, 'hash')",
                        params![group_id, mid],
                    )?;
                }
            }
            group_id += 1;
        }
    }

    // Phase 2: EXIF fingerprint duplicates (same date + camera + dimensions)
    {
        let mut stmt = tx.prepare(
            "SELECT GROUP_CONCAT(id) as ids
             FROM media
             WHERE exif_date_ts IS NOT NULL
               AND exif_camera_make IS NOT NULL
               AND exif_width IS NOT NULL
             GROUP BY exif_date_ts, exif_camera_make, exif_camera_model, exif_width, exif_height
             HAVING COUNT(*) > 1"
        )?;
        let rows: Vec<String> = stmt.query_map([], |r| r.get(0))?
            .flatten()
            .collect();

        for ids_str in rows {
            for id_str in ids_str.split(',') {
                if let Ok(mid) = id_str.trim().parse::<i64>() {
                    tx.execute(
                        "INSERT OR IGNORE INTO duplicate_groups (group_id, media_id, match_type) VALUES (?1, ?2, 'exif_fingerprint')",
                        params![group_id, mid],
                    )?;
                }
            }
            group_id += 1;
        }
    }

    tx.commit()?;
    Ok(())
}

