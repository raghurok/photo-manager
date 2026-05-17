import type { IndexProgress as ProgressType } from "../types";

interface Props { progress: ProgressType; }

export default function IndexProgress({ progress }: Props) {
  const pct = progress.total > 0
    ? Math.round((progress.done / progress.total) * 100)
    : 0;

  return (
    <div className="bg-gray-800 border-b border-blue-800 px-4 py-2 flex items-center gap-4 shrink-0">
      <div className="flex-1">
        <div className="flex justify-between text-xs text-gray-400 mb-1">
          <span>{progress.phase}</span>
          <span>{progress.done.toLocaleString()} / {progress.total.toLocaleString()} {progress.errors > 0 && `(${progress.errors} errors)`}</span>
        </div>
        <div className="h-1.5 bg-gray-700 rounded-full overflow-hidden">
          <div
            className="h-full bg-blue-500 transition-all duration-300"
            style={{ width: `${pct}%` }}
          />
        </div>
      </div>
      <span className="text-sm font-mono text-blue-400">{pct}%</span>
    </div>
  );
}
