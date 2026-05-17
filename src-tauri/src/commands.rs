use crate::db;
use crate::indexer::{self, Progress};
use crate::models::*;
use anyhow::Result;
use rusqlite::params;
use std::sync::{Arc, Mutex};
use tauri::State;

pub struct AppState {
    pub progress: Arc<Progress>,
    pub library_path: Mutex<Option<String>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            progress: Arc::new(Progress::new()),
            library_path: Mutex::new(None),
        }
    }
}

#[tauri::command]
pub fn scan_library(path: String, state: State<AppState>) -> Result<(), String> {
    use std::sync::atomic::Ordering;

    if state.progress.running.load(Ordering::SeqCst) {
        return Err("Indexing already in progress".to_string());
    }

    *state.library_path.lock().unwrap() = Some(path.clone());

    let progress = Arc::clone(&state.progress);
    std::thread::spawn(move || {
        if let Err(e) = indexer::run_scan(path, progress.clone()) {
            eprintln!("Scan error: {}", e);
            progress.running.store(false, std::sync::atomic::Ordering::SeqCst);
        }
    });

    Ok(())
}


#[tauri::command]
pub fn get_index_progress(state: State<AppState>) -> IndexProgress {
    use std::sync::atomic::Ordering;
    IndexProgress {
        total: state.progress.total.load(Ordering::Relaxed),
        done: state.progress.done.load(Ordering::Relaxed),
        errors: state.progress.errors.load(Ordering::Relaxed),
        phase: state.progress.phase.lock().unwrap().clone(),
        running: state.progress.running.load(Ordering::SeqCst),
    }
}

