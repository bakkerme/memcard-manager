import { useMemo, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import type { LibraryView } from "../card";
import { IconArchive, IconFolder, IconRefresh } from "../icons";
import { PixelIcon } from "./PixelIcon";

export function LocalBackups({ library, loading, busy, error, frameTick, onChoose, onRefresh, onReveal }: {
  library: LibraryView | null;
  loading: boolean;
  busy: boolean;
  error: string | null;
  frameTick: number;
  onChoose: () => void;
  onRefresh: () => void;
  onReveal: (path: string) => void;
}) {
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [snapshotPath, setSnapshotPath] = useState("");
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
    return result;
  }, [library]);
  const disabled = busy || loading || !isTauri();
  return (
    <main className="workspace library-workspace" aria-busy={loading}>
      <div className="card-head library-head">
        <IconArchive />
        <div className="card-id">
          <h2>Local backups</h2>
          <p className="library-path">{library?.displayPath ?? "Choose a directory for your save library"}</p>
        </div>
        <div className="library-actions">
          <button className="btn" disabled={disabled} onClick={onChoose}
            title={isTauri() ? "Choose the directory used by Local backups and Sync" : "Requires the desktop app"}>
            <IconFolder />{library?.directory ? "Change folder" : "Choose folder"}
          </button>
          <button className="btn" disabled={disabled || !library?.directory} onClick={onRefresh}>
            <IconRefresh />{loading ? "Refreshing…" : "Refresh"}
          </button>
        </div>
      </div>
      {error && <div className="banner error" role="alert">{error}</div>}
      {!!library?.warnings.length && <div className="banner" role="status">
        <details><summary>{library.warnings.length} file or folder{library.warnings.length === 1 ? "" : "s"} could not be read</summary>
          <ul>{library.warnings.map((warning, i) => <li key={i}>{warning}</li>)}</ul>
        </details>
      </div>}
      <div className="workspace-body library-body">
        <section className="library-saves" aria-label="Local save library">
          <p className="library-count" role="status">{loading ? "Reading local backups…" : `${library?.saves.length ?? 0} saves · ${groups.size} games`}</p>
          {groups.size ? Array.from(groups, ([game, entries]) => (
            <section className="library-game" key={game} aria-label={game}>
              <h3>{game}</h3>
              <div className="library-grid">
                {entries.map(entry => (
                  <div className={`tile library-tile ${entry.path === selectedPath ? "selected" : ""}`} key={entry.path}>
                    <button type="button" className="tile-hit" aria-pressed={entry.path === selectedPath}
                      onClick={() => { setSelectedPath(entry.path); setSnapshotPath(""); }}>
                      <PixelIcon frames={entry.save.frames} frameIndex={frameTick % (entry.save.frameCount || 1)} dim={entry.save.deleted} />
                      <span className="tile-title">{entry.save.title}</span>
                      <span className="tile-blocks">{entry.save.linkedSlots.length} block{entry.save.linkedSlots.length === 1 ? "" : "s"}{entry.save.deleted ? " · Deleted" : ""}</span>
                    </button>
                  </div>
                ))}
              </div>
            </section>
          )) : !loading && (
            <div className="empty-board">
              <h2>{library?.directory ? "No saves found" : "Your local save library"}</h2>
              <p className="muted">{library?.directory
                ? "This directory has no readable .mcs saves. Open a card and use Sync to add its saves, or choose another folder."
                : "Choose the directory containing your .mcs saves. Saves in game folders appear here too. Sync will use this directory for future backups."}</p>
              {!isTauri() && <p className="muted">Choose a local directory in the desktop app.</p>}
            </div>
          )}
        </section>
        <aside className="inspector library-inspector">
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
