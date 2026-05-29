import { useState } from "react";
import * as api from "../lib/api";
import type { CatalogEntry } from "../lib/api";

interface Props {
  onAdd: (entry: CatalogEntry) => void;
}

export default function CatalogSearch({ onAdd }: Props) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<CatalogEntry[]>([]);
  const [searching, setSearching] = useState(false);

  const run = async () => {
    const q = query.trim();
    if (!q) return;
    setSearching(true);
    try {
      setResults(await api.searchCatalog(q, 20));
    } catch {
      setResults([]);
    } finally {
      setSearching(false);
    }
  };

  return (
    <div className="bg-surface rounded p-2">
      <div className="flex gap-2">
        <input
          className="flex-1 bg-panel px-2 py-1 rounded text-sm"
          aria-label="catalog-search"
          placeholder="クリップ検索 (例: dance, drink, salute)"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && run()}
        />
        <button className="px-3 py-1 rounded bg-panel text-sm" onClick={run} disabled={searching}>
          {searching ? "…" : "検索"}
        </button>
      </div>
      {results.length > 0 && (
        <ul className="mt-2 max-h-48 overflow-auto space-y-1" data-testid="catalog-results">
          {results.map((e) => (
            <li
              key={e.key}
              className="flex items-center justify-between gap-2 text-xs bg-panel rounded px-2 py-1"
            >
              <div className="min-w-0">
                <div className="truncate">
                  {e.displayName} <span className="opacity-50">[{e.category}]</span>
                </div>
                <div className="truncate font-mono opacity-60">
                  {e.dict} / {e.clip}
                </div>
              </div>
              <button
                aria-label={`add-${e.key}`}
                className="px-2 py-0.5 rounded bg-accent text-black shrink-0"
                onClick={() => onAdd(e)}
              >
                ＋追加
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
