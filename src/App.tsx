import { useState, useRef } from "react";
import type { EmoteSpec } from "./types/emote";
import { emptyEmote } from "./types/emote";
import * as api from "./lib/api";
import type { ValidationIssue } from "./lib/api";
import PromptBar from "./components/PromptBar";
import EmoteEditor from "./components/EmoteEditor";
import EmoteLibrary from "./components/EmoteLibrary";
import ExportDialog from "./components/ExportDialog";
import SettingsPanel from "./components/SettingsPanel";
import MotionPanel from "./components/MotionPanel";

export default function App() {
  const [emotes, setEmotes] = useState<EmoteSpec[]>([]);
  const [activeIndex, setActiveIndex] = useState(-1);
  const [issues, setIssues] = useState<ValidationIssue[]>([]);
  const [generating, setGenerating] = useState(false);
  const [previewing, setPreviewing] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  // 編集中の検証は非同期。古い結果が新しい状態を上書きしないよう連番でガードする。
  const validateSeq = useRef(0);

  const active = activeIndex >= 0 ? emotes[activeIndex] : null;

  // 子パネル用: 一方を出すとき他方は消す（error と status は同時に出さない既存方針に合わせる）。
  const notifyError = (msg: string) => {
    setStatus(null);
    setError(msg);
  };
  const notifyStatus = (msg: string) => {
    setError(null);
    setStatus(msg);
  };

  // emote を切り替えたら検証結果は対象が変わるので破棄する（前 emote の issues 残留を防ぐ）。
  const selectEmote = (i: number) => {
    setActiveIndex(i);
    setIssues([]);
  };

  const addEmote = (spec: EmoteSpec) => {
    setEmotes((prev) => {
      const next = [...prev, spec];
      setActiveIndex(next.length - 1);
      return next;
    });
    // 新規/生成 emote は別物なので前の検証結果を破棄する。handleGenerate は直後に
    // 本物の issues を setIssues するため（同一 tick でバッチされ後勝ち）上書きされる。
    setIssues([]);
  };

  const updateActive = (spec: EmoteSpec) => {
    if (activeIndex < 0) return;
    setEmotes((prev) => prev.map((e, i) => (i === activeIndex ? spec : e)));
  };

  const handleGenerate = async (prompt: string) => {
    setError(null);
    setStatus(null);
    setGenerating(true);
    try {
      const result = await api.generateEmote(prompt);
      addEmote(result.spec);
      setIssues(result.issues);
      setStatus(`「${result.spec.displayName}」を生成しました`);
    } catch (e) {
      setError(String(e));
    } finally {
      setGenerating(false);
    }
  };

  const handleEditorChange = async (spec: EmoteSpec) => {
    updateActive(spec);
    const seq = ++validateSeq.current;
    try {
      const result = await api.validateEmote(spec);
      // この検証より後に編集が走っていたら結果を捨てる（順不同到着での巻き戻し防止）。
      if (seq !== validateSeq.current) return;
      setIssues(result.issues);
      // 正規化済み spec はあえて書き戻さない（name サニタイズだけでなく playbackRate/blend の
      // クランプ等すべての正規化を含む）。毎キーストロークで書き戻すと識別子欄にスペースや `_`
      // を打てず（"wave hello" が "wavehello" に潰れる）、数値も入力途中で飛ぶため。
      // 正規化は preview/export の検証ゲートで確実に適用される。
    } catch {
      /* 検証呼び出しの失敗は致命的でないため無視（編集は反映済み）。
         壊れた spec は preview/export 時の検証ゲートで確実に止まる。 */
    }
  };

  const handlePreview = async () => {
    if (!active) return;
    setError(null);
    setPreviewing(true);
    try {
      await api.previewEmote(active);
      setStatus("実ゲームに送信しました");
    } catch (e) {
      setError(`プレビュー失敗: ${e}（FiveM と emoteforge_bridge を確認してください）`);
    } finally {
      setPreviewing(false);
    }
  };

  const handleStop = async () => {
    try {
      await api.stopPreview();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleExport = async (outDir: string, resourceName: string) => {
    setError(null);
    setExporting(true);
    try {
      const manifest = await api.exportEmotes(emotes, outDir, resourceName);
      setStatus(`${manifest.emoteCount} 件を ${manifest.dir} に書き出しました`);
    } catch (e) {
      setError(`エクスポート失敗: ${e}`);
    } finally {
      setExporting(false);
    }
  };

  const handleNewEmote = () => addEmote(emptyEmote());

  const handleDelete = (i: number) => {
    setEmotes((prev) => prev.filter((_, idx) => idx !== i));
    setActiveIndex((cur) => (i === cur ? -1 : cur > i ? cur - 1 : cur));
  };

  return (
    <div className="min-h-screen bg-surface text-gray-100 flex flex-col">
      <header className="px-4 py-3 border-b border-panel flex items-center justify-between">
        <h1 className="text-lg font-semibold">EmoteForge</h1>
        <span className="text-xs text-gray-500">FiveM エモートスタジオ</span>
      </header>

      <div className="px-4 pt-3">
        <PromptBar onGenerate={handleGenerate} generating={generating} />
      </div>

      {error && (
        <div
          className="mx-4 mt-2 bg-red-950/60 border border-red-700 text-red-200 rounded px-3 py-2 text-sm"
          role="alert"
        >
          {error}
        </div>
      )}
      {status && !error && (
        <div className="mx-4 mt-2 text-sm text-gray-400" role="status">
          {status}
        </div>
      )}

      <main className="flex-1 grid grid-cols-[260px_1fr_300px] gap-4 p-4 min-h-0">
        <aside className="bg-panel rounded-lg p-3 overflow-auto">
          <button
            className="w-full mb-3 px-3 py-1.5 rounded bg-surface text-sm"
            onClick={handleNewEmote}
          >
            ＋ 空のエモート
          </button>
          <EmoteLibrary
            emotes={emotes}
            activeIndex={activeIndex}
            onSelect={selectEmote}
            onDelete={handleDelete}
          />
        </aside>

        <section className="bg-panel rounded-lg p-4 overflow-auto">
          {active ? (
            <EmoteEditor
              spec={active}
              issues={issues}
              onChange={handleEditorChange}
              onPreview={handlePreview}
              onStop={handleStop}
              previewing={previewing}
            />
          ) : (
            <div className="h-full flex items-center justify-center text-gray-500">
              プロンプトから生成するか、空のエモートを作成してください
            </div>
          )}
        </section>

        <aside className="overflow-auto space-y-4">
          <ExportDialog emoteCount={emotes.length} onExport={handleExport} exporting={exporting} />
          <SettingsPanel onError={notifyError} onStatus={notifyStatus} />
          <MotionPanel onError={notifyError} onStatus={notifyStatus} />
        </aside>
      </main>
    </div>
  );
}
