import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useTauriCommand } from "../hooks/useTauriCommand";
import type { DuplicateGroup, CleanupResult } from "../types";

interface Props { onDelete: () => void; }

function fmtSize(bytes: number) {
  if (bytes > 1e9) return (bytes / 1e9).toFixed(2) + " GB";
  if (bytes > 1e6) return (bytes / 1e6).toFixed(2) + " MB";
  return (bytes / 1e3).toFixed(0) + " KB";
}

function fmtDate(ts: number | null) {
  if (!ts) return "—";
  return new Date(ts * 1000).toLocaleDateString();
}

export default function DuplicatesView({ onDelete }: Props) {
  const { data: groups, loading, refetch } = useTauriCommand<DuplicateGroup[]>("get_duplicates");
  const [deleting, setDeleting] = useState<Set<number>>(new Set());
  const [cleanupResult, setCleanupResult] = useState<CleanupResult | null>(null);
  const [cleaningUp, setCleaningUp] = useState(false);

  async function handleDelete(id: number) {
    setDeleting((s) => new Set(s).add(id));
    try {
      await invoke("delete_file", { id });
      onDelete();
      await refetch();
    } finally {
      setDeleting((s) => { const n = new Set(s); n.delete(id); return n; });
    }
  }

  async function handleCleanup(dryRun: boolean) {
    setCleaningUp(true);
    try {
      const result = await invoke<CleanupResult>("cleanup_name_duplicates", { dryRun });
      setCleanupResult(result);
      if (!dryRun) {
        onDelete();
        await refetch();
      }
    } catch (e) {
      console.error(e);
    } finally {
      setCleaningUp(false);
    }
  }

  if (loading) {
    return <div className="flex-1 flex items-center justify-center text-gray-400">Loading duplicates…</div>;
  }

  if (!groups || groups.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center text-gray-400">
          <p className="text-lg mb-1">No duplicates found.</p>
          <p className="text-sm">Run Re-index Library if you haven't indexed yet.</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto p-4 space-y-6">
      {/* Bulk Cleanup Panel */}
      <div className="bg-gray-800 border border-gray-700 rounded-lg p-4">
        <h3 className="text-sm font-semibold text-gray-200 mb-1">Bulk Cleanup</h3>
        <p className="text-xs text-gray-400 mb-3">
          Deletes all-but-one copy from every duplicate group (exact hash matches and EXIF metadata matches).
          Prefers keeping the copy that belongs to an album; falls back to the earliest-indexed copy.
        </p>
        <div className="flex flex-wrap gap-2 mb-3">
          <button
            onClick={() => handleCleanup(true)}
            disabled={cleaningUp}
            className="px-3 py-1.5 text-xs bg-gray-600 hover:bg-gray-500 text-gray-100 rounded disabled:opacity-50"
          >
            {cleaningUp && cleanupResult?.dry_run !== false ? "Analyzing…" : "Dry Run"}
          </button>
          {cleanupResult?.dry_run && cleanupResult.files_deleted > 0 && (
            <button
              onClick={() => handleCleanup(false)}
              disabled={cleaningUp}
              className="px-3 py-1.5 text-xs bg-red-700 hover:bg-red-600 text-red-100 rounded disabled:opacity-50"
            >
              {cleaningUp ? "Deleting…" : `Delete ${cleanupResult.files_deleted} files (free ${fmtSize(cleanupResult.bytes_freed)})`}
            </button>
          )}
        </div>
        {cleanupResult && (
          <div className="space-y-2">
            <div className="flex gap-4 text-xs">
              <span className="text-gray-300">
                <span className="font-medium text-white">{cleanupResult.groups_eligible}</span> groups
              </span>
              <span className="text-gray-300">
                <span className="font-medium text-white">{cleanupResult.files_deleted}</span> files {cleanupResult.dry_run ? "to delete" : "deleted"}
              </span>
              <span className="text-gray-300">
                <span className="font-medium text-white">{fmtSize(cleanupResult.bytes_freed)}</span> {cleanupResult.dry_run ? "to free" : "freed"}
              </span>
            </div>
            {cleanupResult.dry_run && cleanupResult.preview.length > 0 && (
              <details className="text-xs">
                <summary className="cursor-pointer text-gray-400 hover:text-gray-200">
                  Preview (first {cleanupResult.preview.length} groups)
                </summary>
                <div className="mt-2 max-h-48 overflow-y-auto space-y-1 font-mono">
                  {cleanupResult.preview.map((item, i) => (
                    <div key={i} className="text-gray-400">
                      <span className="text-green-400">keep</span> {item.kept_path}
                      {item.deleted_paths.map((p, j) => (
                        <div key={j}><span className="text-red-400">del </span> {p}</div>
                      ))}
                    </div>
                  ))}
                </div>
              </details>
            )}
            {!cleanupResult.dry_run && (
              <p className="text-xs text-green-400">Cleanup complete. Files moved to Recycle Bin.</p>
            )}
          </div>
        )}
      </div>

      <p className="text-gray-400 text-sm">{groups.length} duplicate group{groups.length !== 1 ? "s" : ""} found</p>
      {groups.map((group) => (
        <div key={group.group_id} className="bg-gray-800 rounded-lg p-3">
          <div className="flex items-center gap-2 mb-3">
            <span className={`text-xs px-2 py-0.5 rounded-full ${group.match_type === "hash" ? "bg-red-800 text-red-200" : "bg-yellow-800 text-yellow-200"}`}>
              {group.match_type === "hash" ? "Exact duplicate" : "EXIF match"}
            </span>
            <span className="text-xs text-gray-500">{group.items.length} files</span>
          </div>
          <div className="flex gap-3 overflow-x-auto pb-1">
            {group.items.map((item) => (
              <div key={item.id} className="shrink-0 w-40 bg-gray-700 rounded overflow-hidden">
                <div className="h-32 bg-gray-600 flex items-center justify-center">
                  {item.thumbnail_path ? (
                    <img
                      src={convertFileSrc(item.thumbnail_path, "localfile")}
                      alt={item.file_name}
                      className="w-full h-full object-cover"
                    />
                  ) : (
                    <span className="text-gray-400 text-xs">No thumb</span>
                  )}
                </div>
                <div className="p-2">
                  <p className="text-xs text-gray-200 truncate" title={item.file_name}>{item.file_name}</p>
                  <p className="text-xs text-gray-400">{fmtSize(item.file_size)}</p>
                  <p className="text-xs text-gray-400">{fmtDate(item.photo_taken_ts)}</p>
                  {item.album_title && <p className="text-xs text-blue-300 truncate">{item.album_title}</p>}
                  <button
                    onClick={() => handleDelete(item.id)}
                    disabled={deleting.has(item.id)}
                    className="mt-2 w-full py-1 text-xs bg-red-800 hover:bg-red-700 text-red-100 rounded disabled:opacity-50"
                  >
                    {deleting.has(item.id) ? "Deleting…" : "Delete"}
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
