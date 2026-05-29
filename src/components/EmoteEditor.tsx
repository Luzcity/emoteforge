import type { ClipRef, EmoteSpec } from "../types/emote";
import { defaultClip } from "../types/emote";
import type { ValidationIssue, CatalogEntry } from "../lib/api";
import ClipList from "./ClipList";
import FlagControls from "./FlagControls";
import CatalogSearch from "./CatalogSearch";
import PropFacialControls from "./PropFacialControls";

interface Props {
  spec: EmoteSpec;
  issues: ValidationIssue[];
  onChange: (spec: EmoteSpec) => void;
  onPreview: () => void;
  onStop: () => void;
  previewing: boolean;
}

export default function EmoteEditor({
  spec,
  issues,
  onChange,
  onPreview,
  onStop,
  previewing,
}: Props) {
  const patch = (p: Partial<EmoteSpec>) => onChange({ ...spec, ...p });
  const setClips = (clips: ClipRef[]) => patch({ clips });
  const addCatalogClip = (e: CatalogEntry) =>
    setClips([...spec.clips, defaultClip(e.dict, e.clip)]);
  const addEmptyClip = () => setClips([...spec.clips, defaultClip("", "")]);

  return (
    <div className="space-y-4">
      <div className="flex gap-3">
        <label className="flex-1 text-sm">
          表示名
          <input
            className="w-full bg-surface px-2 py-1 rounded mt-1"
            aria-label="displayName"
            value={spec.displayName}
            onChange={(e) => patch({ displayName: e.target.value })}
          />
        </label>
        <label className="flex-1 text-sm">
          識別子 (name)
          <input
            className="w-full bg-surface px-2 py-1 rounded mt-1 font-mono"
            aria-label="name"
            value={spec.name}
            onChange={(e) => patch({ name: e.target.value })}
          />
        </label>
      </div>

      <FlagControls spec={spec} onChange={patch} />

      <div>
        <div className="flex items-center justify-between mb-2">
          <h3 className="text-xs uppercase tracking-wide text-gray-500">クリップ列</h3>
          <button className="text-xs px-2 py-0.5 rounded bg-surface" onClick={addEmptyClip}>
            ＋ 空のクリップ
          </button>
        </div>
        <ClipList clips={spec.clips} onChange={setClips} />
        <div className="mt-2">
          <CatalogSearch onAdd={addCatalogClip} />
        </div>
      </div>

      <div>
        <h3 className="text-xs uppercase tracking-wide text-gray-500 mb-2">プロップ / 表情</h3>
        <PropFacialControls spec={spec} onChange={patch} />
      </div>

      {issues.length > 0 && (
        <div className="bg-red-950/50 border border-red-700 rounded p-2 text-sm" role="alert">
          <p className="text-red-300 font-medium mb-1">検証で問題が見つかりました:</p>
          <ul className="space-y-1">
            {issues.map((iss, i) => (
              <li key={i} className="text-red-200">
                <span className="font-mono">{iss.field}</span>: {iss.message}
                {iss.suggestions.length > 0 && (
                  <div className="text-xs text-gray-400">
                    候補: {iss.suggestions.slice(0, 5).join(" / ")}
                  </div>
                )}
              </li>
            ))}
          </ul>
        </div>
      )}

      <div className="flex gap-2">
        <button
          className="px-4 py-2 rounded bg-accent text-black font-medium disabled:opacity-50"
          onClick={onPreview}
          disabled={previewing || spec.clips.length === 0}
        >
          実ゲームでプレビュー
        </button>
        <button className="px-4 py-2 rounded bg-panel" onClick={onStop}>
          停止
        </button>
      </div>
    </div>
  );
}
