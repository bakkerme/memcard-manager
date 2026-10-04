import { useEffect, useMemo, useRef, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import type { LibraryView } from "../card";
import { CATALOG_PAGE_SIZE, filterCatalog, type CatalogSort } from "../card/catalog";
import { IconArchive, IconFolder, IconSearch, IconSettings } from "../icons";
import { PixelIcon } from "./PixelIcon";
import { GameDetailsButton } from "./GameDetails";

export function LocalBackups({ library, loading, busy, error, frameTick, onSettings, onReveal }: {
  library: LibraryView | null;
  loading: boolean;
  busy: boolean;
  error: string | null;
  frameTick: number;
  onSettings: () => void;
  onReveal: (path: string) => void;
}) {
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [snapshotPath, setSnapshotPath] = useState("");
  const [query, setQuery] = useState("");
  const [game, setGame] = useState<string | null>(null);
  const [sort, setSort] = useState<CatalogSort>("title");
  const [page, setPage] = useState(1);
  const resultsRef = useRef<HTMLDivElement>(null);
  const selected = library?.saves.find(entry => entry.path === selectedPath) ?? null;
  const chosenSnapshot = selected?.snapshots.find(item => item.path === snapshotPath) ?? null;
  const snapshot = chosenSnapshot ?? selected?.snapshots.find(item => item.current) ?? null;
  const shownSave = chosenSnapshot?.save ?? selected?.save ?? null;
  const shownPath = chosenSnapshot?.path ?? selected?.path;
  const groups = useMemo(() => {
    const result = new Map<string, NonNullable<LibraryView>["saves"]>();
    for (const entry of library?.saves ?? []) {
      const key = entry.save.prodCode || "Unknown game";
      const saves = result.get(key) ?? [];
      saves.push(entry);
      result.set(key, saves);
    }
    return new Map([...result].sort(([a], [b]) => a.localeCompare(b)));
  }, [library]);
  const activeGame = game !== null && groups.has(game || "Unknown game") ? game : null;
  const matches = useMemo(() => filterCatalog(library?.saves ?? [], query, activeGame, sort), [library, query, activeGame, sort]);
  const pageCount = Math.max(1, Math.ceil(matches.length / CATALOG_PAGE_SIZE));
  const currentPage = Math.min(page, pageCount);
  const start = (currentPage - 1) * CATALOG_PAGE_SIZE;
  const entries = matches.slice(start, start + CATALOG_PAGE_SIZE);
  useEffect(() => { setPage(1); }, [query, activeGame, sort, library]);
  useEffect(() => { resultsRef.current?.scrollTo({ top: 0 }); }, [currentPage, query, activeGame, sort]);
  function clearFilters() { setQuery(""); setGame(null); setPage(1); }
  return (
    <main className="workspace library-workspace" aria-busy={loading}>
      <div className="collection-heading"><IconArchive /><h2>Local saves</h2></div>
      {error && <div className="banner error" role="alert">{error}</div>}
      {!!library?.warnings.length && <div className="banner" role="status">
        <details><summary>{library.warnings.length} file or folder{library.warnings.length === 1 ? "" : "s"} could not be read</summary>
          <ul>{library.warnings.map((warning, i) => <li key={i}>{warning}</li>)}</ul>
        </details>
      </div>}
      <div className="workspace-body library-body">
        <section className="library-saves" aria-label="Local save library">
          <div className="catalog-toolbar">
            <label className="catalog-search">
              <IconSearch />
              <input type="search" aria-label="Search local saves" placeholder="Search saves, games or files" value={query}
                onChange={event => setQuery(event.target.value)} />
            </label>
            <div className="catalog-filters">
              <label>Game
                <select aria-label="Game" value={activeGame === null ? "all" : `game:${activeGame}`} onChange={event => setGame(event.target.value === "all" ? null : event.target.value.slice(5))}>
                  <option value="all">All games ({groups.size})</option>
                  {Array.from(groups, ([code, saves]) => <option key={code} value={`game:${saves[0].save.prodCode}`}>
                    {code} ({saves.length})
                  </option>)}
                </select>
              </label>
              <label>Sort
                <select aria-label="Sort" value={sort} onChange={event => setSort(event.target.value as CatalogSort)}>
                  <option value="title">Save title</option>
                  <option value="game">Game code</option>
                  <option value="recent">Latest capture</option>
                </select>
              </label>
            </div>
            <div className="catalog-summary">
              <p className="library-count" role="status">{loading ? "Reading local backups…" : query || activeGame !== null
                ? `${matches.length} of ${library?.saves.length ?? 0} saves`
                : `${library?.saves.length ?? 0} saves · ${groups.size} games`}</p>
              {(query || activeGame !== null) && <button type="button" className="catalog-clear" onClick={clearFilters}>Clear filters</button>}
            </div>
          </div>
          <div className="catalog-results" ref={resultsRef} tabIndex={0} role="region" aria-label="Save results">
          {entries.length ? <div className="library-grid">
                {entries.map(entry => (
                  <div className={`tile library-tile ${entry.path === selectedPath ? "selected" : ""}`} key={entry.path}>
                    <button type="button" className="tile-hit" aria-pressed={entry.path === selectedPath}
                      onClick={() => { setSelectedPath(entry.path); setSnapshotPath(""); }}>
                      <PixelIcon frames={entry.save.frames} frameIndex={frameTick % (entry.save.frameCount || 1)} dim={entry.save.deleted} />
                      <span className="tile-title">{entry.save.title}</span>
                      <span className="catalog-game-code">{entry.save.prodCode || "Unknown game"}</span>
                      <span className="tile-blocks">{entry.save.linkedSlots.length} block{entry.save.linkedSlots.length === 1 ? "" : "s"}{entry.save.deleted ? " · Deleted" : ""}</span>
                    </button>
                  </div>
                ))}
              </div> : !loading && (
            <div className="empty-board">
              <h2>{library?.saves.length ? "No matching saves" : library?.directory ? "No saves found" : "Your local save library"}</h2>
              <p className="muted">{library?.saves.length ? "Try another title, game code or filename, or clear your filters."
                : library?.directory
                ? "This directory has no readable .mcs saves. Open a card and use Sync to add its saves, or change the collection folder in Settings."
                : "Choose a collection folder in Settings, then use Sync on an open card to add its saves."}</p>
              {!library?.directory && <div><button type="button" className="btn" disabled={busy} onClick={onSettings}><IconSettings />Open Settings</button></div>}
              {!isTauri() && !library?.directory && <p className="muted">Choose a local directory in the desktop app.</p>}
            </div>
          )}
          </div>
          {!!matches.length && <nav className="catalog-pagination" aria-label="Save catalog pages">
            <span>{start + 1}–{Math.min(start + CATALOG_PAGE_SIZE, matches.length)} of {matches.length}</span>
            <div>
              <button type="button" className="btn" disabled={currentPage === 1} onClick={() => setPage(currentPage - 1)}>Previous</button>
              <span role="status">Page {currentPage} of {pageCount}</span>
              <button type="button" className="btn" disabled={currentPage === pageCount} onClick={() => setPage(currentPage + 1)}>Next</button>
            </div>
          </nav>}
        </section>
        <aside className="inspector library-inspector" aria-label="Save inspector" tabIndex={0}>
          {selected && shownSave ? <>
            <div className="inspector-hero">
              <PixelIcon frames={shownSave.frames} frameIndex={frameTick % (shownSave.frameCount || 1)} dim={shownSave.deleted} />
              <div className="inspector-title"><h2>{shownSave.title}</h2></div>
              <dl>
                <div><dt>Region</dt><dd>{shownSave.region}</dd></div>
                <div><dt>Product</dt><dd>{shownSave.prodCode || "—"}</dd></div>
                <div><dt>Identifier</dt><dd>{shownSave.identifier || "—"}</dd></div>
                <div><dt>File</dt><dd>{chosenSnapshot ? chosenSnapshot.path.split(/[\\/]/).slice(-3).join("/") : selected.relativePath}</dd></div>
              </dl>
            </div>
            <p className="inspector-size">{shownSave.linkedSlots.length} block{shownSave.linkedSlots.length === 1 ? "" : "s"} · {shownSave.sizeKb} KB{shownSave.deleted ? " · Deleted save" : ""}</p>
            <GameDetailsButton save={shownSave} />
            <section className="snapshot-history" aria-label="Save snapshots">
              <label htmlFor="snapshot-choice">Snapshots ({selected.snapshots.length})</label>
              {selected.snapshots.length ? <>
                <select id="snapshot-choice" value={chosenSnapshot?.path ?? ""} onChange={event => setSnapshotPath(event.target.value)}>
                  <option value="">Latest dump</option>
                  {selected.snapshots.map(item => <option value={item.path} key={item.path}>
                    {item.capturedAt ? new Date(item.capturedAt).toLocaleString() : "Capture time unknown"} · {item.contentId.slice(0, 8)}{item.current ? " · Latest" : ""}
                  </option>)}
                </select>
                {snapshot && <>
                  <p className="snapshot-id" title={snapshot.contentId}>Snapshot ID: {snapshot.contentId.slice(0, 12)}</p>
                  {snapshot.sources.length ? <details>
                    <summary>Captured from {snapshot.sources.length} card image{snapshot.sources.length === 1 ? "" : "s"}</summary>
                    {snapshot.sources.map(source => <p key={`${source.imageId}-${source.sourceName}-${source.source}`}>
                      {source.sourceName} ({source.source === "usb" ? "adaptor read" : "card file"})
                    </p>)}
                  </details> : <p className="muted">Preserved from an existing backup. Original card source unknown.</p>}
                </>}
                <p className="muted">Snapshots share a game filename and may contain different playthroughs.</p>
              </> : <p className="muted">Sync this save to start keeping snapshots.</p>}
            </section>
            <button className="btn" onClick={() => shownPath && onReveal(shownPath)} disabled={busy || !isTauri()}><IconFolder />Reveal in Finder</button>
          </> : <><h3>Inspector</h3><p className="muted">Select a backup to inspect its metadata and file location.</p></>}
        </aside>
      </div>
    </main>
  );
}
