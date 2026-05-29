import type { EmoteSpec } from "../types/emote";

interface Props {
  emotes: EmoteSpec[];
  activeIndex: number;
  onSelect: (i: number) => void;
  onDelete: (i: number) => void;
}

export default function EmoteLibrary({ emotes, activeIndex, onSelect, onDelete }: Props) {
  return (
    <div className="space-y-1">
      <h2 className="text-xs uppercase tracking-wide text-gray-500 mb-2">エモート一覧 ({emotes.length})</h2>
      {emotes.map((e, i) => (
        <div
          key={i}
          className={`flex items-center justify-between px-2 py-1.5 rounded cursor-pointer ${
            i === activeIndex ? "bg-accent text-black" : "bg-surface hover:bg-panel"
          }`}
          onClick={() => onSelect(i)}
          data-testid="library-item"
        >
          <div className="min-w-0">
            <div className="truncate text-sm">{e.displayName || e.name}</div>
            <div className="truncate text-xs opacity-70 font-mono">{e.name}</div>
          </div>
          <button
            aria-label={`delete-${i}`}
            className="text-red-400 px-1 shrink-0"
            onClick={(ev) => {
              ev.stopPropagation();
              onDelete(i);
            }}
          >
            ✕
          </button>
        </div>
      ))}
      {emotes.length === 0 && <p className="text-gray-500 text-sm">まだありません</p>}
    </div>
  );
}
