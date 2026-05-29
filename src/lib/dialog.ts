// フォルダ/ファイル選択ダイアログのラッパ（テストでモックしやすくするため分離）。
import { open, save } from "@tauri-apps/plugin-dialog";

export async function pickDirectory(title: string): Promise<string | null> {
  const result = await open({ directory: true, multiple: false, title });
  if (typeof result === "string") return result;
  return null;
}

/** 単一ファイルを選択する。filters 例: [{ name: "BVH", extensions: ["bvh"] }] */
export async function pickFile(
  title: string,
  filters?: { name: string; extensions: string[] }[]
): Promise<string | null> {
  const result = await open({ directory: false, multiple: false, title, filters });
  if (typeof result === "string") return result;
  return null;
}

/** 保存先パスを選択する。 */
export async function pickSavePath(
  title: string,
  defaultPath?: string,
  filters?: { name: string; extensions: string[] }[]
): Promise<string | null> {
  const result = await save({ title, defaultPath, filters });
  return result ?? null;
}
