#!/usr/bin/env python3
"""
Blender(Sollumz) ヘッドレスで .ycd.xml(CodeWalker XML) を .ycd バイナリへ変換する補助スクリプト。

検証境界（重要）:
  本スクリプトは Blender + Sollumz アドオンがインストールされた Windows/Linux 上でのみ動作する。
  リポジトリの CI/開発環境（Blender 不在）では実行・検証できない。ロジックは Sollumz の
  YCD import/export 機能に依存する（CodeWalker XML を中間形式とする方式）。

前提:
  - Blender 4.x/5.x
  - Sollumz アドオン有効化済み (https://github.com/Sollumz/Sollumz)

使い方:
  blender --background --python phase2/blender/sollumz_export.py -- <input.ycd.xml> <output_dir>

処理の流れ:
  1) CodeWalker XML(.ycd.xml) を Sollumz の YCD import で読み込む
  2) Sollumz の YCD export で <output_dir>/<name>.ycd を書き出す

注意:
  Sollumz のバージョンにより operator 名は変わり得る。下記は一般的な名称の例で、
  実環境に合わせて調整すること。
"""

import sys
import os


def main(argv):
    if len(argv) < 2:
        print("usage: blender --background --python sollumz_export.py -- <input.ycd.xml> <output_dir>")
        return 1

    input_xml = argv[0]
    output_dir = argv[1]
    os.makedirs(output_dir, exist_ok=True)

    try:
        import bpy  # noqa: F401  (Blender 実行時のみ存在)
    except ImportError:
        print("ERROR: must run inside Blender (bpy unavailable)")
        return 2

    import bpy

    # クリーンシーン
    bpy.ops.wm.read_factory_settings(use_empty=True)

    # --- Sollumz YCD import ---
    # operator 名は Sollumz バージョン依存。代表例:
    #   bpy.ops.sollumz.importycd(filepath=input_xml)
    try:
        bpy.ops.sollumz.importycd(filepath=input_xml)  # type: ignore[attr-defined]
    except Exception as e:  # pragma: no cover - Blender 実行時のみ
        print(f"ERROR: Sollumz YCD import failed: {e}")
        print("Sollumz が有効か、operator 名が正しいか確認してください。")
        return 3

    # --- Sollumz YCD export ---
    try:
        bpy.ops.sollumz.exportycd(directory=output_dir)  # type: ignore[attr-defined]
    except Exception as e:  # pragma: no cover
        print(f"ERROR: Sollumz YCD export failed: {e}")
        return 4

    print(f"OK: exported .ycd to {output_dir}")
    return 0


if __name__ == "__main__":
    # "--" 以降が本スクリプト用の引数。
    argv = sys.argv
    if "--" in argv:
        argv = argv[argv.index("--") + 1:]
    else:
        argv = []
    raise SystemExit(main(argv))
