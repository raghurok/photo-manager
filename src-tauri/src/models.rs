use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaSummary {
    pub id: i64,
    pub file_path: String,
    pub file_name: String,
    pub file_size: i64,
    pub media_type: String,
    pub photo_taken_ts: Option<i64>,
    pub thumbnail_path: Option<String>,
    pub album_title: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaDetail {
    pub id: i64,
    pub file_path: String,
    pub file_name: String,
    pub file_size: i64,
    pub media_type: String,
    pub extension: String,
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
    pub exif_width: Option<i64>,
    pub exif_height: Option<i64>,
    pub exif_camera_make: Option<String>,
    pub exif_camera_model: Option<String>,
    pub exif_date_ts: Option<i64>,
    pub album_id: Option<i64>,
    pub album_title: Option<String>,
    pub thumbnail_path: Option<String>,
    pub file_hash: Option<String>,
    pub people: Vec<String>,
    pub in_duplicate_group: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PersonSummary {
    pub id: i64,
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AlbumSummary {
    pub id: i64,
    pub title: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LibraryStats {
    pub total: i64,
    pub photos: i64,
    pub videos: i64,
    pub total_size_bytes: i64,
    pub people_count: i64,
    pub duplicate_groups: i64,
    pub indexed: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IndexProgress {
    pub total: u64,
    pub done: u64,
    pub errors: u64,
    pub phase: String,
    pub running: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SearchFilters {
    #[serde(default)]
    pub people: Vec<i64>,
    pub media_type: Option<String>,
    pub min_size_bytes: Option<i64>,
    pub max_size_bytes: Option<i64>,
    pub date_from: Option<i64>,
    pub date_to: Option<i64>,
    pub album_id: Option<i64>,
    pub sort_by: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DuplicateGroup {
    pub group_id: i64,
    pub match_type: String,
    pub items: Vec<DuplicateItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DuplicateItem {
    pub id: i64,
    pub file_path: String,
    pub file_name: String,
    pub file_size: i64,
    pub thumbnail_path: Option<String>,
    pub photo_taken_ts: Option<i64>,
    pub album_title: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CleanupPreviewItem {
    pub file_name: String,
    pub kept_path: String,
    pub deleted_paths: Vec<String>,
    pub bytes_freed: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CleanupResult {
    pub dry_run: bool,
    pub groups_eligible: i64,
    pub groups_skipped: i64,
    pub files_deleted: i64,
    pub bytes_freed: i64,
    pub preview: Vec<CleanupPreviewItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TakeoutFixResult {
    pub total_scanned: i64,
    pub updated: i64,
    pub no_sidecar: i64,
    pub errors: i64,
}
