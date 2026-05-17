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
  onSelect: (id: number) => void;
  loadMore: () => void;
}

function GridCell({ columnIndex, rowIndex, style, data }: GridChildComponentProps<CellData>) {
  const { items, cols, selectedId, onSelect, loadMore } = data;
  const idx = rowIndex * cols + columnIndex;

  if (idx >= items.length) {
    if (columnIndex === 0) {
      return <div style={style} ref={(el) => { if (el) loadMore(); }} />;
    }
    return <div style={style} />;
  }

  const item = items[idx];
  const src = item.thumbnail_path ? convertFileSrc(item.thumbnail_path) : null;

  return (
    <div style={style} className="p-0.5 cursor-pointer group" onClick={() => onSelect(item.id)}>
      <div className={`w-full h-full relative overflow-hidden rounded ${selectedId === item.id ? "ring-2 ring-blue-500" : ""}`}>
        {src ? (
          <img
            src={src}
            alt={item.file_name}
            className="w-full h-full object-cover group-hover:opacity-90 transition-opacity"
            loading="lazy"
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
}

export default function Gallery({ filters, onSelect, selectedId }: Props) {
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

  // Stable ref so the sentinel div never causes a remount loop when loadMore changes.
  const loadMoreRef = useRef(loadMore);
  loadMoreRef.current = loadMore;
  const stableLoadMore = useCallback(() => loadMoreRef.current(), []);

  const cols = Math.max(1, Math.floor(dims.width / CELL_SIZE));
  const rows = Math.ceil(items.length / cols) + (hasMore ? 1 : 0);

  const cellData = useMemo<CellData>(
    () => ({ items, cols, selectedId, onSelect, loadMore: stableLoadMore }),
    [items, cols, selectedId, onSelect, stableLoadMore],
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
