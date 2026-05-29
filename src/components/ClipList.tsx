import type { ClipRef } from "../types/emote";

interface Props {
  clips: ClipRef[];
  onChange: (clips: ClipRef[]) => void;
}

export default function ClipList({ clips, onChange }: Props) {
  const update = (i: number, patch: Partial<ClipRef>) => {
    onChange(clips.map((c, idx) => (idx === i ? { ...c, ...patch } : c)));
  };
  const move = (i: number, dir: -1 | 1) => {
    const j = i + dir;
    if (j < 0 || j >= clips.length) return;
    const next = [...clips];
    [next[i], next[j]] = [next[j], next[i]];
    onChange(next);
  };
  const remove = (i: number) => onChange(clips.filter((_, idx) => idx !== i));

  return (
    <div className="space-y-2">
      {clips.map((c, i) => (
        <div key={i} className="bg-surface rounded p-2 text-sm" data-testid="clip-row">
          <div className="flex items-center gap-2">
            <span className="text-accent font-mono shrink-0">#{i + 1}</span>
            <input
              className="flex-1 bg-panel px-2 py-1 rounded font-mono text-xs"
              aria-label={`clip-${i}-dict`}
              value={c.dict}
              onChange={(e) => update(i, { dict: e.target.value })}
            />
            <input
              className="w-40 bg-panel px-2 py-1 rounded font-mono text-xs"
              aria-label={`clip-${i}-clip`}
              value={c.clip}
              onChange={(e) => update(i, { clip: e.target.value })}
            />
            <button aria-label={`clip-${i}-up`} onClick={() => move(i, -1)} className="px-1">↑</button>
            <button aria-label={`clip-${i}-down`} onClick={() => move(i, 1)} className="px-1">↓</button>
            <button aria-label={`clip-${i}-remove`} onClick={() => remove(i)} className="px-1 text-red-400">✕</button>
          </div>
          <div className="flex gap-3 mt-1 text-xs text-gray-400">
            <label className="flex items-center gap-1">
              速度
              <input
                type="number"
                step="0.1"
                className="w-16 bg-panel px-1 rounded"
                aria-label={`clip-${i}-rate`}
                value={c.playbackRate}
                onChange={(e) => update(i, { playbackRate: parseFloat(e.target.value) || 1 })}
              />
            </label>
            <label className="flex items-center gap-1">
              duration
              <input
                type="number"
                className="w-20 bg-panel px-1 rounded"
                aria-label={`clip-${i}-duration`}
                value={c.duration}
                onChange={(e) => update(i, { duration: parseInt(e.target.value, 10) || -1 })}
              />
            </label>
          </div>
        </div>
      ))}
      {clips.length === 0 && <p className="text-gray-500 text-sm">クリップがありません</p>}
    </div>
  );
}
