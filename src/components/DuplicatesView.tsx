import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useTauriCommand } from "../hooks/useTauriCommand";
import type { DuplicateGroup } from "../types";

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
                      src={convertFileSrc(item.thumbnail_path)}
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
