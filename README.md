# EmoteForge — FiveM エモートスタジオ

プロンプトを打つだけで FiveM 用エモートを生成・編集・実ゲームプレビュー・スタンドアロン
リソースとしてエクスポートできる Win11 デスクトップソフト。Codex（ChatGPT サブスク）を
「振付師」として使い、既存 GTA V アニメクリップを部品に新規エモートを組み立てる。

- 設計書: `docs/superpowers/specs/2026-05-29-fivem-emote-tool-design.md`
- 実装プラン: `docs/superpowers/plans/2026-05-29-fivem-emote-tool.md`

## フェーズ

| Phase | 内容 | 状態 |
|---|---|---|
| 1 | Codex 振付（既存クリップを Lua レベルで合成、`.ycd` 不要） | **実装済み・検証済み**（実 codex E2E / cargo / luac / Vitest） |
| 2 | 外部モーション取込 → GTA リターゲット → `.ycd` 生成 | 未実装（これから） |
| 3 | AI text-to-motion 生成 → `.ycd` パイプラインへ | 未実装（これから） |

> **検証境界について**: Phase 1 は実 codex 連携・生成 Lua の構文・src-tauri コンパイル・UI まで
> 実機検証済み。Phase 2/3 は `.ycd` バイナリ生成に Blender(Sollumz)/CodeWalker(.NET, Windows)/GPU を
> 要し、本リポジトリの Linux 開発環境では**最終段（実 `.ycd` 書き出し・FiveM 実ロード）を検証できない**。
> 該当箇所は「実装済みだが未検証（外部ツール必須）」として明記する。

## アーキテクチャ

- `core/` … Tauri 非依存の純 Rust ライブラリ `emoteforge_core`（モデル/カタログ/Codex/検証/エクスポート/プレビュー、Phase2/3 パイプライン）。`cargo test` で全ロジックを検証可能。
- `src-tauri/` … Tauri v2 デスクトップシェル（コマンド層）。Win11 でビルド。
- `src/` … React + TypeScript + Tailwind CSS の UI。
- `catalog/` … アニメカタログのビルダーと生成データ。
- `fivem/emoteforge_bridge/` … 実ゲームプレビュー用コンパニオン FiveM リソース。

## 必要環境（Win11）

- [Codex CLI](https://github.com/openai/codex)（`codex login` で ChatGPT ログイン済みであること）
- Node.js 20+ / Rust 1.94+ / Tauri v2 prerequisites（WebView2 ランタイム）
- 実ゲームプレビューを使う場合: FiveM サーバー（`/opt/fivem` 等）

## セットアップ

```bash
# 1) アニメカタログを生成（DurtyFree dump を取得して索引化）
curl -sL "https://raw.githubusercontent.com/DurtyFree/gta-v-data-dumps/master/animDictsCompact.json" \
  -o catalog/animDictsCompact.json
node catalog/build_catalog.mjs        # → catalog/catalog.json, catalog/dump_index.json

# 2) 依存インストール
npm install

# 3) 開発起動 / ビルド
npm run tauri dev                     # 開発
npm run tauri build                   # Win11 インストーラ(NSIS/MSI)を生成
```

## 使い方

1. プロンプト欄にやりたいエモートを日本語/英語で入力 → 「生成」。
2. Codex が既存クリップを選び新規エモート仕様を返す。クリップ列・ループ/上半身/移動・速度を編集可能。
3. 「実ゲームでプレビュー」… 下記 Bridge を有効化した FiveM で自キャラに即再生。
4. 「スタンドアロンリソースを書き出す」… 依存なしの FiveM リソースを生成。サーバーの
   `resources/` に置き `ensure <resource_name>` で有効化。ゲーム内で `/emote <name>` / `/emotestop`。

### 実ゲームプレビュー (Preview Bridge)

`fivem/emoteforge_bridge/` を FiveM の `resources/` に置き `ensure emoteforge_bridge`。
アプリは `http://localhost:30120/emoteforge_bridge/preview` に POST する（接続先は変更可）。

## テスト / 検証

```bash
cargo test -p emoteforge_core                 # コアロジック（モデル/カタログ/Codex/検証/エクスポート/プレビュー/Phase2-3）
cargo test -p emoteforge_core --test codex_live -- --ignored  # 実 codex 連携（要ログイン・課金なし）
npm run test                                  # UI（Vitest）
cargo check                                   # src-tauri（Tauri v2）コンパイル
luac -p <(cargo run -q -p emoteforge_core --example dump_client_lua)  # 生成 Lua 構文
```

## Phase 2/3 の外部ツール

`.ycd` バイナリ生成は OSS を再利用する（再発明しない）:
- [Sollumz](https://github.com/Sollumz/Sollumz)（Blender アドオン）… YCD の import/edit/export（CodeWalker XML 経由）
- [CodeWalker](https://github.com/dexyfex/CodeWalker) … XML ↔ バイナリ変換・フォーマット参照

詳細は実装プラン `docs/superpowers/plans/2026-05-29-fivem-emote-tool.md` を参照（Phase 2 実装時に `phase2/README.md` を追加予定）。
