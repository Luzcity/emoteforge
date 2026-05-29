# EmoteForge — プロジェクトガイド（AI/開発者共通）

FiveM 向けエモートスタジオ。Tauri v2 デスクトップアプリ。Rust コア + React/TS フロント。
プロンプトから codex でエモート生成 → 検証 → FiveM リソースとしてエクスポート、ローカル Bridge でプレビュー。

## 構成

- `core/` — ドメインロジック（catalog 検索/存在検証、validate、export、phase2/3 モーション、codex orchestrator）。ワークスペースメンバー。
- `src-tauri/` — Tauri シェル（コマンド/状態）。**ワークスペース除外**: webkit 依存で Linux 開発機ではビルド不可。
- `src/` — React フロント（vite + vitest）。
- `catalog/` — アニメカタログ生成。`catalog.json`（コミット）と `dump_index.json`（生成物・gitignore）。
- `schema/` — `emote.schema.json`（codex の `--output-schema` 用）。
- `fivem/` — Bridge リソース（Lua）。

## ビルド/テスト

- Rust core: `nix shell nixpkgs#cargo nixpkgs#gcc --command cargo test --workspace`（リンカに gcc 必須）。fmt は `cargo fmt --all --check`。
- src-tauri: ローカル不可。CI の `tauri-check`（ubuntu + apt webkit）が `clippy --locked -D warnings`、リリースは windows-latest でビルド。**`--locked` のため Cargo.lock を常に最新に保つ**（`cargo generate-lockfile --manifest-path src-tauri/Cargo.toml`）。
- フロント: `npm ci` → `npm test`（vitest）/ `npm run typecheck` / `npm run build` / `npm run format:check`。
- catalog 再生成: `node catalog/build_catalog.mjs`（dump は `DUMP_URL` env で指定、既定 master）。

## 重要な前提

- データ埋め込み: tracked な `catalog.json`/`emote.schema.json` は `include_bytes!` で直接、生成物 `dump_index.json` は `build.rs` が `OUT_DIR` 経由で埋め込む（未生成時 `{}`）。ポータブル exe 単体で起動する。
- リリースは Windows のみ。exe 起動に WebView2（Win11 標準）必要、AI 生成は外部 `codex` CLI 依存。
- Tauri コマンドはスレッドプールで実行されるため、共有状態・一時ファイルは並行実行を前提に。

## レビュー観点（コードレビュー依頼時はこれを体系的に適用）

確信度の高い指摘に絞り、**重大度(Critical/High/Medium/Low)を付け、コードを根拠に、日本語で**。該当なしの観点は省略。推測で埋めない。

1. **正確性**: 境界条件/off-by-one、`Result`/エラーの握り潰し、`unwrap` パニック、並行性（Tauri コマンド多重実行・共有 temp の衝突）、プラットフォーム差異（Windows リリース vs Linux/WSL 開発、パス・temp・改行）、`include_bytes!`/`build.rs` のパスと生成物前提。
2. **セキュリティ**: 入力検証（bridge URL の SSRF/外部送信、パストラバーサル、runner_bin/args）、シェル経由の有無（`Command::new` は無シェル）、CI の script injection（`${{ }}` を `run:` に直接展開しない・env 経由）、fork PR でのシークレット露出、`permissions:` 最小化。
3. **データ整合**: `catalog.json` ↔ `dump_index.json` の存在検証の一貫性、schema との整合、空 `{}` の扱い。
4. **設計/altitude**: 特例の積み増しより共通機構の一般化。tracked vs generated の埋め込み方針の一貫性。
5. **テスト**: 出荷経路（埋め込み bytes 経路）がテストで覆われているか。本番とテストで別経路を通っていないか。
