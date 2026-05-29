# FiveM エモート開発ソフト 実装プラン

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** プロンプトから FiveM 用エモートを生成・編集・実ゲームプレビュー・スタンドアロンリソースとしてエクスポートできる Win11 デスクトップソフトを、3 フェーズ（Codex振付 / .ycdパイプライン / AI生成）で実装する。

**Architecture:** Tauri v2（Rust コア）+ React/TS/Tailwind（UI）。Rust コアが codex CLI を `codex exec --output-schema` で叩き構造化 `EmoteSpec` を得る → バリデート → UI 編集 → Preview Bridge（FiveM コンパニオン resource, localhost HTTP）で実ゲーム再生 / Exporter でスタンドアロン resource 生成。Phase 2 以降は `.ycd` パイプライン（Blender+Sollumz+CodeWalker）と AI motion を同じ resource 殻に載せる。

**Tech Stack:** Rust (Tauri v2, serde, jsonschema, axum/reqwest), TypeScript, React, Vite, Tailwind CSS, Vitest, codex CLI, FiveM (Lua), Blender/Sollumz/CodeWalker (Phase 2), text-to-motion model (Phase 3)。

---

## ファイル構成

```
fivem_emote/
├─ src-tauri/                      # Rust コア
│  ├─ Cargo.toml
│  ├─ tauri.conf.json
│  ├─ build.rs
│  └─ src/
│     ├─ main.rs                   # Tauri エントリ + command 登録
│     ├─ model/emote.rs            # EmoteSpec データモデル(serde)
│     ├─ catalog/mod.rs            # カタログ読込・検索
│     ├─ catalog/types.rs          # CatalogEntry 型
│     ├─ codex/orchestrator.rs     # codex exec 呼び出し抽象 + 実装
│     ├─ codex/prompt.rs           # プロンプト/スキーマ組み立て
│     ├─ validate/validator.rs     # dict/clip 実在検証・補修
│     ├─ export/exporter.rs        # EmoteSpec → FiveM resource
│     ├─ export/lua_templates.rs   # fxmanifest/client.lua テンプレ
│     ├─ preview/bridge_client.rs  # Preview Bridge への HTTP
│     └─ commands.rs               # UI から呼ぶ Tauri command
├─ src/                            # React UI
│  ├─ main.tsx, App.tsx
│  ├─ components/ (PromptBar, EmoteEditor, ClipList, FlagControls, EmoteLibrary, ExportDialog)
│  ├─ lib/api.ts                   # Tauri invoke ラッパ
│  └─ types/emote.ts               # EmoteSpec(TS) — Rust と一致
├─ schema/emote.schema.json        # codex --output-schema 用
├─ catalog/
│  ├─ build_catalog.mjs            # dump→curated catalog ビルド
│  └─ catalog.json                 # 生成物（同梱）
├─ fivem/emoteforge_bridge/        # Preview Bridge resource
│  ├─ fxmanifest.lua, server.lua, client.lua
├─ phase2/                         # .ycd パイプライン
│  ├─ import/ (fbx.rs, bvh.rs)
│  ├─ retarget/skeleton_map.rs
│  ├─ ycd/xml_builder.rs           # CodeWalker XML 生成
│  └─ blender/sollumz_export.py    # Blender ヘッドレス変換
└─ phase3/
   └─ motion/ (client.rs, hf_space.rs)  # text-to-motion 連携
```

---

# Phase 1: Codex 振付エンジン（.ycd なし）

### Task 1: プロジェクト初期化（Tauri + React + TS + Tailwind）

**Files:** `package.json`, `src-tauri/Cargo.toml`, `tauri.conf.json`, `vite.config.ts`, `tailwind.config.js`, `src/main.tsx`

- [ ] `npm create tauri-app` 相当の最小構成を手で作成（Vite+React+TS）
- [ ] Tailwind 導入（`tailwind.config.js`, `index.css` に directives）
- [ ] `cargo build`（src-tauri）と `npm run build` が通ることを確認
- [ ] Commit: `chore: scaffold Tauri+React+TS+Tailwind`

### Task 2: EmoteSpec データモデル（Rust + TS + JSON Schema）

**Files:** `src-tauri/src/model/emote.rs`, `src/types/emote.ts`, `schema/emote.schema.json`

- [ ] 設計書 §6 の `EmoteSpec`/`ClipRef`/`Prop`/`Facial` を Rust struct(serde) で定義
- [ ] 同型を TS に定義（フィールド名一致）
- [ ] `emote.schema.json` を定義（codex `--output-schema` 用、enum/必須を明示）
- [ ] Test: serde round-trip（JSON→struct→JSON 同値）、schema が代表 JSON を valid とする
- [ ] Commit: `feat: add EmoteSpec model and JSON schema`

### Task 3: カタログビルドスクリプト + 型

**Files:** `catalog/build_catalog.mjs`, `catalog/catalog.json`, `src-tauri/src/catalog/types.rs`, `src-tauri/src/catalog/mod.rs`

