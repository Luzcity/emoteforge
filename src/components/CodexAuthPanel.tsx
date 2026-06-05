import { useEffect, useState } from "react";
import * as api from "../lib/api";
import type { CodexAuthStatus, CodexLoginMethod, CodexLoginPrompt } from "../lib/api";

interface Props {
  onError: (msg: string) => void;
  onStatus: (msg: string) => void;
}

const METHODS: { value: CodexLoginMethod; label: string; hint: string }[] = [
  { value: "browser", label: "ブラウザ", hint: "ChatGPT サブスク枠（ブラウザで OAuth）" },
  { value: "device", label: "デバイスコード", hint: "ブラウザが開かない時。コードを入力して認証" },
  { value: "apiKey", label: "API キー", hint: "OpenAI API キーを貼り付け" },
];

/** codex のログイン状態の表示と、方式を選んでのログイン/ログアウト操作。 */
export default function CodexAuthPanel({ onError, onStatus }: Props) {
  const [auth, setAuth] = useState<CodexAuthStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [method, setMethod] = useState<CodexLoginMethod>("browser");
  const [apiKey, setApiKey] = useState("");
  const [prompt, setPrompt] = useState<CodexLoginPrompt | null>(null);

  const refresh = async () => {
    try {
      setAuth(await api.codexLoginStatus());
    } catch (e) {
      // codex 未インストール等。致命的でないので未ログイン表示に倒す。
      setAuth(null);
      onError(`codex の状態取得に失敗: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  // マウント時に現在のログイン状態を取得し、進行中の URL/コードイベントを購読する
  // （ブラウザが自動で開かない場合の手動フォールバック表示のため）。
  useEffect(() => {
    void refresh();
    // 購読登録は非同期。Promise 解決前にアンマウントされても確実に解除できるよう、
    // cancelled フラグで判定する（解決が後着なら即座に unlisten を呼ぶ）。
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    void api
      .onCodexLoginPrompt((p) => setPrompt(p))
      .then((u) => {
        if (cancelled) {
          u();
        } else {
          unlisten = u;
        }
      });
    return () => {
      cancelled = true;
      unlisten?.();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const login = async () => {
    if (method === "apiKey" && !apiKey.trim()) {
      onError("API キーを入力してください");
      return;
    }
    setBusy(true);
    setPrompt(null);
    onStatus(method === "apiKey" ? "API キーで認証中…" : "ブラウザでログインを完了してください…");
    try {
      const next = await api.codexLogin(method, method === "apiKey" ? apiKey : undefined);
      setAuth(next);
      setApiKey("");
      onStatus(next.loggedIn ? "codex にログインしました" : "ログインが完了しませんでした");
    } catch (e) {
      onError(`ログイン失敗: ${e}`);
    } finally {
      setBusy(false);
      setPrompt(null);
    }
  };

  const logout = async () => {
    setBusy(true);
    try {
      await api.codexLogout();
      await refresh();
      onStatus("codex からログアウトしました");
    } catch (e) {
      onError(`ログアウト失敗: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="bg-panel rounded-lg p-3 space-y-2">
      <h2 className="text-xs uppercase tracking-wide text-gray-500">codex アカウント</h2>

      <div className="text-sm">
        {loading ? (
          <span className="text-gray-500">状態を確認中…</span>
        ) : auth?.loggedIn ? (
          <span className="text-green-400">
            ログイン済み{auth.method ? `（${auth.method}）` : ""}
          </span>
        ) : (
          <span className="text-gray-400">未ログイン</span>
        )}
      </div>

      {/* 方式セレクタ */}
      <div className="flex gap-1" role="radiogroup" aria-label="ログイン方式">
        {METHODS.map((m) => (
          <button
            key={m.value}
            role="radio"
            aria-checked={method === m.value}
            title={m.hint}
            className={`flex-1 px-2 py-1 rounded text-xs ${
              method === m.value ? "bg-accent text-white" : "bg-surface text-gray-300"
            } disabled:opacity-50`}
            onClick={() => setMethod(m.value)}
            disabled={busy}
          >
            {m.label}
          </button>
        ))}
      </div>
      <p className="text-xs text-gray-500">{METHODS.find((m) => m.value === method)?.hint}</p>

      {method === "apiKey" && (
        <input
          type="password"
          aria-label="apiKey"
          placeholder="sk-..."
          className="w-full px-2 py-1 rounded bg-surface text-sm font-mono"
          value={apiKey}
          onChange={(e) => setApiKey(e.target.value)}
          disabled={busy}
        />
      )}

      <div className="flex gap-2">
        <button
          className="flex-1 px-3 py-1.5 rounded bg-surface text-sm disabled:opacity-50"
          onClick={login}
          disabled={busy}
        >
          {busy ? "処理中…" : auth?.loggedIn ? "再ログイン" : "ログイン"}
        </button>
        {auth?.loggedIn && (
          <button
            className="px-3 py-1.5 rounded bg-surface text-sm disabled:opacity-50"
            onClick={logout}
            disabled={busy}
          >
            ログアウト
          </button>
        )}
      </div>

      {/* 進行中の手動フォールバック：ブラウザが自動で開かない場合に URL/コードを提示。 */}
      {busy && prompt && (prompt.url || prompt.code) && (
        <div className="rounded bg-surface p-2 space-y-1 text-xs">
          {prompt.code && (
            <div>
              <span className="text-gray-500">ワンタイムコード：</span>
              <code className="ml-1 select-all font-mono text-base text-accent">{prompt.code}</code>
            </div>
          )}
          {prompt.url && (
            <div className="break-all">
              <span className="text-gray-500">
                ブラウザが開かない場合はこの URL を開いてください：
              </span>
              <br />
              <code className="select-all font-mono text-gray-300">{prompt.url}</code>
            </div>
          )}
        </div>
      )}

      {method !== "apiKey" && (
        <p className="text-xs text-gray-500">
          ブラウザが自動で開かない場合は「デバイスコード」を選ぶか、上に表示される URL
          を手動で開いてください。
        </p>
      )}
    </div>
  );
}
