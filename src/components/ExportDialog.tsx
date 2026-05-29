import { useState } from "react";
import { pickDirectory } from "../lib/dialog";

interface Props {
  emoteCount: number;
  onExport: (outDir: string, resourceName: string) => void;
  exporting: boolean;
}

export default function ExportDialog({ emoteCount, onExport, exporting }: Props) {
  const [outDir, setOutDir] = useState("");
  const [resourceName, setResourceName] = useState("my_emotes");

  const browse = async () => {
    const dir = await pickDirectory("エクスポート先フォルダを選択");
    if (dir) setOutDir(dir);
  };

  const valid = outDir.trim() !== "" && /^[A-Za-z0-9_-]+$/.test(resourceName) && emoteCount > 0;

  return (
    <div className="bg-panel rounded-lg p-3 space-y-2">
      <h2 className="text-xs uppercase tracking-wide text-gray-500">エクスポート</h2>
      <label className="block text-sm">
        リソース名
        <input
          className="w-full bg-surface px-2 py-1 rounded mt-1 font-mono"
          aria-label="resourceName"
          value={resourceName}
          onChange={(e) => setResourceName(e.target.value)}
        />
      </label>
      <div className="flex gap-2 items-end">
        <label className="flex-1 text-sm">
          出力先
          <input
            className="w-full bg-surface px-2 py-1 rounded mt-1 text-xs"
            aria-label="outDir"
            value={outDir}
            onChange={(e) => setOutDir(e.target.value)}
            placeholder="resources フォルダ等"
          />
        </label>
        <button className="px-3 py-1 rounded bg-surface" onClick={browse}>
          参照
        </button>
      </div>
      <button
        className="w-full px-4 py-2 rounded bg-accent text-black font-medium disabled:opacity-50"
        onClick={() => onExport(outDir.trim(), resourceName)}
        disabled={!valid || exporting}
      >
        {exporting ? "書き出し中…" : `スタンドアロンリソースを書き出す (${emoteCount})`}
      </button>
    </div>
  );
}
