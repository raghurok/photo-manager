mod commands;
mod db;
mod indexer;
mod models;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::new())
        .register_uri_scheme_protocol("localfile", |_app, request| {
            use std::io::{Read, Seek, SeekFrom};

            let file_path = percent_decode(request.uri().path().trim_start_matches('/'));

            // Restrict to data_dir (thumbs, HEIC cache) and the library root.
            // This prevents a malicious script from using this protocol to read
            // arbitrary files off the filesystem.
            let library_root = db::get_library_root_quick();
            let data_dir = db::data_dir();
            let req_path = std::path::Path::new(&file_path);
            let allowed = req_path.starts_with(&data_dir)
                || (!library_root.is_empty() && req_path.starts_with(&library_root));
            if !allowed {
                return tauri::http::Response::builder().status(403).body(vec![]).unwrap();
            }

            let mime = guess_mime(&file_path);

            let meta = match std::fs::metadata(&file_path) {
                Ok(m) => m,
                Err(_) => return tauri::http::Response::builder().status(404).body(vec![]).unwrap(),
            };
            let total = meta.len();

            // Support range requests so the <video> element can seek/buffer.
            if let Some(range_hdr) = request.headers().get("Range") {
                if let Ok(range_str) = range_hdr.to_str() {
                    if let Some((start, end)) = parse_range(range_str, total) {
                        let length = end - start + 1;
                        let mut file = match std::fs::File::open(&file_path) {
                            Ok(f) => f,
                            Err(_) => return tauri::http::Response::builder().status(500).body(vec![]).unwrap(),
                        };
                        let _ = file.seek(SeekFrom::Start(start));
                        let mut buf = vec![0u8; length as usize];
                        let _ = file.read_exact(&mut buf);
                        return tauri::http::Response::builder()
                            .status(206)
                            .header("Content-Type", mime)
                            .header("Content-Range", format!("bytes {}-{}/{}", start, end, total))
                            .header("Content-Length", length.to_string())
                            .header("Accept-Ranges", "bytes")
                            .body(buf)
                            .unwrap();
                    }
                }
            }

            match std::fs::read(&file_path) {
                Ok(data) => tauri::http::Response::builder()
                    .header("Content-Type", mime)
                    .header("Content-Length", total.to_string())
                    .header("Accept-Ranges", "bytes")
                    .body(data)
                    .unwrap(),
                Err(_) => tauri::http::Response::builder().status(404).body(vec![]).unwrap(),
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::scan_library,
            commands::get_index_progress,
            commands::query_media,
            commands::get_media_detail,
            commands::get_people,
            commands::get_albums,
            commands::get_stats,
            commands::get_duplicates,
            commands::cleanup_name_duplicates,
            commands::decode_heic_for_viewer,
            commands::open_in_explorer,
            commands::delete_files,
            commands::delete_file,
            commands::fix_google_takeout_timestamps,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn guess_mime(path: &str) -> &'static str {
    let p = path.to_lowercase();
    if p.ends_with(".mp4") || p.ends_with(".m4v") { "video/mp4" }
    else if p.ends_with(".webm") { "video/webm" }
    else if p.ends_with(".mov") { "video/quicktime" }
    else if p.ends_with(".avi") { "video/x-msvideo" }
    else if p.ends_with(".mkv") { "video/x-matroska" }
    else if p.ends_with(".png") { "image/png" }
    else if p.ends_with(".gif") { "image/gif" }
    else if p.ends_with(".webp") { "image/webp" }
    else { "image/jpeg" }
}

fn parse_range(range_str: &str, total: u64) -> Option<(u64, u64)> {
    let s = range_str.strip_prefix("bytes=")?;
    let mut parts = s.splitn(2, '-');
    let start: u64 = parts.next()?.trim().parse().ok()?;
    let end_str = parts.next()?.trim();
    let end = if end_str.is_empty() {
        total.saturating_sub(1)
    } else {
        end_str.parse::<u64>().ok()?.min(total.saturating_sub(1))
    };
    if start <= end { Some((start, end)) } else { None }
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[i+1..i+3]) {
                if let Ok(b) = u8::from_str_radix(hex, 16) {
                    out.push(b);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
