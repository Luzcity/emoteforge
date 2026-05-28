# FiveM エモート開発ソフトウェア — 設計書

- 日付: 2026-05-29
- ステータス: Phase 1 設計確定（Phase 2/3 はビジョンとして方向性のみ記載）
- 対象環境: Windows 11（開発は Linux、最終ビルドは Win11 / クロスビルド）

## 1. 目的とビジョン

プロンプトを打つだけで FiveM で使える新規エモートを生成・編集・エクスポートできる
Win11 デスクトップソフトを作る。既存 OSS（emote メニュー類）や更新停止した公式系ツールの
弱点を改善し、Codex（ChatGPT サブスク）を「振付師」として活用する。

最終ビジョンは「AI による完全新規モーション生成」だが、技術リスクが `.ycd`（新規クリップ辞書の
バイナリ生成）に集中するため、その境界で 3 フェーズに分解する。

## 2. フェーズ分解（`.ycd` 境界）

| | Phase 1（本書の対象） | Phase 2 | Phase 3 |
|---|---|---|---|
| 内容 | Codex が既存クリップを部品として振付 | 外部モーション取込→GTAリターゲット→`.ycd` | text-to-motion AI 生成→Phase2 へ |
| 新規モーションデータ | 作らない（既存 clip 参照のみ） | 作る（`.ycd` ベイク） | 作る（AI） |
| 出力 | FiveM リソース（Lua: `RequestAnimDict`+`TaskPlayAnim`） | `.ycd` を `stream/` にストリーム | 同左 |
| プレビュー | 実ゲーム（FiveM）ライブ | アプリ内 3D（Three.js）も可 | 同左 |
| リスク | 低（確実に動く） | 高 | 最高 |
| 主要依存 | codex CLI + カタログ JSON | Blender+Sollumz+CodeWalker | + GPU/HF motion model |

Phase 1〜3 は「FiveM リソースの殻」だけを共有する**別パイプライン**。本書は Phase 1 のみを仕様化する。

## 3. Phase 1 スコープ確定事項

- **核**: ユーザーのプロンプト → Codex が既存 GTA V アニメ clip を**部品**として選択・連結・
  パラメータ調整し、新規エモートを組み立てる。
- **厳密な線引き（重要）**: 振付は **Lua レベル**に限定する。複数 clip の連結は `TaskPlayAnim`
  の Lua チェーンで実現し、**モーションカーブのベイク（新規アニメデータ生成）は一切しない**。
  これが (b) モーションベイクに滑ると Phase 1 が密かに Phase 2 になるため、(a) に固定。
- **エクスポート形態**: **スタンドアロンの自己完結リソース**。rpemotes / scully 等の外部メニュー
  形式には依存しない（自作のエモートスクリプトとして出力）。
- **プレビュー**: 実ゲーム（FiveM）でのライブ確認。アプリ内 3D ビューアは Phase 1 では作らない。
- **AI 生成（Phase 3）はスコープ外**だが、データモデルとエクスポータに拡張点を残す。

## 4. 技術スタック

- デスクトップ: **Tauri v2**（Rust コア + WebView）
- フロントエンド: **React + TypeScript + Tailwind CSS**
- LLM オーケストレーション: **Codex CLI**（`codex exec`）。ChatGPT サブスク認証を使用し API 課金なし。
- カタログ: JSON（同梱データ）

### 検証済みの前提（実機確認）

