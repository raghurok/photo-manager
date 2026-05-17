import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { LibraryStats, TakeoutFixResult } from "../types";

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
  const [fixingTakeout, setFixingTakeout] = useState(false);

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

  async function handleFixTakeout() {
    const dir = await open({ directory: true, multiple: false, title: "Select Google Takeout folder" });
    if (!dir) return;
    setFixingTakeout(true);
    try {
      const result = await invoke<TakeoutFixResult>("fix_google_takeout_timestamps", { path: dir });
      alert(
        `Takeout timestamp fix complete.\n\n` +
        `Updated: ${result.updated.toLocaleString()} files\n` +
        `No sidecar / no date: ${result.no_sidecar.toLocaleString()} files\n` +
        `Errors: ${result.errors}\n` +
        `Total scanned: ${result.total_scanned.toLocaleString()} media files`
      );
    } catch (e) {
      alert(`Error: ${e}`);
    } finally {
      setFixingTakeout(false);
    }
  }

  return (
    <div className="flex items-center gap-4 text-xs text-gray-400">
      <span>{stats.total.toLocaleString()} items</span>
      <span>{fmt(stats.total_size_bytes)}</span>
      <span>{stats.people_count} people</span>
      <button
        onClick={handleFixTakeout}
        disabled={fixingTakeout || busy}
        className="px-3 py-1 text-sm bg-gray-600 hover:bg-gray-500 text-white rounded disabled:opacity-50"
      >
        {fixingTakeout ? "Fixing…" : "Fix Takeout Dates"}
      </button>
      <button
        onClick={handleReindex}
        disabled={busy || fixingTakeout}
        className="px-3 py-1 text-sm bg-blue-700 hover:bg-blue-600 text-white rounded disabled:opacity-50"
      >
        {busy ? "Starting…" : "Re-index Library"}
      </button>
    </div>
  );
}