#[tauri::command]
pub fn query_media(filters: SearchFilters) -> Result<Vec<MediaSummary>, String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    let root = db::get_library_root(&conn);
    let data = db::data_dir();
    let limit = filters.limit.unwrap_or(100);
    let offset = filters.offset.unwrap_or(0);

    let mut where_parts: Vec<String> = Vec::new();
    let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ref mt) = filters.media_type {
        where_parts.push(format!("m.media_type = ?{}", param_values.len() + 1));
        param_values.push(Box::new(mt.clone()));
    }
    if let Some(min) = filters.min_size_bytes {
        where_parts.push(format!("m.file_size >= ?{}", param_values.len() + 1));
        param_values.push(Box::new(min));
    }
    if let Some(max) = filters.max_size_bytes {
        where_parts.push(format!("m.file_size <= ?{}", param_values.len() + 1));
        param_values.push(Box::new(max));
    }
    if let Some(df) = filters.date_from {
        where_parts.push(format!("m.photo_taken_ts >= ?{}", param_values.len() + 1));
        param_values.push(Box::new(df));
    }
    if let Some(dt) = filters.date_to {
        where_parts.push(format!("m.photo_taken_ts <= ?{}", param_values.len() + 1));
        param_values.push(Box::new(dt));
    }
    if let Some(aid) = filters.album_id {
        where_parts.push(format!("m.album_id = ?{}", param_values.len() + 1));
        param_values.push(Box::new(aid));
    }

    // People filter: AND logic — media must have ALL specified people tagged
    if !filters.people.is_empty() {
        for pid in &filters.people {
            where_parts.push(format!(
                "EXISTS (SELECT 1 FROM media_people mp WHERE mp.media_id = m.id AND mp.person_id = ?{})",
                param_values.len() + 1
            ));
            param_values.push(Box::new(*pid));
        }
    }

    let where_clause = if where_parts.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_parts.join(" AND "))
    };

    let order = match filters.sort_by.as_deref() {
        Some("date_asc")  => "m.photo_taken_ts ASC NULLS LAST",
        Some("size_desc") => "m.file_size DESC",
        Some("size_asc")  => "m.file_size ASC",
        _                 => "m.photo_taken_ts DESC NULLS LAST",
    };

    let sql = format!(
        "SELECT m.id, m.file_path, m.file_name, m.file_size, m.media_type, m.photo_taken_ts, m.thumbnail_path, a.title
         FROM media m
         LEFT JOIN albums a ON m.album_id = a.id
         {}
         ORDER BY {}
         LIMIT ?{} OFFSET ?{}",
        where_clause,
        order,
        param_values.len() + 1,
        param_values.len() + 2
    );

    param_values.push(Box::new(limit));
    param_values.push(Box::new(offset));

    let params_refs: Vec<&dyn rusqlite::ToSql> = param_values.iter().map(|b| b.as_ref()).collect();

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params_refs.as_slice(), |r| {
        let rel_file: String = r.get(1)?;
        let rel_thumb: Option<String> = r.get(6)?;
        Ok(MediaSummary {
            id: r.get(0)?,
            file_path: db::to_absolute(&root, &rel_file),
            file_name: r.get(2)?,
            file_size: r.get(3)?,
            media_type: r.get(4)?,
            photo_taken_ts: r.get(5)?,
            thumbnail_path: rel_thumb.map(|t| data.join(&t).to_string_lossy().into_owned()),
            album_title: r.get(7)?,
        })
    }).map_err(|e| e.to_string())?;

    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_media_detail(id: i64) -> Result<MediaDetail, String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    let root = db::get_library_root(&conn);
    let data = db::data_dir();

    let detail = conn.query_row(
        "SELECT m.id, m.file_path, m.file_name, m.file_size, m.media_type, m.extension,
                m.title, m.description, m.photo_taken_ts, m.creation_ts,
                m.latitude, m.longitude, m.altitude, m.image_views, m.google_url,
                m.origin_type, m.device_type,
                m.exif_width, m.exif_height, m.exif_camera_make, m.exif_camera_model, m.exif_date_ts,
                m.album_id, a.title, m.thumbnail_path, m.file_hash
         FROM media m
         LEFT JOIN albums a ON m.album_id = a.id
         WHERE m.id = ?1",
        params![id],
        |r| {
            let rel_file: String = r.get(1)?;
            let rel_thumb: Option<String> = r.get(24)?;
            Ok(MediaDetail {
                id: r.get(0)?,
                file_path: db::to_absolute(&root, &rel_file),
                file_name: r.get(2)?,
                file_size: r.get(3)?,
                media_type: r.get(4)?,
                extension: r.get(5)?,
                title: r.get(6)?,
                description: r.get(7)?,
                photo_taken_ts: r.get(8)?,
                creation_ts: r.get(9)?,
                latitude: r.get(10)?,
                longitude: r.get(11)?,
                altitude: r.get(12)?,
                image_views: r.get(13)?,
                google_url: r.get(14)?,
                origin_type: r.get(15)?,
                device_type: r.get(16)?,
                exif_width: r.get(17)?,
                exif_height: r.get(18)?,
                exif_camera_make: r.get(19)?,
                exif_camera_model: r.get(20)?,
                exif_date_ts: r.get(21)?,
                album_id: r.get(22)?,
                album_title: r.get(23)?,
                thumbnail_path: rel_thumb.map(|t| data.join(&t).to_string_lossy().into_owned()),
                file_hash: r.get(25)?,
                people: vec![],
                in_duplicate_group: false,
            })
        },
    ).map_err(|e| e.to_string())?;

    let mut detail = detail;

    // People
    let mut stmt = conn.prepare(
        "SELECT p.name FROM people p JOIN media_people mp ON p.id = mp.person_id WHERE mp.media_id = ?1 ORDER BY p.name"
    ).map_err(|e| e.to_string())?;
    detail.people = stmt.query_map(params![id], |r| r.get(0))
        .map_err(|e| e.to_string())?
        .flatten()
        .collect();

    // Duplicate flag
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM duplicate_groups WHERE media_id = ?1",
        params![id],
        |r| r.get(0),
    ).unwrap_or(0);
    detail.in_duplicate_group = count > 0;

    Ok(detail)
}

#[tauri::command]
pub fn get_people() -> Result<Vec<PersonSummary>, String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, COUNT(mp.media_id) as cnt
         FROM people p JOIN media_people mp ON p.id = mp.person_id
         GROUP BY p.id ORDER BY cnt DESC, p.name"
    ).map_err(|e| e.to_string())?;
    let result = stmt.query_map([], |r| Ok(PersonSummary { id: r.get(0)?, name: r.get(1)?, count: r.get(2)? }))
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string());
    result
}

