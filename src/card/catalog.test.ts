import { describe, expect, it } from "vitest";
import type { LibrarySave } from "./api";
import { demoView } from "./demo";
import { filterCatalog } from "./catalog";

const base = demoView().saves[0];
function entry(title: string, code: string, file: string, capturedAt: string | null = null): LibrarySave {
  const save = { ...base, title, prodCode: code, identifier: "playthrough", region: "America" };
  return { path: `/saves/${file}`, relativePath: `${code}/${file}`, save, snapshots: [
    { path: `/snapshots/${file}`, contentId: "demo", capturedAt, sources: [], current: true, save },
  ] };
}

describe("local save catalog", () => {
  it("matches multiple search terms across metadata without changing the source order", () => {
    const saves = [entry("Zelda", "GAME-B", "first.mcs"), entry("Crash", "GAME-A", "second.mcs")];
    expect(filterCatalog(saves, " CRASH game-a second ", null, "title")).toEqual([saves[1]]);
    expect(saves[0].save.title).toBe("Zelda");
  });

  it("combines game filtering with search and distinguishes unknown games", () => {
    const saves = [entry("Save 1", "A", "one.mcs"), entry("Save 2", "B", "two.mcs"), entry("Save 3", "", "three.mcs")];
    expect(filterCatalog(saves, "save", "B", "title")).toEqual([saves[1]]);
    expect(filterCatalog(saves, "", "", "title")).toEqual([saves[2]]);
    expect(filterCatalog(saves, "missing", null, "title")).toEqual([]);
  });

  it("sorts titles naturally and uses file paths to keep equal titles stable", () => {
    const saves = [entry("Save 10", "A", "ten.mcs"), entry("Save 2", "A", "b.mcs"), entry("Save 2", "A", "a.mcs")];
    expect(filterCatalog(saves, "", null, "title")).toEqual([saves[2], saves[1], saves[0]]);
  });

  it("sorts by game before title", () => {
    const saves = [entry("Alpha", "B", "one.mcs"), entry("Zeta", "A", "two.mcs")];
    expect(filterCatalog(saves, "", null, "game")).toEqual([saves[1], saves[0]]);
  });

  it("uses the latest snapshot capture and leaves unknown or invalid dates last", () => {
    const saves = [entry("Unknown", "A", "one.mcs"), entry("Older", "A", "two.mcs", "2026-09-01T00:00:00Z"),
      entry("Newer", "A", "three.mcs", "2026-10-01T00:00:00Z"), entry("Invalid", "A", "four.mcs", "invalid")];
    saves[1].snapshots.push({ ...saves[1].snapshots[0], capturedAt: "2026-10-02T00:00:00Z" });
    expect(filterCatalog(saves, "", null, "recent")).toEqual([saves[1], saves[2], saves[3], saves[0]]);
  });
});
