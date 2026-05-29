import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("../lib/api", () => ({
  setBridgeUrl: vi.fn().mockResolvedValue(undefined),
  setCodexModel: vi.fn().mockResolvedValue(undefined),
  installBridgeResource: vi.fn().mockResolvedValue("/res/emoteforge_bridge"),
}));
vi.mock("../lib/dialog", () => ({ pickDirectory: vi.fn() }));

import SettingsPanel from "./SettingsPanel";
import * as api from "../lib/api";
import { pickDirectory } from "../lib/dialog";

describe("SettingsPanel", () => {
  beforeEach(() => vi.clearAllMocks());

  it("applies the bridge url", async () => {
    render(<SettingsPanel onError={() => {}} onStatus={() => {}} />);
    fireEvent.change(screen.getByLabelText("bridgeUrl"), {
      target: { value: "http://10.0.0.5:30120/emoteforge_bridge" },
    });
    fireEvent.click(screen.getAllByText("適用")[0]);
    await waitFor(() =>
      expect(api.setBridgeUrl).toHaveBeenCalledWith("http://10.0.0.5:30120/emoteforge_bridge")
    );
  });

  it("installs the bridge into a chosen directory", async () => {
    (pickDirectory as any).mockResolvedValue("/srv/fivem/resources");
    render(<SettingsPanel onError={() => {}} onStatus={() => {}} />);
    fireEvent.click(screen.getByText("ブリッジを resources に導入"));
    await waitFor(() =>
      expect(api.installBridgeResource).toHaveBeenCalledWith("/srv/fivem/resources")
    );
  });

  it("sends null when the model field is empty", async () => {
    render(<SettingsPanel onError={() => {}} onStatus={() => {}} />);
    fireEvent.click(screen.getAllByText("適用")[1]);
    await waitFor(() => expect(api.setCodexModel).toHaveBeenCalledWith(null));
  });
});
