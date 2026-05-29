#!/usr/bin/env node
// DurtyFree の animDictsCompact.json から emote 実用域を抽出し、
// タグ付き curated catalog.json と、存在検証用 dump_index.json を生成する。
//
// 入力: catalog/animDictsCompact.json (無ければ DUMP_URL から取得)
// 出力: catalog/catalog.json, catalog/dump_index.json
//
// 使い方: node catalog/build_catalog.mjs

import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const HERE = dirname(fileURLToPath(import.meta.url));
const DUMP_PATH = join(HERE, "animDictsCompact.json");
const DUMP_URL =
  "https://raw.githubusercontent.com/DurtyFree/gta-v-data-dumps/master/animDictsCompact.json";

// emote として有用な dict ファミリ（接頭辞）と既定の振る舞い。
const FAMILIES = [
  { prefix: "amb@world_human_", cat: "ambient", loop: true, upper: false },
  { prefix: "anim@mp_player_intcelebration", cat: "celebration", loop: true, upper: true },
  { prefix: "anim@mp_player_intupper", cat: "upper", loop: true, upper: true },
  { prefix: "mp_player_int", cat: "interaction", loop: false, upper: true },
  { prefix: "anim@mp_player_int", cat: "interaction", loop: false, upper: true },
  { prefix: "special_ped@", cat: "scenario", loop: false, upper: false },
  { prefix: "timetable@", cat: "daily", loop: true, upper: false },
  { prefix: "rcmnigel", cat: "pose", loop: false, upper: false },
  { prefix: "random@", cat: "reaction", loop: false, upper: false },
  { prefix: "reaction@", cat: "reaction", loop: false, upper: true },
  { prefix: "anim@amb@", cat: "ambient", loop: true, upper: false },
  { prefix: "rcm", cat: "pose", loop: false, upper: false },
];

// displayName / tags から除外するノイズトークン。
const NOISE = new Set([
  "amb", "world", "human", "anim", "mp", "player", "int", "ig", "male", "female",
  "a", "b", "c", "d", "e", "f", "base", "idle", "enter", "exit", "rcm", "ped",
  "special", "intro", "outro", "loop", "0", "1", "2", "3", "the",
]);

// 各 dict から代表クリップを選ぶ優先順。
const PREFERRED_CLIPS = ["base", "idle_a", "idle_c", "idle_b", "idle", "loop"];

function tokensFrom(dict, clip) {
  const raw = `${dict} ${clip}`.split(/[@_/\s\d]+/).map((t) => t.toLowerCase());
  const seen = new Set();
  const out = [];
  for (const t of raw) {
    if (t.length < 2 || NOISE.has(t) || seen.has(t)) continue;
    seen.add(t);
    out.push(t);
  }
  return out;
}

function titleCase(tokens) {
  return tokens.slice(0, 4).map((t) => t[0].toUpperCase() + t.slice(1)).join(" ");
}

function pickClips(dict, anims) {
  // 代表クリップ: PREFERRED に一致するもの優先、無ければ先頭 1 つ。
  const pref = PREFERRED_CLIPS.filter((p) => anims.includes(p));
  if (pref.length) return pref.slice(0, 1);
  // celebration / interaction 系は名詞的クリップが多いので最大 2 つ採用。
  return anims.slice(0, 1);
}

function familyOf(dict) {
  return FAMILIES.find((f) => dict.startsWith(f.prefix));
}

function sanitizeKey(dict, clip) {
  return `${dict}__${clip}`.replace(/[^a-z0-9]+/gi, "_").replace(/_+/g, "_").toLowerCase();
}

function main() {
  if (!existsSync(DUMP_PATH)) {
    console.error(`dump not found at ${DUMP_PATH}`);
    console.error(`download it first:\n  curl -sL "${DUMP_URL}" -o "${DUMP_PATH}"`);
    process.exit(1);
  }

  const dump = JSON.parse(readFileSync(DUMP_PATH, "utf8"));

  // dump_index.json: { dict: [clip, ...] } 形式（存在検証用）。
  const index = {};
  for (const e of dump) index[e.DictionaryName] = e.Animations;
  writeFileSync(join(HERE, "dump_index.json"), JSON.stringify(index));

  // curated catalog 生成。
  const entries = [];
  const usedKeys = new Set();
  for (const e of dump) {
    const fam = familyOf(e.DictionaryName);
    if (!fam) continue;
    for (const clip of pickClips(e.DictionaryName, e.Animations)) {
      const key = sanitizeKey(e.DictionaryName, clip);
      if (usedKeys.has(key)) continue;
      const tags = tokensFrom(e.DictionaryName, clip);
      if (tags.length === 0) continue;
      usedKeys.add(key);
      entries.push({
        key,
        displayName: titleCase(tags) || e.DictionaryName,
        dict: e.DictionaryName,
        clip,
        category: fam.cat,
        tags,
        defaults: {
          loop: fam.loop,
          upperBodyOnly: fam.upper,
          movementType: fam.upper ? "walkable" : "stationary",
        },
      });
    }
  }

  entries.sort((a, b) => a.key.localeCompare(b.key));
  writeFileSync(
    join(HERE, "catalog.json"),
    JSON.stringify({ version: 1, count: entries.length, entries }, null, 0)
  );
  console.log(
    `built catalog.json (${entries.length} entries) and dump_index.json (${Object.keys(index).length} dicts)`
  );
}

main();
