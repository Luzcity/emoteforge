import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("./lib/api", () => ({
  generateEmote: vi.fn(),
  validateEmote: vi.fn(),
  previewEmote: vi.fn(),
  stopPreview: vi.fn(),
  exportEmotes: vi.fn(),
  searchCatalog: vi.fn().mockResolvedValue([]),
  codexLoginStatus: vi.fn().mockResolvedValue({ loggedIn: false, method: null, detail: "" }),
  codexLogin: vi.fn(),
  codexLogout: vi.fn(),
}));
vi.mock("./lib/dialog", () => ({ pickDirectory: vi.fn() }));

import App from "./App";
import * as api from "./lib/api";
import type { EmoteSpec } from "./types/emote";

const sampleSpec: EmoteSpec = {
  name: "cheer",
  displayName: "乾杯",
  clips: [
    {
      source: "builtin",
      dict: "amb@world_human_cheering@male_a",
      clip: "base",
      blendIn: 1,
      blendOut: 1,
      duration: -1,
      playbackRate: 1,
      flags: [],
    },
  ],
  loop: true,
  upperBodyOnly: false,
  movementType: "stationary",
  meta: { source: "codex", schemaVersion: 1 },
};

describe("App", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("generates an emote and shows it in the editor", async () => {
    (api.generateEmote as any).mockResolvedValue({ spec: sampleSpec, issues: [] });

    render(<App />);
    fireEvent.change(screen.getByLabelText("prompt"), { target: { value: "乾杯する" } });
    fireEvent.click(screen.getByText("生成"));

    await waitFor(() => expect(api.generateEmote).toHaveBeenCalledWith("乾杯する"));
    // 表示名が editor に出る
    expect(await screen.findByDisplayValue("乾杯")).toBeInTheDocument();
    // ライブラリに 1 件
    expect(screen.getByTestId("library-item")).toBeInTheDocument();
  });

  it("surfaces generation errors", async () => {
    (api.generateEmote as any).mockRejectedValue("codex failed");
    render(<App />);
    fireEvent.change(screen.getByLabelText("prompt"), { target: { value: "x" } });
    fireEvent.click(screen.getByText("生成"));
    expect(await screen.findByRole("alert")).toHaveTextContent("codex failed");
  });

  it("shows validation issues from edits", async () => {
    (api.generateEmote as any).mockResolvedValue({ spec: sampleSpec, issues: [] });
    (api.validateEmote as any).mockResolvedValue({
      spec: sampleSpec,
      issues: [{ field: "clips[0].clip", message: "unknown animation", suggestions: ["base"] }],
    });

    render(<App />);
    fireEvent.change(screen.getByLabelText("prompt"), { target: { value: "x" } });
    fireEvent.click(screen.getByText("生成"));
    await screen.findByDisplayValue("乾杯");

    fireEvent.change(screen.getByLabelText("name"), { target: { value: "renamed" } });
    await waitFor(() => expect(api.validateEmote).toHaveBeenCalled());
    expect(await screen.findByText(/unknown animation/)).toBeInTheDocument();
  });

  it("clears validation issues when switching emotes", async () => {
    (api.generateEmote as any)
      .mockResolvedValueOnce({ spec: { ...sampleSpec, name: "a", displayName: "AAA" }, issues: [] })
      .mockResolvedValueOnce({
        spec: { ...sampleSpec, name: "b", displayName: "BBB" },
        issues: [],
      });
    (api.validateEmote as any).mockResolvedValue({
      spec: sampleSpec,
      issues: [{ field: "clips[0].clip", message: "unknown animation", suggestions: [] }],
    });

    render(<App />);
    const promptInput = screen.getByLabelText("prompt");
    // emote A
    fireEvent.change(promptInput, { target: { value: "a" } });
    fireEvent.click(screen.getByText("生成"));
    await screen.findByDisplayValue("AAA");
    // emote B
    fireEvent.change(promptInput, { target: { value: "b" } });
    fireEvent.click(screen.getByText("生成"));
    await screen.findByDisplayValue("BBB");

    // B を編集して issues を出す
    fireEvent.change(screen.getByLabelText("name"), { target: { value: "renamed" } });
    expect(await screen.findByText(/unknown animation/)).toBeInTheDocument();

    // A に切り替えると B の issues は消える
    fireEvent.click(screen.getAllByTestId("library-item")[0]);
    await waitFor(() => expect(screen.queryByText(/unknown animation/)).not.toBeInTheDocument());
  });

  it("clears stale issues when creating a new empty emote", async () => {
    (api.generateEmote as any).mockResolvedValue({ spec: sampleSpec, issues: [] });
    (api.validateEmote as any).mockResolvedValue({
      spec: sampleSpec,
      issues: [{ field: "clips[0].clip", message: "unknown animation", suggestions: [] }],
    });

    render(<App />);
    fireEvent.change(screen.getByLabelText("prompt"), { target: { value: "x" } });
    fireEvent.click(screen.getByText("生成"));
    await screen.findByDisplayValue("乾杯");
    fireEvent.change(screen.getByLabelText("name"), { target: { value: "renamed" } });
    expect(await screen.findByText(/unknown animation/)).toBeInTheDocument();

    // 空のエモートを新規作成すると前の issues は消える
    fireEvent.click(screen.getByText("＋ 空のエモート"));
    await waitFor(() => expect(screen.queryByText(/unknown animation/)).not.toBeInTheDocument());
  });

  it("does not overwrite the name field with the normalized value while typing", async () => {
    (api.generateEmote as any).mockResolvedValue({ spec: sampleSpec, issues: [] });
    // 検証は正規化済み name を返すが、編集中に書き戻してはならない。
    (api.validateEmote as any).mockResolvedValue({
      spec: { ...sampleSpec, name: "wavehello" },
      issues: [],
    });

    render(<App />);
    fireEvent.change(screen.getByLabelText("prompt"), { target: { value: "x" } });
    fireEvent.click(screen.getByText("生成"));
    await screen.findByDisplayValue("乾杯");

    const nameInput = screen.getByLabelText("name") as HTMLInputElement;
    fireEvent.change(nameInput, { target: { value: "wave hello" } });
    await waitFor(() => expect(api.validateEmote).toHaveBeenCalled());
    // 正規化された "wavehello" ではなく、ユーザーが打った値が残る。
    expect(nameInput.value).toBe("wave hello");
  });

  it("previews the active emote in-game", async () => {
    (api.generateEmote as any).mockResolvedValue({ spec: sampleSpec, issues: [] });
    (api.previewEmote as any).mockResolvedValue(undefined);

    render(<App />);
    fireEvent.change(screen.getByLabelText("prompt"), { target: { value: "x" } });
    fireEvent.click(screen.getByText("生成"));
    await screen.findByDisplayValue("乾杯");

    fireEvent.click(screen.getByText("実ゲームでプレビュー"));
    await waitFor(() => expect(api.previewEmote).toHaveBeenCalledWith(sampleSpec));
  });
});
