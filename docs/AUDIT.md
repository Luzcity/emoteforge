# パフォーマンス / セキュリティ監査レポート

日付: 2026-05-29 / 対象: 全フェーズ実装後の EmoteForge

## パフォーマンス

`cargo run --release --example perf_catalog` 実測（開発機）:

| 項目 | 結果 | 評価 |
|---|---|---|
| `Catalog::load`（catalog.json 3,632 件） | 8 ms | 良好 |
| `with_dump_index`（dump_index.json 20,179 dicts / 269k clips） | 104 ms | 起動一度のみ。許容 |
| `search`（40 件返却） | 約 2.9 ms/クエリ | オンデマンド呼出。許容 |
| `contains`（存在検証） | O(1) HashMap | 良好 |

- ボトルネックは codex 呼び出し（外部・数秒）だが、タイムアウト/キャンセルを実装済み。
- 起動時の合計データロードは ~112 ms。修正不要。

## セキュリティ

### 確認済み（問題なし）
- **シークレット非ハードコード**: コードに API キー/トークン/パスワードなし。codex は ChatGPT
  ログイン（外部認証）を使用しコード内に資格情報を持たない。
- **コマンド注入なし**: `Command::new(bin).args(...)` でシェルを介さず、プロンプトは stdin 渡し。
  引数連結・`sh -c` 不使用。
- **XSS なし**: React のテキストレンダリングのみ。`dangerouslySetInnerHTML`/`innerHTML`/`eval` 不使用。
- **パストラバーサル対策**: エクスポートのリソース名は `is_valid_resource_name` で `../` 等を拒否。
- **入力検証**: codex 出力は JSON Schema 強制＋カタログ実在検証（ハルシネーション排除）。
  数値は範囲クランプ、名前はサニタイズ。

### 適用したハードニング / 修正
- **CSP を有効化**（従来 `null`）: `default-src 'self'` 系の制限的ポリシーを `tauri.conf.json` に設定。
- **Tauri capabilities** を最小化（`core:default` + `dialog:default` のみ）。アプリコマンドは
  `main` ウィンドウからのみ到達可能（リモート到達不可）。
- **バンドルリソースパスのバグ修正**: `tauri.conf.json` は `../catalog/...` を列挙し、Tauri v2 は
  バンドル時に `../` を `_up_/` へ再マップする（`$RESOURCE/_up_/catalog/catalog.json`）。当初の
  `state.rs` は `catalog/catalog.json` を解決しており、**バンドル版 Win11 で `AppState::load` が
  失敗し起動不能**になる潜在バグだった（dev フォールバックが開発時に隠蔽）。`_up_/` 候補・
  `resource_dir` 直下・dev フォールバックを順に試すよう修正。

### 実起動スモーク（dev）
- `xvfb-run ./target/debug/emoteforge` で 12 秒間クラッシュせず稼働を確認。`AppState::load`
  成功（catalog/dump_index/schema ロード）・ウィンドウ生成・イベントループ稼働。
  ※これは dev パスでの起動確認。バンドル版 `_up_/` パスは実 Win11 ビルドでの確認が必要。

### 既知の留意点（リスク受容 + 文書化）
- **Preview Bridge は開発専用**: `emoteforge_bridge` の HTTP ハンドラは未認証で `POST /preview`
  を受け、全クライアントに再生イベントを送る。**本番サーバーには配置しないこと**（fxmanifest に
  dev-only 明記）。本番運用ではバンドル解除を推奨。
- **npm 依存の moderate 脆弱性（dev のみ）**: `esbuild`(GHSA-67mh-4wv8-2f99) が vitest/vite-node の
  推移依存に存在。**開発サーバー限定**で、出荷される Tauri アプリ（ビルド済み静的 dist）には
  含まれない。破壊的な `vitest@4` 強制更新は見送り、記録に留める。

### Phase 2/3 の検証境界
- `.ycd` バイナリ生成・FiveM 実ロードは CodeWalker/Blender(Sollumz)/GPU を要し未検証。
  該当コードは best-effort として明記済み（`phase2/README.md`、各モジュール冒頭コメント）。

## Codex レスバ・レビュー（3 ラウンド）で判明・修正した点

実コードを Codex(`codex exec`)に読ませ、反論しながら 3 ラウンド議論して洗い出した。

修正済み:
- **検証ゲートの一元化**: `export_emotes` / `preview_emote` の両方で全 emote を `validate()`
  してから処理（無効なら中止）。編集時も正規化を UI へ書き戻し。
- **anim flag の修正**: 誤った `51` 既定を撤廃し、`loop=1 / walkable upper=49(16|32) / one-shot=0`
  の正準値に。`AF_TAG_SYNC_OUT` を `64`→`32768` に訂正（`64=REORIENT`）。
- **facial を `PlayFacialAnim`** に変更（body skeleton の secondary slot 誤用を是正）。さらに
  `validate()` で facial dict/clip の実在も検証（不在時の沈黙失敗を防止）。
- **日本語プロンプト対策**: 検索が薄い時のみ下限(16 件)までカテゴリ横断サンプルで候補底上げ
  （常時 40 件まで埋めてノイズ化させない）。
- **UI の致命的欠落を解消**: カタログ検索・クリップ追加・prop/表情編集 UI を追加
  （従来は空 emote にクリップを足せなかった）。

判断を保留/撤回した点:
- **ycd の移動(translation)チャンネル**: 一旦追加したが、CodeWalker のトラック別チャンネル
  仕様を検証できないまま足すとファイル全体が読めなくなる恐れがある（ドメイン批評）。
  rotations-only に戻し、移動データは MotionClip に保持。実サンプルで仕様確定後に追加する。

## 結論
出荷対象（コアロジック + Tauri アプリ）に未修正の CRITICAL/High なし。CSP 有効化を適用。
Codex レビューで判明した実バグ（検証ゲート抜け・flag 誤り・facial 誤用・候補ノイズ）は修正済み。
Phase 2 の `.ycd` 互換性と prop プリセット拡充は外部ツール検証待ちの既知課題。
