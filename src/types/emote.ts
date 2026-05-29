// EmoteSpec — Rust (core/src/model/emote.rs) と JSON Schema (schema/emote.schema.json) に一致させること。

export type ClipSource = "builtin" | "ycd";
export type MovementType = "stationary" | "walkable" | "actionLoop";

export interface ClipRef {
  source: ClipSource;
  dict: string;
  clip: string;
  blendIn: number;
  blendOut: number;
  duration: number; // ms, -1 = clip length
  playbackRate: number;
  flags: string[];
}

export interface Prop {
  model: string;
  bone: number;
  offset: [number, number, number];
  rot: [number, number, number];
}

export interface Facial {
  dict: string;
  clip: string;
}

export interface Meta {
  source: string;
  schemaVersion: number;
}

export interface EmoteSpec {
  name: string;
  displayName: string;
  clips: ClipRef[];
  loop: boolean;
  upperBodyOnly: boolean;
  prop?: Prop;
  facial?: Facial;
  movementType: MovementType;
  meta: Meta;
}

export const defaultClip = (dict: string, clip: string): ClipRef => ({
  source: "builtin",
  dict,
  clip,
  blendIn: 1.0,
  blendOut: 1.0,
  duration: -1,
  playbackRate: 1.0,
  flags: [],
});

export const emptyEmote = (): EmoteSpec => ({
  name: "new_emote",
  displayName: "新規エモート",
  clips: [],
  loop: false,
  upperBodyOnly: false,
  movementType: "stationary",
  meta: { source: "manual", schemaVersion: 1 },
});
