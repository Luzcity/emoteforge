// Tauri コマンドの型付きラッパ。コマンド名は src-tauri/src/commands.rs と一致させること。

import { invoke } from "@tauri-apps/api/core";
import type { EmoteSpec } from "../types/emote";

export interface CatalogEntry {
  key: string;
  displayName: string;
  dict: string;
  clip: string;
  category: string;
  tags: string[];
  defaults: { loop: boolean; upperBodyOnly: boolean; movementType: string };
}

export interface ValidationIssue {
  field: string;
  message: string;
  suggestions: string[];
}

export interface GeneratedEmote {
  spec: EmoteSpec;
  issues: ValidationIssue[];
}

export interface ResourceManifest {
  resourceName: string;
  dir: string;
  files: string[];
  emoteCount: number;
}

export interface YcdBuildResult {
  outPath: string;
  frameCount: number;
  boneCount: number;
  /** GTA ボーンへ対応付けできなかった元ボーン名（情報提示用）。 */
  unmapped: string[];
}

export const generateEmote = (prompt: string) =>
  invoke<GeneratedEmote>("generate_emote", { prompt });

export const validateEmote = (spec: EmoteSpec) =>
  invoke<GeneratedEmote>("validate_emote", { spec });

export const searchCatalog = (query: string, limit = 30) =>
  invoke<CatalogEntry[]>("search_catalog", { query, limit });

export const previewEmote = (spec: EmoteSpec) => invoke<void>("preview_emote", { spec });

export const stopPreview = () => invoke<void>("stop_preview");

export const setBridgeUrl = (url: string) => invoke<void>("set_bridge_url", { url });

export const setCodexModel = (model: string | null) => invoke<void>("set_codex_model", { model });

export const exportEmotes = (specs: EmoteSpec[], outDir: string, resourceName: string) =>
  invoke<ResourceManifest>("export_emotes", { specs, outDir, resourceName });

export const installBridgeResource = (resourcesDir: string) =>
  invoke<string>("install_bridge_resource", { resourcesDir });

// ---- Phase 2/3: .ycd パイプライン ----

export const importBvhYcdXml = (bvhPath: string, outPath: string) =>
  invoke<YcdBuildResult>("import_bvh_ycd_xml", { bvhPath, outPath });

export const generateAiMotionYcdXml = (
  prompt: string,
  runnerBin: string,
  runnerArgs: string[],
  outPath: string
) =>
  invoke<YcdBuildResult>("generate_ai_motion_ycd_xml", {
    prompt,
    runnerBin,
    runnerArgs,
    outPath,
  });