- `codex exec --output-schema <FILE>` で JSON Schema を渡し**構造化 JSON 出力を強制**できる。
- `codex exec --json` / `-o, --output-last-message <FILE>` で結果を機械可読に取得できる。
- `codex login status` = "Logged in using ChatGPT"（サブスク認証で稼働）。
- アニメカタログのデータソース: [DurtyFree/gta-v-data-dumps](https://github.com/DurtyFree/gta-v-data-dumps)
  `animDictsCompact.json`（20,179 辞書 / 269,414 アニメ）、補助に
  [alexguirre list](https://alexguirre.github.io/animations-list/) / Pleb Masters Forge。

## 5. アーキテクチャ（コンポーネント）

各コンポーネントは単一責務・明確なインターフェースで分離する。

1. **Tauri Rust コア（バックエンド）**
   - 責務: codex CLI 実行、カタログ読込、バリデーション、ファイル生成、Preview Bridge への HTTP 送信。
   - 依存: codex CLI（外部プロセス）、カタログ JSON、ローカル FS。
2. **React + Tailwind UI（フロントエンド）**
   - 責務: プロンプト入力、生成結果（clip 列・フラグ）の可視化と手動微調整、エモート一覧、
     プレビュー実行、エクスポート操作。
   - 依存: Tauri command（IPC）経由でコアを呼ぶ。
3. **Animation Catalog（同梱データ + ビルドスクリプト）**
   - 責務: emote 向けにキュレーション＆タグ付けした辞書を提供。
   - 構成: `catalog.json`（人間可読名 + animDict + clip + tags + 既定フラグ）＋
     `dump.index`（全 dump を実在チェック用に検索可能化）。
   - ビルド: DurtyFree dump＋既存 emote 定義（rpemotes 等の意味づけ）＋ alexguirre/Forge メタを統合。
4. **Codex Orchestrator（Rust モジュール）**
   - 責務: プロンプト＋カタログ要約＋ `emote.schema.json` を組み立て `codex exec` を実行、
     構造化エモート仕様を取得。
   - インターフェース: `generate(prompt, catalogContext) -> EmoteSpec`。
5. **Validator（Rust モジュール）**
   - 責務: Codex が返した dict/clip がカタログ/dump に実在するか検証。不正は候補提示 or 再生成要求。
   - インターフェース: `validate(EmoteSpec) -> Result<EmoteSpec, ValidationReport>`。
6. **Exporter（Rust モジュール）**
   - 責務: `EmoteSpec` → スタンドアロン FiveM リソース（`fxmanifest.lua` + `client.lua` + データ）。
   - インターフェース: `export(EmoteSpec, outDir) -> ResourceManifest`。
7. **Preview Bridge（コンパニオン FiveM リソース）**
   - 責務: localhost で HTTP 受信し、受け取った `EmoteSpec` を自キャラで即再生。
   - 配置: `/opt/fivem` の既存サーバーに置ける開発用リソース。

## 6. データモデル（EmoteSpec）

```jsonc
{
  "name": "drunk_cheers",
  "displayName": "酔っ払い乾杯",
  "clips": [
    {
      "dict": "amb@world_human_drinking@coffee@male@idle_a",
      "clip": "idle_c",
      "blendIn": 1.0,
      "blendOut": 1.0,
      "duration": -1,          // -1 = clip 長 / ループ時無視
      "playbackRate": 0.9,
      "flags": ["AF_LOOPING", "AF_UPPERBODY"]  // movement/exit/loop など
    }
  ],
  "loop": true,
  "upperBodyOnly": false,
  "prop": { "model": "prop_beer_bottle", "bone": 18905, "offset": [0,0,0], "rot": [0,0,0] },
  "facial": { "dict": "facials@gen_male@base", "clip": "mood_drunk_1" },
  "movementType": "stationary",  // stationary | walkable | actionloop
  "meta": { "source": "codex", "schemaVersion": 1 }
}
```

- `emote.schema.json`（JSON Schema）で codex の出力形を強制し、UI 編集も同型を共有。
- 拡張点: 将来 Phase 2/3 で `clips[].source` に `ycd` 参照を追加できる構造にしておく
  （Phase 1 では `dict`/`clip` の既存参照のみ）。

## 7. データフロー

```
ユーザープロンプト
  → Rust(Orchestrator): カタログ要約 + emote.schema.json を添えて codex exec 実行
  → Codex: 構造化 EmoteSpec(JSON) を返す
  → Rust(Validator): dict/clip 実在チェック → 不正なら候補提示 or 再生成
  → UI: clip 列・フラグを可視化＆手動微調整
  → [プレビュー] Preview Bridge へ POST → 実ゲーム再生
     [エクスポート] Exporter → スタンドアロン FiveM リソース生成
```

## 8. エクスポート出力（スタンドアロンリソース）

生成物（例）:

```
my_emotes/
  fxmanifest.lua          # 自動生成
  client.lua              # /emote <name> コマンド + 再生ロジック（RequestAnimDict→TaskPlayAnim）
  emotes.json             # EmoteSpec の配列（データ駆動）
```

- 依存リソースなし。任意のサーバーの `resources/` に置けば動く。
- 複数エモートを 1 リソースに束ねてエクスポート可能。

## 9. Preview Bridge（実ゲームライブ確認）

- コンパニオン FiveM リソース `emoteforge_bridge` が `SetHttpHandler` で localhost のエンドポイントを開く。
- アプリは現在編集中の `EmoteSpec` を `POST http://localhost:<port>/preview` で送信。
- リソースが client イベント経由で自キャラに即再生（停止/ループ解除コマンドも提供）。
- これにより「生成 → その場で実ゲーム確認 → 微調整」のループが回る。

## 10. エラー処理（境界で検証）

- codex 実行失敗 / タイムアウト / 非 0 終了 → リトライ可能なエラーとして UI に提示。
- JSON Schema 不一致 → Validator がパースエラーを捕捉し再生成 or 手動修正へ誘導。
- clip 不実在（ハルシネーション）→ カタログから近い候補を提示。
- Preview Bridge 未接続 → 「FiveM 起動 / リソース有効化」を促す日本語メッセージ。
- 方針: エラーメッセージ本文は英語、UI 表示は日本語。エラーは握り潰さずログ（構造化 JSON）に残す。

## 11. テスト方針

- Validator / Exporter / schema パースは Rust のユニットテストで TDD。
- Codex オーケストレーションは codex 呼び出しを抽象化（trait）し、固定 JSON でモック。
- Exporter 出力の Lua は構文/スナップショットテスト。
- 実ゲーム確認は Preview Bridge を介した手動 E2E（`/opt/fivem`）。

## 12. Phase 1 の非スコープ（YAGNI）

- `.ycd` 生成、外部モーション取込、リターゲット（→ Phase 2）。
- AI モーション生成（→ Phase 3）。
- アプリ内 3D ビューア（Phase 1 は実ゲームプレビュー）。
- 外部メニュー（rpemotes/scully）互換エクスポート（スタンドアロンのみ）。

## 13. 既知のリスク / 留意点

- Tauri の Win11 向け最終ビルドは Windows 上 or クロスビルドで実施（開発は Linux の `tauri dev`）。
- Codex の出力品質はプロンプト設計とカタログ要約の質に依存 → 反復改善前提。
- カタログのキュレーション品質が UX を左右する → 初期は emote 実用域に絞って育てる。