- [ ] `build_catalog.mjs`: DurtyFree `animDictsCompact.json`（ローカル取得済みを入力）から emote 実用域を抽出し、`{key, displayName, dict, clip, tags[], defaults{}}` 形式の `catalog.json` を生成。初期キュレーションは `amb@`, `anim@`, `mp_player_int*`, `rcm*`, `special_ped@`, dance 系等の代表 dict をタグ付け。
- [ ] `catalog/mod.rs`: `Catalog::load()`, `search(query) -> Vec<CatalogEntry>`, `contains(dict, clip) -> bool`
- [ ] Test: load が件数>0、`contains` が既知 dict/clip に true、未知に false、`search("dance")` がヒット
- [ ] Commit: `feat: catalog builder and loader`

### Task 4: Codex オーケストレータ（抽象 + 実装 + モック）

**Files:** `src-tauri/src/codex/orchestrator.rs`, `src-tauri/src/codex/prompt.rs`

- [ ] `trait CodexRunner { fn run(&self, prompt:&str, schema_path:&Path) -> Result<String> }`
- [ ] 実装 `CliCodexRunner`: `codex exec --output-schema <schema> -o <tmp> "<prompt>"` を実行し最終メッセージ JSON を読む。`--skip-git-repo-check`, タイムアウト, 非0終了ハンドリング。
- [ ] `prompt.rs`: ユーザープロンプト + カタログ要約（上位候補の key/dict/clip/tags）+ 出力指示を組み立て
- [ ] `Orchestrator::generate(prompt) -> EmoteSpec`（runner をモックしたテスト）
- [ ] Test: モック runner が固定 JSON を返し EmoteSpec にパースされる、不正 JSON でエラー
- [ ] Commit: `feat: codex orchestrator with mockable runner`

### Task 5: バリデータ

**Files:** `src-tauri/src/validate/validator.rs`

- [ ] `validate(spec, &catalog) -> Result<EmoteSpec, ValidationReport>`：各 clip の dict/clip 実在チェック、不正は `suggestions`（カタログ近傍）を付与
- [ ] フラグ/enum の妥当性、空 clips 拒否
- [ ] Test: 実在 spec は Ok、偽 dict は Err+suggestion、空 clips は Err
- [ ] Commit: `feat: emote spec validator`

### Task 6: Exporter（スタンドアロン resource）

**Files:** `src-tauri/src/export/exporter.rs`, `src-tauri/src/export/lua_templates.rs`

- [ ] `export(specs:&[EmoteSpec], out_dir) -> ResourceManifest`：`fxmanifest.lua` + `client.lua`（`RequestAnimDict`→`TaskPlayAnim`、複数 clip は Lua チェーン、prop/facial/flags 反映、`/emote <name>` と `/emotestop`）+ `emotes.json`
- [ ] Test: 出力ファイルが存在、Lua が各 emote 名を含む、emotes.json が spec と一致、Lua 構文の簡易検査（バランス/必須ネイティブ）
- [ ] Commit: `feat: standalone FiveM resource exporter`

### Task 7: Preview Bridge resource + クライアント

**Files:** `fivem/emoteforge_bridge/{fxmanifest.lua,server.lua,client.lua}`, `src-tauri/src/preview/bridge_client.rs`

- [ ] bridge resource: `server.lua` が `SetHttpHandler` で `POST /preview`（EmoteSpec JSON）受信→ client へ event → 自キャラ再生。`POST /stop`。
- [ ] `bridge_client.rs`: `preview(spec, base_url)` が JSON を POST。接続不可は明示エラー。
- [ ] Test: bridge_client が mock HTTP（wiremock）へ正しい body を POST、接続不可でエラー
- [ ] Commit: `feat: in-game preview bridge`

### Task 8: Tauri commands（IPC）

**Files:** `src-tauri/src/commands.rs`, `src-tauri/src/main.rs`

- [ ] commands: `generate_emote(prompt)->EmoteSpec`, `validate_emote(spec)`, `search_catalog(q)`, `preview_emote(spec)`, `export_emotes(specs, dir)`
- [ ] main.rs に登録、状態（Catalog, Orchestrator）を `manage`
- [ ] Test: command 関数の単体テスト（内部関数を直接）
- [ ] Commit: `feat: wire Tauri commands`

### Task 9: React UI

**Files:** `src/App.tsx`, `src/components/*`, `src/lib/api.ts`, `src/types/emote.ts`

- [ ] `api.ts`: `invoke` ラッパ（型付き）
- [ ] PromptBar（入力→generate）、EmoteEditor（ClipList 並べ替え/追加/削除、FlagControls、prop/facial）、EmoteLibrary（複数管理）、ExportDialog（出力先選択）、Preview ボタン
- [ ] Tailwind で Win11 風の落ち着いた UI
- [ ] Test: Vitest + Testing Library でコンポーネントの主要操作
- [ ] Commit: `feat: emote studio UI`

