import { useState } from "react";

interface Props {
  onGenerate: (prompt: string) => void;
  generating: boolean;
}

export default function PromptBar({ onGenerate, generating }: Props) {
  const [text, setText] = useState("");

  const submit = () => {
    const p = text.trim();
    if (p && !generating) onGenerate(p);
  };

  return (
    <div className="flex gap-2 p-3 bg-panel rounded-lg">
      <input
        className="flex-1 bg-surface text-gray-100 px-3 py-2 rounded outline-none border border-transparent focus:border-accent"
        placeholder="例: 酔っ払って千鳥足で乾杯するエモート"
        value={text}
        aria-label="prompt"
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") submit();
        }}
        disabled={generating}
      />
      <button
        className="px-4 py-2 rounded bg-accent text-black font-medium disabled:opacity-50"
        onClick={submit}
        disabled={generating || !text.trim()}
      >
        {generating ? "生成中…" : "生成"}
      </button>
    </div>
  );
}
