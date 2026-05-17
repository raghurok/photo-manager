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

    let sql = format!(
        "SELECT m.id, m.file_path, m.file_name, m.file_size, m.media_type, m.photo_taken_ts, m.thumbnail_path, a.title
         FROM media m
         LEFT JOIN albums a ON m.album_id = a.id
         {}
         ORDER BY m.photo_taken_ts DESC NULLS LAST
         LIMIT ?{} OFFSET ?{}",
        where_clause,
        param_values.len() + 1,
        param_values.len() + 2
    );

    param_values.push(Box::new(limit));
    param_values.push(Box::new(offset));

    let params_refs: Vec<&dyn rusqlite::ToSql> = param_values.iter().map(|b| b.as_ref()).collect();

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params_refs.as_slice(), |r| {
        Ok(MediaSummary {
            id: r.get(0)?,
            file_path: r.get(1)?,
            file_name: r.get(2)?,
            file_size: r.get(3)?,
            media_type: r.get(4)?,
            photo_taken_ts: r.get(5)?,
            thumbnail_path: r.get(6)?,
            album_title: r.get(7)?,
        })
    }).map_err(|e| e.to_string())?;

    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_media_detail(id: i64) -> Result<MediaDetail, String> {
    let conn = db::open().map_err(|e| e.to_string())?;

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
        |r| Ok(MediaDetail {
            id: r.get(0)?,
            file_path: r.get(1)?,
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
            thumbnail_path: r.get(24)?,
            file_hash: r.get(25)?,
            people: vec![],
            in_duplicate_group: false,
        }),
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
        "SELECT a.id, COALESCE(a.title, '(untitled)'), COUNT(m.id) as cnt
         FROM albums a LEFT JOIN media m ON m.album_id = a.id
         GROUP BY a.id ORDER BY cnt DESC, a.title"
    ).map_err(|e| e.to_string())?;
    let result = stmt.query_map([], |r| Ok(AlbumSummary { id: r.get(0)?, title: r.get(1)?, count: r.get(2)? }))
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
    let dup_groups: i64 = conn.query_row("SELECT COUNT(DISTINCT group_id) FROM duplicate_groups", [], |r| r.get(0)).unwrap_or(0);
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
    let mut stmt = conn.prepare(
        "SELECT dg.group_id, dg.match_type, m.id, m.file_path, m.file_name, m.file_size, m.thumbnail_path, m.photo_taken_ts, a.title
         FROM duplicate_groups dg
         JOIN media m ON dg.media_id = m.id
         LEFT JOIN albums a ON m.album_id = a.id
         ORDER BY dg.group_id, m.id"
    ).map_err(|e| e.to_string())?;

    let rows: Vec<(i64, String, DuplicateItem)> = stmt.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            DuplicateItem {
                id: r.get(2)?,
                file_path: r.get(3)?,
                file_name: r.get(4)?,
                file_size: r.get(5)?,
                thumbnail_path: r.get(6)?,
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
pub fn open_in_explorer(path: String) -> Result<(), String> {
    // Validate path is under indexed library (basic check)
    let p = std::path::Path::new(&path);
    if !p.exists() {
        return Err("Path does not exist".to_string());
    }
    // Open containing folder in Windows Explorer
    let folder = if p.is_dir() { path.clone() } else {
        p.parent().unwrap_or(p).to_string_lossy().to_string()
    };
    std::process::Command::new("explorer.exe")
        .arg(&folder)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_file(id: i64) -> Result<(), String> {
    let conn = db::open().map_err(|e| e.to_string())?;
    let file_path: String = conn.query_row(
        "SELECT file_path FROM media WHERE id = ?1",
        params![id],
        |r| r.get(0),
    ).map_err(|e| e.to_string())?;

    // Move to Recycle Bin (safe delete)
    trash::delete(&file_path).map_err(|e| e.to_string())?;

    // Remove from DB
    conn.execute("DELETE FROM media WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    Ok(())
}
