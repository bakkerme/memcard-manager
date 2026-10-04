import { useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import type { CardBackup, CardColor, LibraryView } from "../card";
import { IconCard, IconFolder, IconSettings } from "../icons";
import { CardColorPicker, MemoryCardThumbnail } from "./MemoryCardThumbnail";

export function CardBackups({ library, loading, busy, error, onSettings, onReveal, onOpen, onLabel }: {
  library: LibraryView | null;
  loading: boolean;
  busy: boolean;
  error: string | null;
  onSettings: () => void;
  onReveal: (path: string) => void;
  onOpen: (card: CardBackup) => void;
  onLabel: (path: string, name: string, color: CardColor) => void;
}) {
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [color, setColor] = useState<CardColor>("grey");
  const selected = library?.cards.find(card => card.path === selectedPath) ?? null;
  const disabled = busy || loading || !isTauri();
  const changed = selected && (name.trim() !== selected.name || color !== selected.color);
  return <main className="workspace library-workspace" aria-busy={loading}>
    <div className="collection-heading"><IconCard /><h2>Card backups</h2></div>
    {error && <div className="banner error" role="alert">{error}</div>}
    {!!library?.warnings.length && <div className="banner" role="status"><details><summary>{library.warnings.length} file or folder warning{library.warnings.length === 1 ? "" : "s"}</summary><ul>{library.warnings.map((warning, i) => <li key={i}>{warning}</li>)}</ul></details></div>}
    <div className="workspace-body library-body">
      <section className="library-saves" aria-label="Whole-card backups">
        <p className="library-count" role="status">{loading ? "Reading card backups…" : `${library?.cards.length ?? 0} card backups`}</p>
        {library?.cards.length ? <div className="library-grid card-backup-grid">
          {library.cards.map(card => <div key={card.path} className={`tile library-tile card-backup-tile ${selectedPath === card.path ? "selected" : ""}`}>
            <button type="button" className="tile-hit" aria-pressed={selectedPath === card.path} onClick={() => { setSelectedPath(card.path); setName(card.name); setColor(card.color); }}>
              <MemoryCardThumbnail color={card.color} />
              <span className="tile-title">{card.name}</span>
              <span className="tile-blocks">{card.saveCount} saves · {card.usedBlocks} / 15 blocks</span>
              <span className="card-backup-date">{card.capturedAt ? new Date(card.capturedAt).toLocaleString() : "Capture time unknown"}</span>
            </button>
          </div>)}
        </div> : !loading && <div className="empty-board"><h2>Your whole-card backups</h2>
          <p className="muted">{library?.collectionConfigured ? "Open a card or connect your adaptor, then use Backup to keep its entire 15-block image here." : "Choose a collection folder in Settings, then use Backup on an open card to keep its entire image here."}</p>
          {!library?.collectionConfigured && <div><button type="button" className="btn" disabled={busy} onClick={onSettings}><IconSettings />Open Settings</button></div>}
        </div>}
      </section>
      <aside className="inspector library-inspector">
        {selected ? <>
          <div className="card-backup-hero"><MemoryCardThumbnail color={color} /><h2>{selected.name}</h2></div>
          <dl className="card-backup-details">
            <div><dt>Contents</dt><dd>{selected.saveCount} active saves · {selected.usedBlocks} blocks used</dd></div>
            <div><dt>Captured</dt><dd>{selected.capturedAt ? new Date(selected.capturedAt).toLocaleString() : "Unknown"}</dd></div>
            <div><dt>Source</dt><dd>{selected.sourceName ?? "Original source unknown"}{selected.source === "usb" ? " · Adaptor read" : ""}</dd></div>
            <div><dt>File</dt><dd>{selected.filename}</dd></div>
          </dl>
          <form className="card-label-form" onSubmit={event => { event.preventDefault(); onLabel(selected.path, name, color); }}>
            <label htmlFor="backup-card-name">Card name</label>
            <input id="backup-card-name" value={name} onChange={event => setName(event.target.value)} required maxLength={80} disabled={disabled || !library?.collectionConfigured} />
            <CardColorPicker value={color} onChange={setColor} disabled={disabled || !library?.collectionConfigured} />
            <p className="muted">A visual label you choose. The adaptor cannot detect the physical shell color.</p>
            <button className="btn" disabled={disabled || !changed || !name.trim() || !library?.collectionConfigured}>{busy ? "Saving…" : "Save label"}</button>
          </form>
          <div className="card-backup-actions">
            <button className="btn btn-primary" disabled={disabled} onClick={() => onOpen(selected)}><IconCard />Open card</button>
            <button className="btn" disabled={disabled} onClick={() => onReveal(selected.path)}><IconFolder />Reveal in Finder</button>
          </div>
        </> : <><h3>Inspector</h3><p className="muted">Select a card backup to name it, choose its color, or open its fifteen blocks.</p></>}
      </aside>
    </div>
  </main>;
}
