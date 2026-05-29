import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("../lib/api", () => {
  const ycdResult = { outPath: "/out/motion.ycd.xml", frameCount: 10, boneCount: 5, unmapped: [] };
  return {
    importBvhYcdXml: vi.fn().mockResolvedValue(ycdResult),
    generateAiMotionYcdXml: vi.fn().mockResolvedValue(ycdResult),
  };
});
vi.mock("../lib/dialog", () => ({ pickFile: vi.fn(), pickSavePath: vi.fn() }));

import MotionPanel from "./MotionPanel";
import * as api from "../lib/api";
import { pickSavePath } from "../lib/dialog";

describe("MotionPanel", () => {
  beforeEach(() => vi.clearAllMocks());

  it("imports a bvh to ycd.xml with chosen output path", async () => {
    (pickSavePath as any).mockResolvedValue("/out/motion.ycd.xml");
    render(<MotionPanel onError={() => {}} onStatus={() => {}} />);
    fireEvent.change(screen.getByLabelText("bvhPath"), { target: { value: "/in/walk.bvh" } });
    fireEvent.click(screen.getByText("BVH → .ycd.xml"));
    await waitFor(() =>
      expect(api.importBvhYcdXml).toHaveBeenCalledWith("/in/walk.bvh", "/out/motion.ycd.xml")
    );
  });

  it("generates ai motion splitting runner args", async () => {
    (pickSavePath as any).mockResolvedValue("/out/ai.ycd.xml");
    render(<MotionPanel onError={() => {}} onStatus={() => {}} />);
    fireEvent.change(screen.getByLabelText("motionPrompt"), { target: { value: "wave hello" } });
    fireEvent.change(screen.getByLabelText("runnerBin"), { target: { value: "python3" } });
    fireEvent.change(screen.getByLabelText("runnerArgs"), { target: { value: "t2m.py --fast" } });
    fireEvent.click(screen.getByText("生成 → .ycd.xml"));
    await waitFor(() =>
      expect(api.generateAiMotionYcdXml).toHaveBeenCalledWith(
        "wave hello",
        "python3",
        ["t2m.py", "--fast"],
        "/out/ai.ycd.xml"
      )
    );
  });
});
