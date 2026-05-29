import type { EmoteSpec, MovementType } from "../types/emote";

interface Props {
  spec: EmoteSpec;
  onChange: (patch: Partial<EmoteSpec>) => void;
}

const MOVEMENT: MovementType[] = ["stationary", "walkable", "actionLoop"];

export default function FlagControls({ spec, onChange }: Props) {
  return (
    <div className="flex flex-wrap gap-4 items-center text-sm">
      <label className="flex items-center gap-2">
        <input
          type="checkbox"
          aria-label="loop"
          checked={spec.loop}
          onChange={(e) => onChange({ loop: e.target.checked })}
        />
        ループ
      </label>
      <label className="flex items-center gap-2">
        <input
          type="checkbox"
          aria-label="upperBodyOnly"
          checked={spec.upperBodyOnly}
          onChange={(e) => onChange({ upperBodyOnly: e.target.checked })}
        />
        上半身のみ
      </label>
      <label className="flex items-center gap-2">
        移動
        <select
          aria-label="movementType"
          className="bg-surface px-2 py-1 rounded"
          value={spec.movementType}
          onChange={(e) => onChange({ movementType: e.target.value as MovementType })}
        >
          {MOVEMENT.map((m) => (
            <option key={m} value={m}>
              {m}
            </option>
          ))}
        </select>
      </label>
    </div>
  );
}
