import { useState } from "react";
import { useTauriCommand, usePolling } from "./hooks/useTauriCommand";
import Gallery from "./components/Gallery";
import FilterSidebar from "./components/FilterSidebar";
import DuplicatesView from "./components/DuplicatesView";
import PhotoDetail from "./components/PhotoDetail";
import IndexProgress from "./components/IndexProgress";
import StatsBar from "./components/StatsBar";
import type { SearchFilters, LibraryStats, IndexProgress as ProgressType } from "./types";

export default function App() {
  const [view, setView] = useState<"gallery" | "duplicates">("gallery");
  const [filters, setFilters] = useState<SearchFilters>({});
  const [selectedId, setSelectedId] = useState<number | null>(null);

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
              <Gallery filters={filters} onSelect={setSelectedId} selectedId={selectedId} />
              {selectedId != null && (
                <PhotoDetail
                  id={selectedId}
                  onClose={() => setSelectedId(null)}
                  onDelete={() => { setSelectedId(null); refetchStats(); }}
                />
              )}
            </>
          )}
          {view === "duplicates" && (
            <DuplicatesView onDelete={refetchStats} />
          )}
        </div>
      )}
    </div>
  );
}
