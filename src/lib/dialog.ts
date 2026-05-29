// フォルダ選択ダイアログのラッパ（テストでモックしやすくするため分離）。
import { open } from "@tauri-apps/plugin-dialog";

export async function pickDirectory(title: string): Promise<string | null> {
  const result = await open({ directory: true, multiple: false, title });
  if (typeof result === "string") return result;
  return null;
}
