import type { EmoteSpec, Prop, Facial } from "../types/emote";

interface Props {
  spec: EmoteSpec;
  onChange: (patch: Partial<EmoteSpec>) => void;
}

const DEFAULT_PROP: Prop = { model: "prop_cs_bottle_01", bone: 18905, offset: [0, 0, 0], rot: [0, 0, 0] };
const DEFAULT_FACIAL: Facial = { dict: "facials@gen_male@base", clip: "mood_happy_1" };

export default function PropFacialControls({ spec, onChange }: Props) {
  const setProp = (p: Prop | undefined) => onChange({ prop: p });
  const setFacial = (f: Facial | undefined) => onChange({ facial: f });

  const updateVec = (key: "offset" | "rot", i: number, v: number) => {
    if (!spec.prop) return;
    const arr = [...spec.prop[key]] as [number, number, number];
    arr[i] = v;
    setProp({ ...spec.prop, [key]: arr });
  };

  return (
    <div className="space-y-3 text-sm">
      {/* prop */}
      <div className="bg-surface rounded p-2">
        <label className="flex items-center gap-2 mb-2">
          <input
            type="checkbox"
            aria-label="prop-enabled"
            checked={!!spec.prop}
            onChange={(e) => setProp(e.target.checked ? DEFAULT_PROP : undefined)}
          />
          プロップを使う
        </label>
        {spec.prop && (
          <div className="space-y-2">
            <div className="flex gap-2">
              <label className="flex-1 text-xs">
                モデル
                <input
                  className="w-full bg-panel px-2 py-1 rounded font-mono"
                  aria-label="prop-model"
                  value={spec.prop.model}
                  onChange={(e) => setProp({ ...spec.prop!, model: e.target.value })}
                />
              </label>
              <label className="w-28 text-xs">
                ボーン
                <input
                  type="number"
                  className="w-full bg-panel px-2 py-1 rounded"
                  aria-label="prop-bone"
                  value={spec.prop.bone}
                  onChange={(e) => setProp({ ...spec.prop!, bone: parseInt(e.target.value, 10) || 0 })}
                />
              </label>
            </div>
            <div className="flex gap-3 text-xs">
              {(["offset", "rot"] as const).map((key) => (
                <div key={key}>
                  <span className="opacity-60">{key}</span>
                  <div className="flex gap-1">
                    {[0, 1, 2].map((i) => (
                      <input
                        key={i}
                        type="number"
                        step="0.01"
                        className="w-14 bg-panel px-1 rounded"
                        aria-label={`prop-${key}-${i}`}
                        value={spec.prop![key][i]}
                        onChange={(e) => updateVec(key, i, parseFloat(e.target.value) || 0)}
                      />
                    ))}
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      {/* facial */}
      <div className="bg-surface rounded p-2">
        <label className="flex items-center gap-2 mb-2">
          <input
            type="checkbox"
            aria-label="facial-enabled"
            checked={!!spec.facial}
            onChange={(e) => setFacial(e.target.checked ? DEFAULT_FACIAL : undefined)}
          />
          表情を使う
        </label>
        {spec.facial && (
          <div className="flex gap-2">
            <label className="flex-1 text-xs">
              dict
              <input
                className="w-full bg-panel px-2 py-1 rounded font-mono"
                aria-label="facial-dict"
                value={spec.facial.dict}
                onChange={(e) => setFacial({ ...spec.facial!, dict: e.target.value })}
              />
            </label>
            <label className="flex-1 text-xs">
              clip
              <input
                className="w-full bg-panel px-2 py-1 rounded font-mono"
                aria-label="facial-clip"
                value={spec.facial.clip}
                onChange={(e) => setFacial({ ...spec.facial!, clip: e.target.value })}
              />
            </label>
          </div>
        )}
      </div>
    </div>
  );
}