### Task 10: Phase 1 結合 & 手動 E2E
- [ ] `tauri dev` 起動確認、codex 経由生成→検証→（bridge があれば）プレビュー→エクスポートの一連を手動確認
- [ ] README にセットアップ/使い方
- [ ] Commit: `docs: phase1 usage`

---

# Phase 2: .ycd パイプライン（外部モーション取込→リターゲット→.ycd）

### Task 11: モーション取込パーサ（BVH/FBX）
- [ ] `phase2/import/bvh.rs`：BVH を内部 `MotionClip`（joint 階層 + フレーム）へ
- [ ] FBX は `fbx2gltf`/外部依存を抽象化（trait）、まず BVH で骨組み
- [ ] Test: 既知 BVH をパースしフレーム数/関節数一致
- [ ] Commit: `feat: motion import (BVH)`

### Task 12: GTA スケルトンリターゲット
- [ ] `phase2/retarget/skeleton_map.rs`：標準ヒューマノイド↔GTA V ボーン名/ID マップ、回転リターゲット
- [ ] Test: 既知関節マッピングが正しい、恒等リターゲットが入力を保つ
- [ ] Commit: `feat: GTA skeleton retarget`

### Task 13: CodeWalker XML(.ycd.xml) ビルダー
- [ ] `phase2/ycd/xml_builder.rs`：`MotionClip` → CodeWalker 互換 `*.ycd.xml`（ClipDictionary/Animation/Sequences/Channels）
- [ ] Test: 生成 XML がスキーマ的に妥当（要素/属性）、既知入力でスナップショット
- [ ] Commit: `feat: ycd XML builder`

### Task 14: Blender(Sollumz) ヘッドレス変換 + CodeWalker 変換
- [ ] `phase2/blender/sollumz_export.py`：Blender `--background` で Sollumz により xml/anim を取り込み `.ycd` 書き出し（手順自動化スクリプト）
- [ ] CodeWalker CLI / RPF 不要のため `stream/` 配置を Exporter に追加（`.ycd` を resource にストリーム）
- [ ] ドキュメント: 必要な外部ツール（Blender+Sollumz, CodeWalker）と実行手順。環境に Blender が無い場合のガイド。
- [ ] Commit: `feat: blender/sollumz ycd export pipeline`

### Task 15: Phase 2 を UI/Exporter に統合
- [x] `ClipRef.source`（`builtin` | `ycd`）拡張済み（モデルに実装）
- [x] Tauri コマンド `import_bvh_ycd_xml` / `generate_ai_motion_ycd_xml` を結線
- [ ] （未実装・将来）アプリ内 3D プレビュー（Three.js）で `MotionClip` を再生
      → `.ycd` の実バイナリ/実スケルトンが無いと正確な検証ができないため後回し。
        現状は実ゲームプレビュー(Phase1 Bridge)＋外部ツール(CodeWalker)確認で代替。

---

# Phase 3: AI text-to-motion 生成

### Task 16: モーション生成抽象 + HF Space クライアント
- [ ] `phase3/motion/client.rs`：`trait MotionGenerator { fn generate(prompt)->MotionClip }`
- [ ] `hf_space.rs`：HuggingFace Space/Inference 経由実装（MoMask 等）。ローカル GPU 実装はオプション（feature flag）。
- [ ] Test: モック generator が固定 MotionClip を返す
- [ ] Commit: `feat: text-to-motion generator abstraction + HF client`

### Task 17: 生成→リターゲット→.ycd→export の結線
- [ ] プロンプト → MotionGenerator → MotionClip → retarget → ycd → resource、を 1 フローに
- [ ] UI に「AI生成」モードを追加（Codex振付 / AI motion を切替）
- [ ] Test: モック generator でフロー結合（ycd まで）
- [ ] Commit: `feat: end-to-end AI motion pipeline`

---

# 完了後フェーズ: 監査

### Task 18: パフォーマンス監査
- [ ] カタログ読込/検索のベンチ（大規模 dump）、codex 呼び出しのタイムアウト/キャンセル、UI 応答性
- [ ] ボトルネック修正（必要なら catalog の索引化/遅延読込）

### Task 19: セキュリティ監査（security-review）
- [ ] codex/外部プロセス起動の引数注入対策、HTTP bridge の localhost 限定・CORS、ファイル書込パストラバーサル、シークレット非ハードコード確認
- [ ] CRITICAL を修正

---

## Self-Review メモ
- スコープ整合: 設計書 §3〜§12 の各項目に対応タスクあり（Phase1 §5 各コンポーネント=Task2-9, プレビュー=Task7, エクスポート=Task6, 非スコープは Phase2/3 へ）。
- 型整合: `EmoteSpec`/`ClipRef`/`CatalogEntry`/`MotionClip`/`ValidationReport`/`ResourceManifest` を全タスクで一貫使用。
- 外部依存（Blender/CodeWalker/GPU）が無い環境では当該バイナリ変換は手順ドキュメント + モックテストで担保し、コードは完成させる。
