import type { LibrarySave } from "./api";

export type CatalogSort = "title" | "game" | "recent";
export const CATALOG_PAGE_SIZE = 24;

function latestCapture(entry: LibrarySave): number {
  return entry.snapshots.reduce((latest, snapshot) => {
    const time = snapshot.capturedAt ? Date.parse(snapshot.capturedAt) : 0;
    return Number.isFinite(time) ? Math.max(latest, time) : latest;
  }, 0);
}

export function filterCatalog(saves: LibrarySave[], query: string, game: string | null, sort: CatalogSort): LibrarySave[] {
  const terms = query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
  return saves.filter(entry => {
    if (game !== null && entry.save.prodCode !== game) return false;
    const searchable = [entry.save.title, entry.save.prodCode, entry.save.identifier, entry.relativePath, entry.save.region]
      .join(" ").toLocaleLowerCase();
    return terms.every(term => searchable.includes(term));
  }).sort((a, b) => {
    const primary = sort === "recent" ? latestCapture(b) - latestCapture(a)
      : sort === "game" ? a.save.prodCode.localeCompare(b.save.prodCode) : 0;
    return primary || a.save.title.localeCompare(b.save.title, undefined, { numeric: true, sensitivity: "base" })
      || a.relativePath.localeCompare(b.relativePath) || a.path.localeCompare(b.path);
  });
}
