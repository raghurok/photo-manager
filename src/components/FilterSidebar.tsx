import { useTauriCommand } from "../hooks/useTauriCommand";
import type { SearchFilters, PersonSummary, AlbumSummary } from "../types";

interface Props {
  filters: SearchFilters;
  onChange: (f: SearchFilters) => void;
}

const SORT_OPTIONS = [
  { label: "Date ↓", value: "date_desc" },
  { label: "Date ↑", value: "date_asc" },
  { label: "Size ↓", value: "size_desc" },
  { label: "Size ↑", value: "size_asc" },
];

const SIZE_PRESETS = [
  { label: "Any size", min: undefined, max: undefined },
  { label: "Small (<1 MB)", min: undefined, max: 1_000_000 },
  { label: "Medium (1–50 MB)", min: 1_000_000, max: 50_000_000 },
  { label: "Large (>50 MB)", min: 50_000_000, max: undefined },
];

export default function FilterSidebar({ filters, onChange }: Props) {
  const { data: people } = useTauriCommand<PersonSummary[]>("get_people");
  const { data: albums } = useTauriCommand<AlbumSummary[]>("get_albums");

  function togglePerson(id: number) {
    const cur = filters.people ?? [];
    const next = cur.includes(id) ? cur.filter((p) => p !== id) : [...cur, id];
    onChange({ ...filters, people: next.length ? next : undefined });
  }

  function setType(t: string | undefined) {
    onChange({ ...filters, media_type: t });
  }

  function setSort(value: string) {
    onChange({ ...filters, sort_by: value === "date_desc" ? undefined : value });
  }

  function setSize(min?: number, max?: number) {
    onChange({ ...filters, min_size_bytes: min, max_size_bytes: max });
  }

  function setAlbum(id: number | undefined) {
    onChange({ ...filters, album_id: id });
  }

  function clearAll() {
    onChange({});
  }

  const hasFilters = Object.keys(filters).some(
    (k) => filters[k as keyof SearchFilters] !== undefined &&
      (Array.isArray(filters[k as keyof SearchFilters])
        ? (filters[k as keyof SearchFilters] as unknown[]).length > 0
        : true)
  );

  return (
    <aside className="w-56 shrink-0 bg-gray-850 border-r border-gray-700 flex flex-col overflow-y-auto" style={{ background: "#111827" }}>
      <div className="p-3 flex items-center justify-between border-b border-gray-700">
        <span className="text-sm font-semibold text-gray-200">Filters</span>
        {hasFilters && (
          <button onClick={clearAll} className="text-xs text-blue-400 hover:text-blue-300">Clear all</button>
        )}
      </div>

      {/* Sort */}
      <Section title="Sort">
        <div className="grid grid-cols-2 gap-1">
          {SORT_OPTIONS.map((opt) => {
            const active = (filters.sort_by ?? "date_desc") === opt.value;
            return (
              <button
                key={opt.value}
                onClick={() => setSort(opt.value)}
                className={`py-1 text-xs rounded ${active ? "bg-blue-600 text-white" : "bg-gray-700 text-gray-300 hover:bg-gray-600"}`}
              >
                {opt.label}
              </button>
            );
          })}
        </div>
      </Section>

      {/* Type */}
      <Section title="Type">
        <div className="flex gap-1">
          {["All", "photo", "video"].map((t) => (
            <button
              key={t}
              onClick={() => setType(t === "All" ? undefined : t)}
              className={`flex-1 py-1 text-xs rounded ${
                (t === "All" ? !filters.media_type : filters.media_type === t)
                  ? "bg-blue-600 text-white"
                  : "bg-gray-700 text-gray-300 hover:bg-gray-600"
              }`}
            >
              {t === "All" ? "All" : t.charAt(0).toUpperCase() + t.slice(1)}
            </button>
          ))}
        </div>
      </Section>

      {/* Size */}
      <Section title="Size">
        <div className="space-y-1">
          {SIZE_PRESETS.map((p) => (
            <button
              key={p.label}
              onClick={() => setSize(p.min, p.max)}
              className={`w-full text-left px-2 py-1 text-xs rounded ${
                filters.min_size_bytes === p.min && filters.max_size_bytes === p.max
                  ? "bg-blue-600 text-white"
                  : "text-gray-300 hover:bg-gray-700"
              }`}
            >
              {p.label}
            </button>
          ))}
        </div>
      </Section>

      {/* People */}
      {people && people.length > 0 && (
        <Section title="People">
          <div className="space-y-1 max-h-48 overflow-y-auto">
            {people.map((p) => {
              const checked = (filters.people ?? []).includes(p.id);
              return (
                <label key={p.id} className="flex items-center gap-2 cursor-pointer group">
                  <input
                    type="checkbox"
                    checked={checked}
                    onChange={() => togglePerson(p.id)}
                    className="accent-blue-500 cursor-pointer"
                  />
                  <span className="text-xs text-gray-300 group-hover:text-white flex-1 truncate">{p.name}</span>
                  <span className="text-xs text-gray-500">{p.count}</span>
                </label>
              );
            })}
          </div>
        </Section>
      )}

      {/* Albums */}
      {albums && albums.length > 0 && (
        <Section title="Albums">
          <div className="space-y-1 max-h-48 overflow-y-auto">
            <button
              onClick={() => setAlbum(undefined)}
              className={`w-full text-left px-2 py-1 text-xs rounded ${!filters.album_id ? "bg-blue-600 text-white" : "text-gray-300 hover:bg-gray-700"}`}
            >
              All albums
            </button>
            {albums.map((a) => (
              <button
                key={a.id}
                onClick={() => setAlbum(a.id)}
                className={`w-full text-left px-2 py-1 text-xs rounded flex justify-between ${
                  filters.album_id === a.id ? "bg-blue-600 text-white" : "text-gray-300 hover:bg-gray-700"
                }`}
              >
                <span className="truncate">{a.title}</span>
                <span className="text-gray-400 ml-1">{a.count}</span>
              </button>
            ))}
          </div>
        </Section>
      )}
    </aside>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="p-3 border-b border-gray-700">
      <p className="text-xs font-semibold text-gray-400 uppercase tracking-wide mb-2">{title}</p>
      {children}
    </div>
  );
}
