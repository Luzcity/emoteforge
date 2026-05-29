# Phase 2 / 3: `.ycd` パイプライン

外部モーション（BVH/FBX/mocap）や AI 生成モーションを GTA V スケルトンへリターゲットし、
`.ycd`（クリップ辞書）として FiveM にストリームするためのパイプライン。

## ⚠ 検証境界

`.ycd` の**バイナリ生成**と**FiveM 実ロード**は、次の外部ツールを要するため、本リポジトリの
Linux 開発環境では**検証していない**（実装はするが「未検証・外部ツール必須」と明記する）。

- [Sollumz](https://github.com/Sollumz/Sollumz)（Blender アドオン）: `.ycd.xml`(CodeWalker XML) ⇄ `.ycd`
- [CodeWalker](https://github.com/dexyfex/CodeWalker)（.NET, Windows）: XML ↔ バイナリ・フォーマット参照
- GPU もしくは HF Space（Phase 3 の text-to-motion モデル）

本リポジトリで**テスト済み**なのは次のロジック:
- BVH パース → `MotionClip`（`core/src/phase2/bvh.rs`）
- ヒューマノイド/Mixamo → GTA スケルトンのリターゲット（`core/src/phase2/retarget.rs`）
- `MotionClip` → CodeWalker `.ycd.xml` 生成（`core/src/phase2/ycd_xml.rs`、best-effort）
- text-to-motion 生成器の抽象＋外部ランナー連携（`core/src/phase3/generator.rs`）

`.ycd.xml` の要素/属性は CodeWalker の `Clip.cs`/`YcdFile.cs` の WriteXML を元に再現した
best-effort。実バイナリ変換前に CodeWalker/Sollumz で読み込み確認すること。

## パイプライン全体

```
[外部モーション BVH/FBX] ──┐
                          ├─→ MotionClip ─→ retarget(GTA) ─→ .ycd.xml ─(CodeWalker/Sollumz)→ .ycd ─→ resource/stream/
[AI text-to-motion]    ──┘
```

## 手順（Win11 想定）

1. アプリ or CLI で `.ycd.xml` を生成（`MotionClip` → `build_ycd_xml`）。
2. `.ycd.xml` を CodeWalker でインポート→ `.ycd` にエクスポート、または
   `blender --background --python phase2/blender/sollumz_export.py -- in.ycd.xml out_dir`。
3. 生成された `.ycd` を スタンドアロンリソースの `stream/` に配置（FiveM は `stream/` を自動ストリーム）。
4. `EmoteSpec` の clip を `source="ycd"`・`dict=<ycd 内辞書名>`・`clip=<クリップ名>` にして再生。

## Phase 3 のローカルランナー

`CommandMotionGenerator` はプロンプトを stdin で受け取り、`MotionClip` JSON を stdout に出力する
任意のコマンド（例: ローカル GPU 上の MoMask/MDM ランナー、HF Space を叩く Python）を呼ぶ。
出力 JSON の形は `core/src/phase2/motion.rs` の `MotionClip` に一致させること。
