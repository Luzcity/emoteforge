import { useState } from "react";
import * as api from "../lib/api";
import { pickDirectory } from "../lib/dialog";

const DEFAULT_BRIDGE_URL = "http://localhost:30120/emoteforge_bridge";

interface Props {
  onError: (msg: string) => void;
  onStatus: (msg: string) => void;
}

/** プレビューブリッジ接続先・ブリッジ導入・codex モデルの設定。 */
export default function SettingsPanel({ onError, onStatus }: Props) {
  const [bridgeUrl, setBridgeUrl] = useState(DEFAULT_BRIDGE_URL);
  const [model, setModel] = useState("");
  const [installing, setInstalling] = useState(false);

  const applyBridgeUrl = async () => {
    const url = bridgeUrl.trim();
    if (!url) return;
    try {
      await api.setBridgeUrl(url);
      onStatus(`ブリッジ接続先を ${url} に設定しました`);
    } catch (e) {
      onError(String(e));
    }
  };

  const applyModel = async () => {
    try {
      await api.setCodexModel(model.trim() || null);
      onStatus(
        model.trim()
          ? `codex モデルを ${model.trim()} に設定しました`
          : "codex モデルを既定に戻しました"
      );
    } catch (e) {
      onError(String(e));
    }
  };

  const installBridge = async () => {
    const dir = await pickDirectory("FiveM の resources フォルダを選択");
    if (!dir) return;
    setInstalling(true);
    try {
      const path = await api.installBridgeResource(dir);
      onStatus(`ブリッジを ${path} に書き出しました（ensure emoteforge_bridge で有効化）`);
    } catch (e) {
      onError(`ブリッジ導入失敗: ${e}`);
    } finally {
      setInstalling(false);
    }
  };

  return (
    <div className="bg-panel rounded-lg p-3 space-y-3">
      <h2 className="text-xs uppercase tracking-wide text-gray-500">設定</h2>

      <div className="space-y-1">
        <label className="block text-sm">
          プレビュー接続先 (Bridge URL)
          <div className="flex gap-2 mt-1">
            <input
              className="flex-1 bg-surface px-2 py-1 rounded text-xs font-mono"
              aria-label="bridgeUrl"
              value={bridgeUrl}
              onChange={(e) => setBridgeUrl(e.target.value)}
            />
            <button className="px-3 py-1 rounded bg-surface text-sm" onClick={applyBridgeUrl}>
              適用
            </button>
          </div>
        </label>
        <button
          className="w-full px-3 py-1.5 rounded bg-surface text-sm disabled:opacity-50"
          onClick={installBridge}
          disabled={installing}
        >
          {installing ? "導入中…" : "ブリッジを resources に導入"}
        </button>
      </div>

      <label className="block text-sm">
        codex モデル（空で既定）
        <div className="flex gap-2 mt-1">
          <input
            className="flex-1 bg-surface px-2 py-1 rounded text-xs font-mono"
            aria-label="codexModel"
            placeholder="例: gpt-5.4"
            value={model}
            onChange={(e) => setModel(e.target.value)}
          />
          <button className="px-3 py-1 rounded bg-surface text-sm" onClick={applyModel}>
            適用
          </button>
        </div>
      </label>
    </div>
  );
}
