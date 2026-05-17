import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { FixedSizeGrid } from "react-window";
import type { GridChildComponentProps } from "react-window";
import type { SearchFilters, MediaSummary } from "../types";

const CELL_SIZE = 180;
const PAGE_SIZE = 200;

interface CellData {
  items: MediaSummary[];
  cols: number;
  selectedId: number | null;
  multiSelectMode: boolean;
  multiSelectedIds: Set<number>;
  onSelect: (id: number) => void;
  onCheckboxClick: (id: number) => void;
  onDoubleClick: (idx: number) => void;
  loadMore: () => void;
}

function GridCell({ columnIndex, rowIndex, style, data }: GridChildComponentProps<CellData>) {
  const { items, cols, selectedId, multiSelectMode, multiSelectedIds, onSelect, onCheckboxClick, onDoubleClick, loadMore } = data;
  const idx = rowIndex * cols + columnIndex;

  if (idx >= items.length) {
    if (columnIndex === 0) {
      return <div style={style} ref={(el) => { if (el) loadMore(); }} />;
    }
    return <div style={style} />;
  }

  const item = items[idx];
  const src = item.thumbnail_path ? convertFileSrc(item.thumbnail_path, "localfile") : null;
  const isMultiSelected = multiSelectedIds.has(item.id);

  return (
    <div
      style={style}
      className="p-0.5 cursor-pointer group"
      onClick={() => multiSelectMode ? onCheckboxClick(item.id) : onSelect(item.id)}
      onDoubleClick={() => { if (!multiSelectMode) onDoubleClick(idx); }}
    >
      <div className={`w-full h-full relative overflow-hidden rounded ${isMultiSelected ? "ring-2 ring-blue-400" : !multiSelectMode && selectedId === item.id ? "ring-2 ring-blue-500" : ""}`}>
        {/* Round checkbox — always visible in multi-select mode, hover-only otherwise */}
        <div
          className={`absolute top-1.5 left-1.5 z-10 w-5 h-5 rounded-full flex items-center justify-center transition-opacity shadow
            ${isMultiSelected ? "opacity-100" : "opacity-0 group-hover:opacity-100"}
            ${isMultiSelected ? "bg-blue-500 border-2 border-blue-400" : "bg-transparent border border-white/60"}`}
          onClick={(e) => { e.stopPropagation(); onCheckboxClick(item.id); }}
        >
          {isMultiSelected && <span className="text-white text-xs leading-none">✓</span>}
        </div>
        {src ? (
          <img
            src={src}
            alt={item.file_name}
            className="w-full h-full object-cover group-hover:opacity-90 transition-opacity"
          />
        ) : (
          <div className="w-full h-full bg-gray-700 flex items-center justify-center text-gray-400 text-xs">
            {item.media_type === "video" ? "▶ Video" : "No thumb"}
          </div>
        )}
        {item.media_type === "video" && (
          <div className="absolute bottom-1 right-1 bg-black/60 text-white text-xs px-1 rounded">▶</div>
        )}
      </div>
    </div>
  );
}

interface Props {
  filters: SearchFilters;
  onSelect: (id: number) => void;
  selectedId: number | null;
  multiSelectMode: boolean;
  multiSelectedIds: Set<number>;
  onCheckboxClick: (id: number) => void;
  onItemsChange?: (items: MediaSummary[]) => void;
  onView?: (items: MediaSummary[], index: number) => void;
}

export default function Gallery({ filters, onSelect, selectedId, multiSelectMode, multiSelectedIds, onCheckboxClick, onItemsChange, onView }: Props) {
  const [items, setItems] = useState<MediaSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [hasMore, setHasMore] = useState(true);
  const containerRef = useRef<HTMLDivElement>(null);
  const [dims, setDims] = useState({ width: 800, height: 600 });

  useEffect(() => {
    if (!containerRef.current) return;
    const obs = new ResizeObserver((entries) => {
      const { width, height } = entries[0].contentRect;
      setDims({ width, height });
    });
    obs.observe(containerRef.current);
    return () => obs.disconnect();
  }, []);

  useEffect(() => {
    setItems([]);
    setHasMore(true);
    setLoading(true);
    invoke<MediaSummary[]>("query_media", { filters: { ...filters, limit: PAGE_SIZE, offset: 0 } })
      .then((data) => {
        setItems(data);
        setHasMore(data.length === PAGE_SIZE);
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [JSON.stringify(filters)]); // eslint-disable-line react-hooks/exhaustive-deps

  const loadMore = useCallback(() => {
    if (loading || !hasMore) return;
    setLoading(true);
    invoke<MediaSummary[]>("query_media", { filters: { ...filters, limit: PAGE_SIZE, offset: items.length } })
      .then((data) => {
        setItems((prev) => [...prev, ...data]);
        setHasMore(data.length === PAGE_SIZE);
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [loading, hasMore, items.length, filters]);

  // Notify parent whenever the visible item list changes (used for viewer navigation).
  const onItemsChangeRef = useRef(onItemsChange);
  onItemsChangeRef.current = onItemsChange;
  useEffect(() => { onItemsChangeRef.current?.(items); }, [items]);

  // Stable ref so the sentinel div never causes a remount loop when loadMore changes.
  const loadMoreRef = useRef(loadMore);
  loadMoreRef.current = loadMore;
  const stableLoadMore = useCallback(() => loadMoreRef.current(), []);

  const onViewRef = useRef(onView);
  onViewRef.current = onView;
  const stableDoubleClick = useCallback((idx: number) => onViewRef.current?.(items, idx), [items]);

  const cols = Math.max(1, Math.floor(dims.width / CELL_SIZE));
  const rows = Math.ceil(items.length / cols) + (hasMore ? 1 : 0);

  const cellData = useMemo<CellData>(
    () => ({ items, cols, selectedId, multiSelectMode, multiSelectedIds, onSelect, onCheckboxClick, onDoubleClick: stableDoubleClick, loadMore: stableLoadMore }),
    [items, cols, selectedId, multiSelectMode, multiSelectedIds, onSelect, onCheckboxClick, stableDoubleClick, stableLoadMore],
  );

  return (
    <div ref={containerRef} className="flex-1 min-w-0 overflow-hidden">
      {loading && items.length === 0 && (
        <div className="flex items-center justify-center h-full text-gray-400">Loading…</div>
      )}
      {!loading && items.length === 0 && (
        <div className="flex items-center justify-center h-full text-gray-400">No media matches your filters.</div>
      )}
      {items.length > 0 && (
        <FixedSizeGrid
          width={dims.width}
          height={dims.height}
          columnCount={cols}
          rowCount={rows}
          columnWidth={CELL_SIZE}
          rowHeight={CELL_SIZE}
          itemData={cellData}
        >
          {GridCell}
        </FixedSizeGrid>
      )}
    </div>
  );
}
