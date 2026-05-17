use anyhow::Result;
use rusqlite::{Connection, params};
use std::path::PathBuf;

pub fn db_path() -> PathBuf {
    let app_data = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs_home().join("AppData").join("Roaming"));
    app_data.join("photo-manager").join("library.db")
}

fn dirs_home() -> PathBuf {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

pub fn open() -> Result<Connection> {
    let path = db_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(&path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON;")?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY);

        CREATE TABLE IF NOT EXISTS albums (
            id          INTEGER PRIMARY KEY,
            dir_path    TEXT NOT NULL UNIQUE,
            title       TEXT,
            description TEXT,
            access      TEXT,
            date_ts     INTEGER
        );

        CREATE TABLE IF NOT EXISTS media (
            id                INTEGER PRIMARY KEY,
            file_path         TEXT NOT NULL UNIQUE,
            file_name         TEXT NOT NULL,
            file_size         INTEGER NOT NULL,
            file_hash         TEXT,
            media_type        TEXT NOT NULL,
            extension         TEXT NOT NULL,
            title             TEXT,
            description       TEXT,
            photo_taken_ts    INTEGER,
            creation_ts       INTEGER,
            latitude          REAL,
            longitude         REAL,
            altitude          REAL,
            image_views       INTEGER,
            google_url        TEXT,
            origin_type       TEXT,
            device_type       TEXT,
            exif_width        INTEGER,
            exif_height       INTEGER,
            exif_camera_make  TEXT,
            exif_camera_model TEXT,
            exif_date_ts      INTEGER,
            album_id          INTEGER REFERENCES albums(id),
            thumbnail_path    TEXT,
            indexed_at        INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS people (
            id   INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE COLLATE NOCASE
        );

        CREATE TABLE IF NOT EXISTS media_people (
            media_id  INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE,
            person_id INTEGER NOT NULL REFERENCES people(id),
            PRIMARY KEY (media_id, person_id)
        );

        CREATE TABLE IF NOT EXISTS duplicate_groups (
            group_id   INTEGER NOT NULL,
            media_id   INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE,
            match_type TEXT NOT NULL,
            PRIMARY KEY (group_id, media_id)
        );

        CREATE INDEX IF NOT EXISTS idx_media_type    ON media(media_type);
        CREATE INDEX IF NOT EXISTS idx_media_hash    ON media(file_hash);
        CREATE INDEX IF NOT EXISTS idx_media_taken   ON media(photo_taken_ts);
        CREATE INDEX IF NOT EXISTS idx_media_size    ON media(file_size);
        CREATE INDEX IF NOT EXISTS idx_media_album   ON media(album_id);
        CREATE INDEX IF NOT EXISTS idx_media_lat_lon ON media(latitude, longitude);
        CREATE INDEX IF NOT EXISTS idx_mp_person     ON media_people(person_id);
        CREATE INDEX IF NOT EXISTS idx_dup_group     ON duplicate_groups(group_id);
    ")?;
    Ok(())
}

pub fn upsert_album(conn: &Connection, dir_path: &str, title: Option<&str>, description: Option<&str>, access: Option<&str>, date_ts: Option<i64>) -> Result<i64> {
    conn.execute(
        "INSERT INTO albums (dir_path, title, description, access, date_ts)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(dir_path) DO UPDATE SET title=excluded.title, description=excluded.description, access=excluded.access, date_ts=excluded.date_ts",
        params![dir_path, title, description, access, date_ts],
    )?;
    let id: i64 = conn.query_row(
        "SELECT id FROM albums WHERE dir_path = ?1",
        params![dir_path],
        |r| r.get(0),
    )?;
    Ok(id)
}

pub fn upsert_person(conn: &Connection, name: &str) -> Result<i64> {
    conn.execute(
        "INSERT OR IGNORE INTO people (name) VALUES (?1)",
        params![name],
    )?;
    let id: i64 = conn.query_row(
        "SELECT id FROM people WHERE name = ?1 COLLATE NOCASE",
        params![name],
        |r| r.get(0),
    )?;
    Ok(id)
}

pub fn clear_duplicate_groups(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM duplicate_groups", [])?;
    Ok(())
}
