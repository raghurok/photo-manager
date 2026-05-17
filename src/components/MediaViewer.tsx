import { useEffect, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { invoke } from "@tauri-apps/api/core";

export interface ViewerItem {
  filePath: string;
  mediaType: string;
  fileName: string;
}

interface Props {
  items: ViewerItem[];
  initialIndex: number;
  onClose: () => void;
}

function isHeic(fileName: string) {
  const ext = fileName.split(".").pop()?.toLowerCase() ?? "";
  return ext === "heic" || ext === "heif";
}

export default function MediaViewer({ items, initialIndex, onClose }: Props) {
  const [index, setIndex] = useState(initialIndex);
  const [displaySrc, setDisplaySrc] = useState<string | null>(null);
  const [converting, setConverting] = useState(false);

  const current = items[index];
  const hasPrev = index > 0;
  const hasNext = index < items.length - 1;

  // Resolve the display URL whenever the current item changes.
  // HEIC files are decoded server-side and cached; everything else goes directly via asset protocol.
  useEffect(() => {
    if (!current) return;
    setDisplaySrc(null);

    if (isHeic(current.fileName)) {
      setConverting(true);
      invoke<string>("decode_heic_for_viewer", { path: current.filePath })
        .then((jpegPath) => setDisplaySrc(convertFileSrc(jpegPath, "localfile")))
        .catch(console.error)
        .finally(() => setConverting(false));
    } else {
      setDisplaySrc(convertFileSrc(current.filePath, "localfile"));
    }
  }, [current?.filePath]);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
      if (e.key === "ArrowLeft") setIndex((i) => Math.max(0, i - 1));
      if (e.key === "ArrowRight") setIndex((i) => Math.min(items.length - 1, i + 1));
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [onClose, items.length]);

  if (!current) return null;

  return (
    <div className="fixed inset-0 z-50 flex flex-col bg-black/95" onClick={onClose}>
      {/* Top bar */}
      <div
        className="shrink-0 flex items-center gap-3 px-4 py-2 bg-black/60"
        onClick={(e) => e.stopPropagation()}
      >
        <span className="text-gray-400 text-xs tabular-nums">{index + 1} / {items.length}</span>
        <span className="text-gray-200 text-sm truncate flex-1">{current.fileName}</span>
        {converting && <span className="text-yellow-400 text-xs">Converting HEIC…</span>}
        <button onClick={onClose} className="text-gray-400 hover:text-white text-2xl leading-none" title="Close (Esc)">
          &times;
        </button>
      </div>

      {/* Media + nav */}
      <div className="flex-1 flex items-center overflow-hidden" onClick={(e) => e.stopPropagation()}>
        {/* Prev */}
        <button
          onClick={() => setIndex((i) => Math.max(0, i - 1))}
          disabled={!hasPrev}
          className="shrink-0 w-14 h-full flex items-center justify-center text-white text-3xl hover:bg-white/10 disabled:opacity-20 disabled:cursor-default transition-colors"
          title="Previous (←)"
        >
          ‹
        </button>

        {/* Content */}
        <div className="flex-1 h-full flex items-center justify-center overflow-hidden">
          {converting && (
            <div className="text-gray-400 text-sm">Decoding HEIC…</div>
          )}
          {!converting && displaySrc && (
            current.mediaType === "video" ? (
              <video
                key={current.filePath}
                src={displaySrc}
                controls
                autoPlay
                className="max-w-full max-h-full"
              />
            ) : (
              <img
                key={current.filePath}
                src={displaySrc}
                alt={current.fileName}
                className="max-w-full max-h-full object-contain"
                draggable={false}
              />
            )
          )}
        </div>

        {/* Next */}
        <button
          onClick={() => setIndex((i) => Math.min(items.length - 1, i + 1))}
          disabled={!hasNext}
          className="shrink-0 w-14 h-full flex items-center justify-center text-white text-3xl hover:bg-white/10 disabled:opacity-20 disabled:cursor-default transition-colors"
          title="Next (→)"
        >
          ›
        </button>
      </div>
    </div>
  );
}
