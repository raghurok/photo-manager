export interface MediaSummary {
  id: number;
  file_path: string;
  file_name: string;
  file_size: number;
  media_type: string;
  photo_taken_ts: number | null;
  thumbnail_path: string | null;
  album_title: string | null;
}

export interface MediaDetail extends MediaSummary {
  extension: string;
  title: string | null;
  description: string | null;
  creation_ts: number | null;
  latitude: number | null;
  longitude: number | null;
  altitude: number | null;
  image_views: number | null;
  google_url: string | null;
  origin_type: string | null;
  device_type: string | null;
  exif_width: number | null;
  exif_height: number | null;
  exif_camera_make: string | null;
  exif_camera_model: string | null;
  exif_date_ts: number | null;
  album_id: number | null;
  file_hash: string | null;
  people: string[];
  in_duplicate_group: boolean;
}

export interface PersonSummary {
  id: number;
  name: string;
  count: number;
}

export interface AlbumSummary {
  id: number;
  title: string;
  count: number;
}

export interface LibraryStats {
  total: number;
  photos: number;
  videos: number;
  total_size_bytes: number;
  people_count: number;
  duplicate_groups: number;
  indexed: boolean;
}

export interface IndexProgress {
  total: number;
  done: number;
  errors: number;
  phase: string;
  running: boolean;
}

export interface SearchFilters {
  people?: number[];
  media_type?: string;
  min_size_bytes?: number;
  max_size_bytes?: number;
  date_from?: number;
  date_to?: number;
  album_id?: number;
  limit?: number;
  offset?: number;
}

export interface DuplicateGroup {
  group_id: number;
  match_type: string;
  items: DuplicateItem[];
}

export interface DuplicateItem {
  id: number;
  file_path: string;
  file_name: string;
  file_size: number;
  thumbnail_path: string | null;
  photo_taken_ts: number | null;
  album_title: string | null;
}
