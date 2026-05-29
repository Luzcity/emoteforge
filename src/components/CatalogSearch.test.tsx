import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("../lib/api", () => ({ searchCatalog: vi.fn() }));
import * as api from "../lib/api";
import CatalogSearch from "./CatalogSearch";
import type { CatalogEntry } from "../lib/api";

const entry: CatalogEntry = {
  key: "amb_cheer",
  displayName: "Cheering",
  dict: "amb@world_human_cheering@male_a",
  clip: "base",
  category: "ambient",
  tags: ["cheering"],
  defaults: { loop: true, upperBodyOnly: false, movementType: "stationary" },
};

describe("CatalogSearch", () => {
  beforeEach(() => vi.clearAllMocks());

  it("searches and adds a catalog clip", async () => {
    (api.searchCatalog as any).mockResolvedValue([entry]);
    const onAdd = vi.fn();
    render(<CatalogSearch onAdd={onAdd} />);

    fireEvent.change(screen.getByLabelText("catalog-search"), { target: { value: "cheer" } });
    fireEvent.click(screen.getByText("検索"));

    await waitFor(() => expect(api.searchCatalog).toHaveBeenCalledWith("cheer", 20));
    const addBtn = await screen.findByLabelText("add-amb_cheer");
    fireEvent.click(addBtn);
    expect(onAdd).toHaveBeenCalledWith(entry);
  });

  it("does not search on empty query", () => {
    render(<CatalogSearch onAdd={() => {}} />);
    fireEvent.click(screen.getByText("検索"));
    expect(api.searchCatalog).not.toHaveBeenCalled();
  });
});
