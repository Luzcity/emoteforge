import { useEffect, useState } from "react";
import * as api from "../lib/api";
import type { CodexAuthStatus } from "../lib/api";

interface Props {
  onError: (msg: string) => void;
  onStatus: (msg: string) => void;
}

/** codex の ChatGPT サブスクログイン状態の表示と、ログイン/ログアウト操作。 */
export default function CodexAuthPanel({ onError, onStatus }: Props) {
  const [auth, setAuth] = useState<CodexAuthStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);

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

  // マウント時に現在のログイン状態を取得しておく（ログイン処理は数分かかり得るため、
  // パネルは即座に現状を表示する）。
  useEffect(() => {
    void refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const login = async () => {
    setBusy(true);
    onStatus("ブラウザでログインを完了してください…");
    try {
      const next = await api.codexLogin();
      setAuth(next);
      onStatus(next.loggedIn ? "codex にログインしました" : "ログインが完了しませんでした");
    } catch (e) {
      onError(`ログイン失敗: ${e}`);
    } finally {
      setBusy(false);
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

      <div className="flex gap-2">
        <button
          className="flex-1 px-3 py-1.5 rounded bg-surface text-sm disabled:opacity-50"
          onClick={login}
          disabled={busy}
        >
          {busy ? "処理中…" : auth?.loggedIn ? "再ログイン" : "ChatGPT でログイン"}
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

      <p className="text-xs text-gray-500">
        ChatGPT のサブスク枠で codex を使えます。ブラウザが自動で開かない場合は、ターミナルで{" "}
        <code className="font-mono">codex login</code> を実行してください。
      </p>
    </div>
  );
}
