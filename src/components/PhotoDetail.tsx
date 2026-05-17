import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useTauriCommand } from "../hooks/useTauriCommand";
import type { MediaDetail } from "../types";

interface Props {
  id: number;
  onClose: () => void;
  onDelete: () => void;
}

function fmtDate(ts: number | null): string {
  if (!ts) return "—";
  return new Date(ts * 1000).toLocaleString();
}

function fmtSize(bytes: number): string {
  if (bytes > 1e9) return (bytes / 1e9).toFixed(2) + " GB";
  if (bytes > 1e6) return (bytes / 1e6).toFixed(2) + " MB";
  return (bytes / 1e3).toFixed(0) + " KB";
}

export default function PhotoDetail({ id, onClose, onDelete }: Props) {
  const { data: detail, loading } = useTauriCommand<MediaDetail>("get_media_detail", { id }, [id]);

  async function handleDelete() {
    if (!confirm(`Move "${detail?.file_name}" to Recycle Bin?`)) return;
    await invoke("delete_file", { id });
    onDelete();
  }

  async function handleOpenInExplorer() {
    if (!detail) return;
    await invoke("open_in_explorer", { path: detail.file_path });
  }

  return (
    <aside className="w-72 shrink-0 bg-gray-800 border-l border-gray-700 flex flex-col overflow-y-auto">
      <div className="flex items-center justify-between px-3 py-2 border-b border-gray-700">
        <span className="text-sm font-semibold text-gray-200">Details</span>
        <button onClick={onClose} className="text-gray-400 hover:text-white text-lg leading-none">&times;</button>
      </div>

      {loading && <div className="flex-1 flex items-center justify-center text-gray-400">Loading…</div>}

      {detail && (
        <>
          {/* Thumbnail / preview */}
          <div className="bg-gray-900 flex items-center justify-center" style={{ height: 200 }}>
            {detail.thumbnail_path ? (
              <img
                src={convertFileSrc(detail.thumbnail_path)}
                alt={detail.file_name}
                className="max-w-full max-h-full object-contain"
              />
            ) : (
              <span className="text-gray-500 text-sm">{detail.media_type === "video" ? "▶ Video" : "No thumbnail"}</span>
            )}
          </div>

          <div className="p-3 space-y-3 text-sm">
            <p className="font-medium text-gray-100 break-all">{detail.file_name}</p>

            <Row label="Date taken" value={fmtDate(detail.photo_taken_ts ?? detail.exif_date_ts)} />
            <Row label="Size" value={fmtSize(detail.file_size)} />
            <Row label="Type" value={detail.extension.toUpperCase()} />

            {(detail.exif_width && detail.exif_height) && (
              <Row label="Resolution" value={`${detail.exif_width} × ${detail.exif_height}`} />
            )}
            {detail.exif_camera_make && (
              <Row label="Camera" value={`${detail.exif_camera_make} ${detail.exif_camera_model ?? ""}`} />
            )}
            {detail.album_title && <Row label="Album" value={detail.album_title} />}
            {detail.description && <Row label="Description" value={detail.description} />}
            {detail.origin_type && <Row label="Origin" value={detail.origin_type} />}
            {detail.device_type && <Row label="Device" value={detail.device_type} />}

            {(detail.latitude != null && detail.longitude != null) && (
              <Row label="Location" value={`${detail.latitude.toFixed(5)}, ${detail.longitude.toFixed(5)}`} />
            )}

            {detail.people.length > 0 && (
              <div>
                <p className="text-xs text-gray-400 mb-1">People</p>
                <div className="flex flex-wrap gap-1">
                  {detail.people.map((name) => (
                    <span key={name} className="px-2 py-0.5 bg-blue-800 text-blue-100 rounded-full text-xs">{name}</span>
                  ))}
                </div>
              </div>
            )}

            {detail.in_duplicate_group && (
              <p className="text-xs text-yellow-400">⚠ This photo is in a duplicate group.</p>
            )}

            <div className="flex gap-2 pt-2">
              <button
                onClick={handleOpenInExplorer}
                className="flex-1 py-1.5 text-xs bg-gray-700 hover:bg-gray-600 rounded text-gray-200"
              >
                Open in Explorer
              </button>
              <button
                onClick={handleDelete}
                className="flex-1 py-1.5 text-xs bg-red-800 hover:bg-red-700 rounded text-red-100"
              >
                Delete
              </button>
            </div>
          </div>
        </>
      )}
    </aside>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-xs text-gray-400">{label}</p>
      <p className="text-sm text-gray-100 break-words">{value}</p>
    </div>
  );
}