#[tauri::command]
pub fn get_albums() -> Result<Vec<AlbumSummary>, String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT a.id, a.title, a.dir_path, COUNT(m.id) as cnt
         FROM albums a LEFT JOIN media m ON m.album_id = a.id
         GROUP BY a.id ORDER BY cnt DESC, a.title"
    ).map_err(|e| e.to_string())?;
    let result = stmt.query_map([], |r| {
        let id: i64 = r.get(0)?;
        let raw_title: Option<String> = r.get(1)?;
        let dir_path: String = r.get(2)?;
        let count: i64 = r.get(3)?;
        let title = raw_title
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| {
                std::path::Path::new(&dir_path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or(dir_path)
            });
        Ok(AlbumSummary { id, title, count })
    })
    .map_err(|e| e.to_string())?
    .collect::<rusqlite::Result<Vec<_>>>()
    .map_err(|e| e.to_string());
    result
}

#[tauri::command]
pub fn get_stats() -> Result<LibraryStats, String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    let total: i64 = conn.query_row("SELECT COUNT(*) FROM media", [], |r| r.get(0)).unwrap_or(0);
    let photos: i64 = conn.query_row("SELECT COUNT(*) FROM media WHERE media_type='photo'", [], |r| r.get(0)).unwrap_or(0);
    let videos: i64 = conn.query_row("SELECT COUNT(*) FROM media WHERE media_type='video'", [], |r| r.get(0)).unwrap_or(0);
    let total_size: i64 = conn.query_row("SELECT COALESCE(SUM(file_size),0) FROM media", [], |r| r.get(0)).unwrap_or(0);
    let people_count: i64 = conn.query_row("SELECT COUNT(*) FROM people", [], |r| r.get(0)).unwrap_or(0);
    let dup_groups: i64 = conn.query_row(
        "SELECT COUNT(*) FROM (SELECT group_id FROM duplicate_groups GROUP BY group_id HAVING COUNT(*) >= 2)",
        [], |r| r.get(0),
    ).unwrap_or(0);
    Ok(LibraryStats {
        total,
        photos,
        videos,
        total_size_bytes: total_size,
        people_count,
        duplicate_groups: dup_groups,
        indexed: total > 0,
    })
}

#[tauri::command]
pub fn get_duplicates() -> Result<Vec<DuplicateGroup>, String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    let root = db::get_library_root(&conn);
    let data = db::data_dir();
    let mut stmt = conn.prepare(
        "SELECT dg.group_id, dg.match_type, m.id, m.file_path, m.file_name, m.file_size, m.thumbnail_path, m.photo_taken_ts, a.title
         FROM duplicate_groups dg
         JOIN media m ON dg.media_id = m.id
         LEFT JOIN albums a ON m.album_id = a.id
         WHERE dg.group_id IN (
             SELECT group_id FROM duplicate_groups GROUP BY group_id HAVING COUNT(*) >= 2
         )
         ORDER BY dg.group_id, m.id"
    ).map_err(|e| e.to_string())?;

    let rows: Vec<(i64, String, DuplicateItem)> = stmt.query_map([], |r| {
        let rel_file: String = r.get(3)?;
        let rel_thumb: Option<String> = r.get(6)?;
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            DuplicateItem {
                id: r.get(2)?,
                file_path: db::to_absolute(&root, &rel_file),
                file_name: r.get(4)?,
                file_size: r.get(5)?,
                thumbnail_path: rel_thumb.map(|t| data.join(&t).to_string_lossy().into_owned()),
                photo_taken_ts: r.get(7)?,
                album_title: r.get(8)?,
            },
        ))
    }).map_err(|e| e.to_string())?
        .flatten()
        .collect();

    let mut result: Vec<DuplicateGroup> = Vec::new();
    for (gid, match_type, item) in rows {
        if let Some(last) = result.last_mut() {
            if last.group_id == gid {
                last.items.push(item);
                continue;
            }
        }
        result.push(DuplicateGroup { group_id: gid, match_type, items: vec![item] });
    }
    Ok(result)
}

