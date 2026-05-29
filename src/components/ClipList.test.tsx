import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import ClipList from "./ClipList";
import { defaultClip } from "../types/emote";

describe("ClipList", () => {
  const clips = [defaultClip("dictA", "clipA"), defaultClip("dictB", "clipB")];

  it("renders a row per clip", () => {
    render(<ClipList clips={clips} onChange={() => {}} />);
    expect(screen.getAllByTestId("clip-row")).toHaveLength(2);
  });

  it("removes a clip", () => {
    const onChange = vi.fn();
    render(<ClipList clips={clips} onChange={onChange} />);
    fireEvent.click(screen.getByLabelText("clip-0-remove"));
    expect(onChange).toHaveBeenCalledWith([clips[1]]);
  });

  it("moves a clip down", () => {
    const onChange = vi.fn();
    render(<ClipList clips={clips} onChange={onChange} />);
    fireEvent.click(screen.getByLabelText("clip-0-down"));
    expect(onChange).toHaveBeenCalledWith([clips[1], clips[0]]);
  });

  it("edits the dict field", () => {
    const onChange = vi.fn();
    render(<ClipList clips={clips} onChange={onChange} />);
    fireEvent.change(screen.getByLabelText("clip-0-dict"), { target: { value: "newdict" } });
    expect(onChange).toHaveBeenCalledWith([{ ...clips[0], dict: "newdict" }, clips[1]]);
  });
});
