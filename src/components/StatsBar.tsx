import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { LibraryStats } from "../types";

function fmt(bytes: number) {
  if (bytes > 1e9) return (bytes / 1e9).toFixed(1) + " GB";
  if (bytes > 1e6) return (bytes / 1e6).toFixed(1) + " MB";
  return (bytes / 1e3).toFixed(0) + " KB";
}

interface Props {
  stats: LibraryStats;
  onReindex: () => void;
  onAfterIndex: () => void;
}

export default function StatsBar({ stats, onAfterIndex }: Props) {
  const [busy, setBusy] = useState(false);

  async function handleReindex() {
    const dir = await open({ directory: true, multiple: false, title: "Select Google Photos folder" });
    if (!dir) return;
    setBusy(true);
    try {
      await invoke("scan_library", { path: dir });
      onAfterIndex();
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="flex items-center gap-4 text-xs text-gray-400">
      <span>{stats.total.toLocaleString()} items</span>
      <span>{fmt(stats.total_size_bytes)}</span>
      <span>{stats.people_count} people</span>
      <button
        onClick={handleReindex}
        disabled={busy}
        className="px-3 py-1 text-sm bg-blue-700 hover:bg-blue-600 text-white rounded disabled:opacity-50"
      >
        {busy ? "Starting…" : "Re-index Library"}
      </button>
    </div>
  );
}
