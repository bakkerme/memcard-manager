import { isTauri } from "@tauri-apps/api/core";
import type { LibraryView } from "../card";
import { IconFolder, IconRefresh, IconSettings } from "../icons";

export function Settings({ library, loading, busy, error, onChoose, onRefresh, onReveal }: {
  library: LibraryView | null;
  loading: boolean;
  busy: boolean;
  error: string | null;
  onChoose: () => void;
  onRefresh: () => void;
  onReveal: (path: string) => void;
}) {
  const configured = Boolean(library?.collectionConfigured && library.directory);
  const disabled = busy || loading || !isTauri();

  return <main className="workspace settings-workspace" aria-busy={loading}>
    <div className="collection-heading"><IconSettings /><h2>Settings</h2></div>
    {error && <div className="banner error" role="alert">{error}</div>}
    <div className="settings-body">
      <section className="settings-section" aria-labelledby="collection-folder-title">
        <h3 id="collection-folder-title">Collection folder</h3>
        <p className="muted">The home for your local saves, snapshots, and whole-card backups.</p>
        <div className="settings-location">
          <IconFolder />
          <div>
            <p className="settings-path">{configured ? library?.displayPath ?? library?.directory : "No collection folder selected"}</p>
            <p className="muted">{configured ? "Individual saves in saves/ · Card images in card-backups/" : "Choose a folder to start keeping your collection."}</p>
          </div>
        </div>
        <div className="settings-actions">
          <button type="button" className={`btn${configured ? "" : " btn-primary"}`} disabled={disabled}
            title={!isTauri() ? "Requires the desktop app" : undefined} onClick={onChoose}>
            <IconFolder />{configured ? "Change folder" : "Choose collection folder"}
          </button>
          <button type="button" className="btn" disabled={disabled || !configured}
            onClick={() => library?.directory && onReveal(library.directory)}><IconFolder />Reveal in Finder</button>
        </div>
        <p className="settings-help muted">Changing folders copies your existing collection; originals stay in place.</p>
      </section>
      <section className="settings-section" aria-labelledby="refresh-collection-title">
        <h3 id="refresh-collection-title">Refresh collection</h3>
        <p className="muted">Local saves and Card backups refresh when you open them. Refresh here to check for files changed outside the app.</p>
        <button type="button" className="btn" disabled={disabled} title={!isTauri() ? "Requires the desktop app" : undefined} onClick={onRefresh}>
          <IconRefresh />{loading ? "Refreshing…" : "Refresh collection"}
        </button>
        <p className="settings-help muted" role="status">{loading ? "Reading collection…" : `${library?.saves.length ?? 0} saves · ${library?.cards.length ?? 0} card backups`}</p>
      </section>
    </div>
  </main>;
}
