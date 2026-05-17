import { useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTauriCommand, usePolling } from "./hooks/useTauriCommand";
import Gallery from "./components/Gallery";
import FilterSidebar from "./components/FilterSidebar";
import DuplicatesView from "./components/DuplicatesView";
import PhotoDetail from "./components/PhotoDetail";
import MediaViewer, { type ViewerItem } from "./components/MediaViewer";
import IndexProgress from "./components/IndexProgress";
import StatsBar from "./components/StatsBar";
import type { SearchFilters, LibraryStats, IndexProgress as ProgressType, MediaSummary } from "./types";

export default function App() {
  const [view, setView] = useState<"gallery" | "duplicates">("gallery");
  const [filters, setFilters] = useState<SearchFilters>({});
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [multiSelectMode, setMultiSelectMode] = useState(false);
  const [multiSelectedIds, setMultiSelectedIds] = useState<Set<number>>(new Set());
  const [galleryItems, setGalleryItems] = useState<MediaSummary[]>([]);
  const [viewer, setViewer] = useState<{ items: ViewerItem[]; index: number } | null>(null);
  const [deleting, setDeleting] = useState(false);

  function handleCheckboxClick(id: number) {
    setMultiSelectMode(true);
    setMultiSelectedIds((prev) => {
      const next = new Set(prev);
      next.has(id) ? next.delete(id) : next.add(id);
      return next;
    });
  }

  async function handleMultiDelete() {
    if (!confirm(`Move ${multiSelectedIds.size} photo${multiSelectedIds.size !== 1 ? "s" : ""} to Recycle Bin?`)) return;
    setDeleting(true);
    try {
      await invoke("delete_files", { ids: [...multiSelectedIds] });
      setMultiSelectedIds(new Set());
      setMultiSelectMode(false);
      setSelectedId(null);
      refetchStats();
    } finally {
      setDeleting(false);
    }
  }

  const toViewerItems = (items: MediaSummary[]): ViewerItem[] =>
    items.map((i) => ({ filePath: i.file_path, mediaType: i.media_type, fileName: i.file_name }));

  const handleGalleryView = useCallback((items: MediaSummary[], index: number) => {
    setViewer({ items: toViewerItems(items), index });
  }, []);

  const handleDetailView = useCallback((filePath: string, mediaType: string, fileName: string) => {
    const idx = galleryItems.findIndex((i) => i.file_path === filePath);
    const viewerItems = toViewerItems(galleryItems);
    setViewer({
      items: idx >= 0 ? viewerItems : [{ filePath, mediaType, fileName }],
      index: Math.max(0, idx),
    });
  }, [galleryItems]);

  const { data: stats, refetch: refetchStats } = useTauriCommand<LibraryStats>("get_stats");
  const progress = usePolling<ProgressType>("get_index_progress", 600, true);

  const isIndexing = progress?.running ?? false;
  const needsIndex = stats && !stats.indexed && !isIndexing;

  return (
    <div className="flex flex-col h-screen bg-gray-900 text-gray-100">
      {/* Top bar */}
      <header className="flex items-center gap-4 px-4 py-2 bg-gray-800 border-b border-gray-700 shrink-0">
        <span className="font-semibold text-white text-lg">Photo Manager</span>
        <nav className="flex gap-1 ml-4">
          <button
            onClick={() => setView("gallery")}
            className={`px-3 py-1 rounded text-sm ${view === "gallery" ? "bg-blue-600 text-white" : "text-gray-300 hover:text-white"}`}
          >
            Gallery
          </button>
          <button
            onClick={() => setView("duplicates")}
            className={`px-3 py-1 rounded text-sm ${view === "duplicates" ? "bg-blue-600 text-white" : "text-gray-300 hover:text-white"}`}
          >
            Duplicates {stats && stats.duplicate_groups > 0 && (
              <span className="ml-1 bg-red-500 text-white text-xs rounded-full px-1.5">{stats.duplicate_groups}</span>
            )}
          </button>
        </nav>
        <div className="ml-auto">
          {stats && <StatsBar stats={stats} onReindex={() => setView("gallery")} onAfterIndex={refetchStats} />}
        </div>
      </header>

      {/* Progress overlay */}
      {isIndexing && progress && <IndexProgress progress={progress} />}

      {/* Welcome / first run */}
      {needsIndex && (
        <div className="flex items-center justify-center flex-1">
          <div className="text-center max-w-sm">
            <p className="text-gray-400 text-lg mb-2">No photos indexed yet.</p>
            <p className="text-gray-500 text-sm">Click <strong>Re-index Library</strong> in the top-right to scan your Google Photos folder.</p>
          </div>
        </div>
      )}

      {/* Main area */}
      {(!needsIndex || stats?.indexed) && (
        <div className="flex flex-1 min-h-0">
          {view === "gallery" && (
            <>
              <FilterSidebar filters={filters} onChange={setFilters} />
              <Gallery
                filters={filters}
                onSelect={setSelectedId}
                selectedId={selectedId}
                multiSelectMode={multiSelectMode}
                multiSelectedIds={multiSelectedIds}
                onCheckboxClick={handleCheckboxClick}
                onItemsChange={setGalleryItems}
                onView={handleGalleryView}
              />
              {selectedId != null && (
                <PhotoDetail
                  id={selectedId}
                  onClose={() => setSelectedId(null)}
                  onDelete={() => { setSelectedId(null); refetchStats(); }}
                  onView={handleDetailView}
                />
              )}
            </>
          )}
          {view === "duplicates" && (
            <DuplicatesView onDelete={refetchStats} />
          )}
        </div>
      )}
      {/* Multi-select action bar */}
      {multiSelectedIds.size > 0 && (
        <div className="fixed bottom-5 left-1/2 -translate-x-1/2 z-40 flex items-center gap-3 bg-gray-800 border border-gray-600 rounded-full px-5 py-2.5 shadow-2xl">
          <span className="text-sm text-gray-200 tabular-nums">{multiSelectedIds.size} selected</span>
          <button
            onClick={handleMultiDelete}
            disabled={deleting}
            className="px-3 py-1 text-sm bg-red-700 hover:bg-red-600 text-red-100 rounded-full disabled:opacity-50"
          >
            {deleting ? "Deleting…" : "Delete"}
          </button>
          <button
            onClick={() => { setMultiSelectedIds(new Set()); setMultiSelectMode(false); }}
            className="px-3 py-1 text-sm bg-gray-600 hover:bg-gray-500 text-gray-200 rounded-full"
          >
            Clear
          </button>
        </div>
      )}

      {viewer && (
        <MediaViewer
          items={viewer.items}
          initialIndex={viewer.index}
          onClose={() => setViewer(null)}
        />
      )}
    </div>
  );
}
