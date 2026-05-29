import { useState } from "react";
import * as api from "../lib/api";
import type { YcdBuildResult } from "../lib/api";
import { pickFile, pickSavePath } from "../lib/dialog";

interface Props {
  onError: (msg: string) => void;
  onStatus: (msg: string) => void;
}

const YCD_FILTERS = [{ name: "ycd.xml", extensions: ["xml"] }];

/** Phase 2/3: BVH 取込 / AI モーション生成 → .ycd.xml 書き出し。
 *  注意: .ycd バイナリ化と FiveM 実ロードは CodeWalker/Sollumz が別途必要な best-effort 出力。 */
export default function MotionPanel({ onError, onStatus }: Props) {
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<YcdBuildResult | null>(null);

  // BVH
  const [bvhPath, setBvhPath] = useState("");
  // AI motion
  const [prompt, setPrompt] = useState("");
  const [runnerBin, setRunnerBin] = useState("python");
  const [runnerArgs, setRunnerArgs] = useState("t2m_runner.py");

  const run = async (fn: () => Promise<YcdBuildResult>, label: string) => {
    setBusy(true);
    setResult(null);
    try {
      const r = await fn();
      setResult(r);
      onStatus(`${label}: ${r.frameCount} フレーム / ${r.boneCount} ボーンを ${r.outPath} に書き出しました`);
    } catch (e) {
      onError(`${label}失敗: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  const importBvh = async () => {
    const out = await pickSavePath("出力する .ycd.xml", "motion.ycd.xml", YCD_FILTERS);
    if (!out) return;
    await run(() => api.importBvhYcdXml(bvhPath.trim(), out), "BVH取込");
  };

  const genAi = async () => {
    const out = await pickSavePath("出力する .ycd.xml", "ai_motion.ycd.xml", YCD_FILTERS);
    if (!out) return;
    const args = runnerArgs.split(/\s+/).filter(Boolean);
    await run(() => api.generateAiMotionYcdXml(prompt.trim(), runnerBin.trim(), args, out), "AIモーション生成");
  };

  const browseBvh = async () => {
    const p = await pickFile("BVH ファイルを選択", [{ name: "BVH", extensions: ["bvh"] }]);
    if (p) setBvhPath(p);
  };

  return (
    <div className="bg-panel rounded-lg p-3 space-y-3">
      <h2 className="text-xs uppercase tracking-wide text-gray-500">
        モーション取込 (Phase 2/3)
      </h2>
      <p className="text-xs text-gray-500">
        .ycd.xml を出力します。実 .ycd 化・FiveM ロードは CodeWalker/Sollumz が別途必要な
        best-effort 出力です。
      </p>

      {/* BVH 取込 */}
      <div className="bg-surface rounded p-2 space-y-2">
        <div className="text-xs text-gray-400">BVH を取込んで GTA リターゲット</div>
        <div className="flex gap-2">
          <input
            className="flex-1 bg-panel px-2 py-1 rounded text-xs font-mono"
            aria-label="bvhPath"
            placeholder="…/motion.bvh"
            value={bvhPath}
            onChange={(e) => setBvhPath(e.target.value)}
          />
          <button className="px-2 py-1 rounded bg-panel text-sm" onClick={browseBvh}>
            参照
          </button>
        </div>
        <button
          className="w-full px-3 py-1.5 rounded bg-accent text-black text-sm font-medium disabled:opacity-50"
          onClick={importBvh}
          disabled={busy || !bvhPath.trim()}
        >
          BVH → .ycd.xml
        </button>
      </div>

      {/* AI モーション */}
      <div className="bg-surface rounded p-2 space-y-2">
        <div className="text-xs text-gray-400">外部 text-to-motion ランナーで生成</div>
        <input
          className="w-full bg-panel px-2 py-1 rounded text-xs"
          aria-label="motionPrompt"
          placeholder="例: a person waving hello"
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
        />
        <div className="flex gap-2">
          <input
            className="w-24 bg-panel px-2 py-1 rounded text-xs font-mono"
            aria-label="runnerBin"
            placeholder="python"
            value={runnerBin}
            onChange={(e) => setRunnerBin(e.target.value)}
          />
          <input
            className="flex-1 bg-panel px-2 py-1 rounded text-xs font-mono"
            aria-label="runnerArgs"
            placeholder="t2m_runner.py"
            value={runnerArgs}
            onChange={(e) => setRunnerArgs(e.target.value)}
          />
        </div>
        <button
          className="w-full px-3 py-1.5 rounded bg-accent text-black text-sm font-medium disabled:opacity-50"
          onClick={genAi}
          disabled={busy || !prompt.trim() || !runnerBin.trim()}
        >
          生成 → .ycd.xml
        </button>
      </div>

      {result && result.unmapped.length > 0 && (
        <div className="text-xs text-yellow-300/80">
          未対応ボーン ({result.unmapped.length}): {result.unmapped.slice(0, 8).join(", ")}
          {result.unmapped.length > 8 ? " …" : ""}
        </div>
      )}
    </div>
  );
}