#[tauri::command]
pub fn cleanup_name_duplicates(dry_run: bool) -> Result<CleanupResult, String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    let root = db::get_library_root(&conn);

    // Single query: all duplicate groups with item details, ordered so we can group in Rust.
    // album_id is included so we can prefer album-backed copies as the keeper.
    let mut stmt = conn.prepare(
        "SELECT dg.group_id, m.id, m.file_path, m.file_name, m.file_size, m.album_id
         FROM duplicate_groups dg
         JOIN media m ON dg.media_id = m.id
         ORDER BY dg.group_id, m.id"
    ).map_err(|e| e.to_string())?;

    // (group_id, id, file_path, file_name, file_size, album_id)
    type Row = (i64, i64, String, String, i64, Option<i64>);

    let flat: Vec<Row> = stmt.query_map([], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
    }).map_err(|e| e.to_string())?
        .flatten()
        .collect();

    // Group rows by group_id (already ordered)
    let mut groups: Vec<Vec<Row>> = Vec::new();
    for row in flat {
        if let Some(last) = groups.last_mut() {
            if last[0].0 == row.0 {
                last.push(row);
                continue;
            }
        }
        groups.push(vec![row]);
    }

    let mut groups_eligible = 0i64;
    let groups_skipped = 0i64;
    let mut files_deleted = 0i64;
    let mut bytes_freed = 0i64;
    let mut preview: Vec<CleanupPreviewItem> = Vec::new();

    for mut items in groups {
        groups_eligible += 1;

        // Keep preference: album-backed copy first, then lowest id (first indexed).
        items.sort_by(|a, b| b.5.is_some().cmp(&a.5.is_some()).then(a.1.cmp(&b.1)));

        let kept = &items[0];
        let to_delete = &items[1..];

        let group_bytes: i64 = to_delete.iter().map(|r| r.4).sum();
        bytes_freed += group_bytes;
        files_deleted += to_delete.len() as i64;

        if dry_run {
            if preview.len() < 100 {
                preview.push(CleanupPreviewItem {
                    file_name: items[0].3.clone(),
                    kept_path: db::to_absolute(&root, &kept.2),
                    deleted_paths: to_delete.iter().map(|r| db::to_absolute(&root, &r.2)).collect(),
                    bytes_freed: group_bytes,
                });
            }
        } else {
            for item in to_delete {
                let abs_path = db::to_absolute(&root, &item.2);
                if std::path::Path::new(&abs_path).exists() {
                    if let Err(e) = trash::delete(&abs_path) {
                        eprintln!("Failed to trash {}: {}", abs_path, e);
                        continue; // File still on disk but trash failed — leave it alone
                    }
                }
                // File is gone (either trashed or already missing) — clean up DB record
                conn.execute("DELETE FROM media WHERE id = ?1", params![item.1])
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(CleanupResult { dry_run, groups_eligible, groups_skipped, files_deleted, bytes_freed, preview })
}

#[tauri::command]
pub fn decode_heic_for_viewer(path: String) -> Result<String, String> {
    use md5::Digest;
    // Cache key: MD5 of the file path string (stable, fast)
    let mut hasher = md5::Md5::new();
    hasher.update(path.as_bytes());
    let hash = format!("{:x}", hasher.finalize());

    let cache_dir = db::data_dir().join("heic-cache");
    std::fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    let cache_path = cache_dir.join(format!("{}.jpg", hash));

    if cache_path.exists() {
        return Ok(cache_path.to_string_lossy().to_string());
    }

    let img = indexer::heic_to_dynamic_image(std::path::Path::new(&path))
        .map_err(|e| e.to_string())?;
    img.save(&cache_path).map_err(|e| e.to_string())?;
    Ok(cache_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_in_explorer(path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    if !p.exists() {
        return Err("Path does not exist".to_string());
    }
    let folder = if p.is_dir() { path.clone() } else {
        p.parent().unwrap_or(p).to_string_lossy().to_string()
    };

    #[cfg(target_os = "windows")]
    std::process::Command::new("explorer.exe").arg(&folder).spawn().map_err(|e| e.to_string())?;

    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg(&folder).spawn().map_err(|e| e.to_string())?;

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    std::process::Command::new("xdg-open").arg(&folder).spawn().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn delete_files(ids: Vec<i64>) -> Result<usize, String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    let root = db::get_library_root(&conn);
    let mut deleted = 0;
    for id in ids {
        let rel_path: String = match conn.query_row(
            "SELECT file_path FROM media WHERE id = ?1",
            params![id],
            |r| r.get(0),
        ) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let abs_path = db::to_absolute(&root, &rel_path);
        if std::path::Path::new(&abs_path).exists() {
            if let Err(e) = trash::delete(&abs_path) {
                eprintln!("Failed to trash {}: {}", abs_path, e);
                continue;
            }
        }
        conn.execute("DELETE FROM media WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        deleted += 1;
    }
    Ok(deleted)
}

#[tauri::command]
pub fn delete_file(id: i64) -> Result<(), String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    let root = db::get_library_root(&conn);
    let rel_path: String = conn.query_row(
        "SELECT file_path FROM media WHERE id = ?1",
        params![id],
        |r| r.get(0),
    ).map_err(|e| e.to_string())?;
    let file_path = db::to_absolute(&root, &rel_path);

    // Move to Recycle Bin / Trash if the file still exists on disk
    if std::path::Path::new(&file_path).exists() {
        trash::delete(&file_path).map_err(|e| e.to_string())?;
    }

    conn.execute("DELETE FROM media WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn fix_google_takeout_timestamps(path: String) -> Result<TakeoutFixResult, String> {
    use walkdir::WalkDir;

    let conn = db::open().map_err(|e| e.to_string())?;
    let library_root = db::get_library_root(&conn);
    let root = std::path::Path::new(&library_root);

    let mut res = TakeoutFixResult { total_scanned: 0, updated: 0, no_sidecar: 0, errors: 0 };

    for entry in WalkDir::new(&path).follow_links(false).into_iter().flatten() {
        if !entry.file_type().is_file() { continue; }
        let media_path = entry.path();

        let fname = media_path.file_name().unwrap_or_default().to_string_lossy();
        if fname.ends_with(".json") || fname.starts_with('.') { continue; }

        let ext = media_path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
        if indexer::classify_media_type(&ext).is_none() { continue; }

        res.total_scanned += 1;

        let sidecar_path = match indexer::find_sidecar(media_path) {
            Some(p) => p,
            None => { res.no_sidecar += 1; continue; }
        };

        let sidecar = match indexer::parse_sidecar(&sidecar_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("parse sidecar {:?}: {}", sidecar_path, e);
                res.errors += 1;
                continue;
            }
        };

        let ts = match sidecar.photo_taken_ts.or(sidecar.creation_ts) {
            Some(t) => t,
            None => { res.no_sidecar += 1; continue; }
        };

        if let Err(e) = set_file_timestamps(media_path, ts) {
            eprintln!("set timestamps {:?}: {}", media_path, e);
            res.errors += 1;
            continue;
        }

        // Update DB record if it exists (non-fatal if the file isn't indexed yet)
        let rel_path = db::to_relative(root, media_path);
        let _ = conn.execute(
            "UPDATE media SET photo_taken_ts = ?1, creation_ts = ?2 WHERE file_path = ?3",
            params![sidecar.photo_taken_ts, sidecar.creation_ts, &rel_path],
        );

        res.updated += 1;
    }

    Ok(res)
}

/// Sets both creation time and modification time on a file using the Windows SetFileTime API.
#[cfg(windows)]
fn set_file_timestamps(path: &std::path::Path, unix_ts: i64) -> Result<(), String> {
    use std::os::windows::io::AsRawHandle;

    #[repr(C)]
    struct FILETIME { dw_low: u32, dw_high: u32 }

    extern "system" {
        fn SetFileTime(
            h_file: *mut std::ffi::c_void,
            lp_creation_time: *const FILETIME,
            lp_last_access_time: *const FILETIME,
            lp_last_write_time: *const FILETIME,
        ) -> i32;
    }

    // Windows FILETIME: 100-nanosecond intervals since 1601-01-01
    // Offset from Unix epoch (1970-01-01): 11,644,473,600 seconds
    let windows_100ns = (unix_ts + 11_644_473_600) * 10_000_000;
    let ft = FILETIME {
        dw_low:  (windows_100ns as u64 & 0xFFFF_FFFF) as u32,
        dw_high: ((windows_100ns as u64 >> 32) & 0xFFFF_FFFF) as u32,
    };

    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .map_err(|e| format!("open {:?}: {}", path, e))?;

    let ok = unsafe { SetFileTime(file.as_raw_handle(), &ft, std::ptr::null(), &ft) };
    if ok == 0 {
        return Err(format!("SetFileTime failed for {:?}", path));
    }
    Ok(())
}

#[cfg(not(windows))]
fn set_file_timestamps(path: &std::path::Path, unix_ts: i64) -> Result<(), String> {
    use std::time::{Duration, UNIX_EPOCH};
    let t = UNIX_EPOCH + Duration::from_secs(unix_ts.max(0) as u64);
    let times = std::fs::FileTimes::new().set_accessed(t).set_modified(t);
    std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .and_then(|f| f.set_times(times))
        .map_err(|e| format!("set_times {:?}: {}", path, e))
}
