import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties, type MouseEvent } from "react";
import { isTauri } from "@tauri-apps/api/core";
import {
  CardError,
  SLOT_COUNT,
  activateCard,
  closeCard,
  backupCard,
  labelCardBackup,
  openCardPath,
  syncCard,
  readLocalBackups,
  chooseLocalBackups,
  composeCard,
  demoView,
  onUsbProgress,
  openCardBytes,
  pickAndOpenCard,
  probeAdaptor,
  readAdaptor,
  revealPath,
  saveExport,
  type BackupResult,
  type CardBackup,
  type CardColor,
  type SyncResult,
  type CardView,
  type LibraryView,
  type HardwareStatus,
  type SaveInfo,
  type SlotInfo,
} from "./card";
import { createAdaptorMonitor } from "./card/adaptorMonitor";
import { CardBackups } from "./components/CardBackups";
import { CardColorPicker, MemoryCardThumbnail } from "./components/MemoryCardThumbnail";
import { Settings } from "./components/Settings";
import { LocalBackups } from "./components/LocalBackups";
import { PixelIcon } from "./components/PixelIcon";
import { GameDetailsButton } from "./components/GameDetails";
import cardThumb from "./assets/plates/card-thumb.png";
import {
  IconArchive,
  IconBackup,
  IconCard,
  IconCloud,
  IconClose,
  IconCopy,
  IconFolder,
  IconImport,
  IconLink,
  IconPencil,
  IconPlayStation,
  IconPlus,
  IconRefresh,
  IconStack,
  IconSettings,
  IconTrash,
} from "./icons";
import "./App.css";

function pad2(n: number): string {
  return String(n).padStart(2, "0");
}

function slotLabel(index: number): string {
  return `Slot ${index + 1}`;
}

function adaptorPresent(hw: HardwareStatus | null): boolean {
  return hw?.state === "adaptor" || hw?.state === "reading" || hw?.state === "live";
}

function laterTitle(feature: string): string {
  return `${feature} is not available yet`;
}

type WorkspacePage = "card" | "saves" | "backups" | "settings";

interface OpenCard {
  id: string;
  view: CardView;
  backup: CardBackup | null;
}

export default function App() {
  const hasOverlayTitlebar = isTauri() && /Mac/.test(navigator.platform);
  const [view, setView] = useState<CardView | null>(null);
  const [openCards, setOpenCards] = useState<OpenCard[]>([]);
  const [activeCardId, setActiveCardId] = useState<string | null>(null);
  const [autoReadPending, setAutoReadPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<number[]>([]);
  const [focus, setFocus] = useState<number | null>(null);
  const [frameTick, setFrameTick] = useState(0);
  const [dragging, setDragging] = useState(false);
  const [busy, setBusy] = useState(false);
  const [hw, setHw] = useState<HardwareStatus | null>(null);
  const [notice, setNotice] = useState<BackupResult | null>(null);
  const [syncNotice, setSyncNotice] = useState<SyncResult | null>(null);
  const [syncing, setSyncing] = useState(false);
  const [page, setPage] = useState<WorkspacePage>("card");
  const libraryOpen = page === "saves";
  const cardsOpen = page === "backups";
  const settingsOpen = page === "settings";
  const collectionOpen = page !== "card";
  const [backupEditing, setBackupEditing] = useState(false);
  const [backupName, setBackupName] = useState("");
  const [backupColor, setBackupColor] = useState<CardColor>("grey");
  const [activeBackup, setActiveBackup] = useState<CardBackup | null>(null);
  const [library, setLibrary] = useState<LibraryView | null>(null);
  const [libraryLoading, setLibraryLoading] = useState(false);
  const [libraryError, setLibraryError] = useState<string | null>(null);
  const libraryRequest = useRef(0);
  const monitorRef = useRef<ReturnType<typeof createAdaptorMonitor> | null>(null);
  const readingRef = useRef(false);
  const fileRef = useRef<HTMLInputElement>(null);

  const applyView = useCallback((next: CardView, backup: CardBackup | null = null) => {
    const id = next.sessionId ?? `${next.source}:${next.imageId}`;
    setOpenCards(current => {
      const entry = { id, view: next, backup };
      return current.some(card => card.id === id)
        ? current.map(card => card.id === id ? entry : card)
        : [...current, entry];
    });
    setActiveCardId(id);
    setView(next);
    setPage("card");
    setBackupEditing(false);
    setActiveBackup(backup);
    setSelected([]);
    setFocus(null);
    setError(null);
    setNotice(null);
    setSyncNotice(null);
  }, []);

  const refreshLibrary = useCallback(async () => {
    if (!isTauri()) return;
    const request = ++libraryRequest.current;
    setLibraryLoading(true);
    setLibraryError(null);
    try {
      const next = await readLocalBackups();
      if (request === libraryRequest.current) setLibrary(next);
    } catch (err) {
      if (request === libraryRequest.current) {
        setLibrary(current => current ? { ...current, saves: [], cards: [], warnings: [] } : null);
        setLibraryError(err instanceof CardError ? err.message : "Could not read local backups. Choose a folder again.");
      }
    } finally {
      if (request === libraryRequest.current) setLibraryLoading(false);
    }
  }, []);

  useEffect(() => { void refreshLibrary(); }, [refreshLibrary]);

  function openSettings() {
    setPage("settings");
    setBackupEditing(false);
  }

  async function configureLibrary() {
    if (busy || libraryLoading) return;
    const request = ++libraryRequest.current;
    setLibraryLoading(true);
    setLibraryError(null);
    try {
      const next = await chooseLocalBackups(library?.directory);
      if (next && request === libraryRequest.current) setLibrary(next);
    } catch (err) {
      if (request === libraryRequest.current) setLibraryError(err instanceof CardError ? err.message : "Could not configure local backups.");
    } finally {
      if (request === libraryRequest.current) setLibraryLoading(false);
    }
  }

  async function revealLibrary(path: string) {
    try { await revealPath(path); }
    catch (err) { setLibraryError(err instanceof CardError ? err.message : "Could not reveal the backup."); }
  }

  const readUsb = useCallback(async () => {
    if (readingRef.current) return;
    readingRef.current = true;
    setPage("card");
    setBackupEditing(false);
    setError(null);
    setBusy(true);
    setHw((current) => ({
      state: "reading", message: "Reading Slot 1", identity: current?.identity ?? null,
      frame: 0, total: 1024,
    }));
    try {
      await monitorRef.current?.pause();
      applyView(await readAdaptor());
      setHw((current) => ({
        state: "live", message: "Slot 1 read successfully", identity: current?.identity ?? null,
        frame: 1024, total: 1024,
      }));
    } catch (err) {
      const message = err instanceof CardError ? err.message : "Could not read the adaptor.";
      setError(`${message} Check the card and adaptor, then reload Slot 1 in the sidebar.`);
      setHw((current) => ({
        state: "adaptor", message, identity: current?.identity ?? null, frame: 0, total: 1024,
      }));
    } finally {
      readingRef.current = false;
      setBusy(false);
      monitorRef.current?.resume();
    }
  }, [applyView]);

  useEffect(() => {
    const id = window.setInterval(() => setFrameTick((n) => n + 1), 500);
    return () => window.clearInterval(id);
  }, []);

  useEffect(() => {
    if (!import.meta.env.DEV) return;
    const params = new URLSearchParams(window.location.search);
    if (params.get("demo") === "1") applyView(demoView());
    const qa = params.get("qa");
    if (qa === "searching") {
      setHw({
        state: "searching",
        message: "Looking for the PS3 memory card adaptor",
        identity: null,
        frame: 0,
        total: 1024,
      });
    } else if (qa === "reading") {
      setBusy(true);
      setHw({
        state: "reading",
        message: "Reading Slot 1",
        identity: null,
        frame: 420,
        total: 1024,
      });
    } else if (qa === "error") {
      setError("Could not read the adaptor.");
      setHw({
        state: "error",
        message: "USB stalled on frame 12",
        identity: null,
        frame: 12,
        total: 1024,
      });
    } else if (qa === "library" || qa === "library-large" || qa === "card-backups") {
      const demo = demoView();
      setLibrary({ directory: "/tmp/memcard-demo", displayPath: "~/Documents/memcard-viewer",
        collectionConfigured: true,
        cards: (["grey", "black", "white", "blue", "green", "red"] as CardColor[]).map((color, index) => ({
          path: `/tmp/memcard-demo/card-backups/${color}.mcr`, filename: `${color}-2026-10-04.mcr`,
          name: ["Original card", "RPG collection", "Platformers", "MiSTer card", "Second playthrough", "Arcade favorites"][index],
          color, capturedAt: "2026-10-04T06:00:00Z", sourceName: "Demo card (synthetic)", source: "file",
          imageId: demo.imageId, saveCount: demo.saves.filter(save => !save.deleted).length, usedBlocks: demo.usedBlocks,
        })),
        saves: Array.from({ length: qa === "library-large" ? 6 : 1 }, (_, batch) => demo.saves.map((original, offset) => {
          const index = batch * demo.saves.length + offset;
          const save = batch ? { ...original, title: `${original.title} · Backup ${batch + 1}` } : original;
          return { save,
          path: `/tmp/memcard-demo/${index}.mcs`,
          relativePath: `${save.prodCode || "unknown-game"}/${index}.mcs`,
          snapshots: [0, 1].map(version => ({
            path: `/tmp/memcard-demo/.snapshots/${index}/${version}.mcs`, contentId: String(version).repeat(64),
            capturedAt: version === 0 ? "2026-09-30T06:00:00Z" : "2026-10-01T06:00:00Z",
            sources: [{ imageId: demo.imageId, sourceName: "Demo card (synthetic)", source: "file" }],
            current: version === 1, save,
          })),
        }; })).flat(), warnings: [] });
      setPage(qa === "card-backups" ? "backups" : "saves");
      setHw({ state: "idle", message: "Preview", identity: null, frame: 0, total: 1024 });
    } else if (qa === "backup-editor") {
      applyView(demoView());
      setBackupEditing(true);
      setBackupName("Blue card");
      setBackupColor("blue");
      setHw({ state: "idle", message: "Preview", identity: null, frame: 0, total: 1024 });
    } else if (qa === "backup") {
      applyView(demoView());
      setHw({
        state: "idle",
        message: "Adaptor probe needs the desktop app",
        identity: null,
        frame: 0,
        total: 1024,
      });
      setNotice({
        path: "/tmp/demo.mcr",
        displayPath: "Documents/memcard-viewer/backups/demo.mcr",
        filename: "demo.mcr",
      });
    }
  }, [applyView]);

  useEffect(() => {
    if (import.meta.env.DEV && new URLSearchParams(window.location.search).has("qa")) return;
    if (!isTauri()) {
      setHw({ state: "idle", message: "Adaptor detection needs the desktop app", identity: null, frame: 0, total: 1024 });
      return;
    }
    let disposed = false;
    let unlisten: (() => void) | undefined;
    const monitor = createAdaptorMonitor(probeAdaptor, setHw, () => undefined, () => {
      setAutoReadPending(true);
    });
    monitorRef.current = monitor;
    void onUsbProgress((status) => {
      if (!disposed && readingRef.current) setHw(status);
    }).then((fn) => {
      if (disposed) fn();
      else {
        unlisten = fn;
        void monitor.check();
      }
    }).catch(() => undefined);
    return () => {
      disposed = true;
      monitor.stop();
      monitorRef.current = null;
      unlisten?.();
    };
  }, [readUsb]);

  useEffect(() => {
    if (!autoReadPending || busy || libraryLoading || readingRef.current || !adaptorPresent(hw)) return;
    setAutoReadPending(false);
    void readUsb();
  }, [autoReadPending, busy, libraryLoading, hw, readUsb]);

  const saveByMaster = useMemo(() => {
    const map = new Map<number, SaveInfo>();
    if (view) for (const save of view.saves) map.set(save.masterSlot, save);
    return map;
  }, [view]);

  const saveBySlot = useMemo(() => {
    const map = new Map<number, SaveInfo>();
    if (view) {
      for (const save of view.saves) {
        for (const slot of save.linkedSlots) map.set(slot, save);
      }
    }
    return map;
  }, [view]);

  const focusedSave = focus != null ? (saveByMaster.get(focus) ?? saveBySlot.get(focus) ?? null) : null;
  const canCompose = selected.length > 0 && !busy && !collectionOpen;
  const connected = adaptorPresent(hw);
  const reading = hw?.state === "reading";
  const readPercent = Math.min(100, Math.floor(((hw?.frame ?? 0) / (hw?.total || 1024)) * 100));
  const virtualCards = openCards.filter(card => card.view.source === "file");
  const physicalCard = openCards.find(card => card.view.source === "usb");
  const usbOpen = view?.source === "usb";

  const loadBytes = useCallback(
    async (file: File) => {
      if (readingRef.current || busy) return;
      setError(null);
      setBusy(true);
      try {
        applyView(await openCardBytes(new Uint8Array(await file.arrayBuffer()), file.name));
      } catch (err) {
        setError(
          !isTauri()
            ? "Opening a card needs the desktop app. Run npm run tauri."
            : err instanceof CardError
              ? err.message
              : "Could not open that file.",
        );
      } finally {
        setBusy(false);
      }
    },
    [applyView, busy],
  );

  function onFiles(files: FileList | null) {
    const file = files?.[0];
    if (file) void loadBytes(file);
  }

  async function openNative() {
    if (readingRef.current) return;
    setError(null);
    if (!isTauri()) {
      fileRef.current?.click();
      return;
    }
    setBusy(true);
    try {
      const opened = await pickAndOpenCard();
      if (opened) applyView(opened);
    } catch (err) {
      setError(err instanceof CardError ? err.message : "Could not open that file.");
    } finally {
      setBusy(false);
    }
  }

  function toggleSelect(master: number, event: MouseEvent) {
    const extend = event.metaKey || event.ctrlKey;
    setFocus(master);
    setSelected((prev) => {
      if (extend) {
        return prev.includes(master) ? prev.filter((s) => s !== master) : [...prev, master].sort((a, b) => a - b);
      }
      if (prev.length === 1 && prev[0] === master) return [];
      return [master];
    });
  }

  async function compose() {
    if (!view || selected.length === 0) return;
    setBusy(true);
    try {
      const dest = await composeCard(selected);
      if (await saveExport(dest)) applyView(await openCardBytes(dest.bytes, dest.filename));
    } catch (err) {
      setError(err instanceof CardError ? err.message : "Compose failed.");
    } finally {
      setBusy(false);
    }
  }

  async function ensureCollection(): Promise<string | null> {
    if (library?.collectionConfigured && library.directory) return library.directory;
    // Resolve the persisted folder before offering a picker, including when
    // the initial collection refresh has not finished yet.
    const current = await readLocalBackups();
    setLibrary(current);
    if (current.collectionConfigured && current.directory) return current.directory;
    const next = await chooseLocalBackups(current.directory);
    if (next) setLibrary(next);
    return next?.directory ?? null;
  }

  function beginBackup() {
    if (!view) return;
    setBackupName(activeBackup?.name ?? (view.source === "usb" ? "Memory Card" : view.sourceName.replace(/\.[^/.]+$/, "")).slice(0, 80));
    setBackupColor(activeBackup?.color ?? "grey");
    setBackupEditing(true);
  }

  async function backup() {
    if (!view || busy) return;
    setBusy(true);
    setError(null);
    setNotice(null);
    setSyncNotice(null);
    try {
      const directory = await ensureCollection();
      if (!directory) return;
      setNotice(await backupCard(directory, backupName, backupColor));
      setBackupEditing(false);
      await refreshLibrary();
    } catch (err) {
      setError(err instanceof CardError ? err.message : "Backup failed.");
    } finally { setBusy(false); }
  }

  async function openBackup(card: CardBackup) {
    if (busy) return;
    const existing = openCards.find(open => open.backup?.path === card.path);
    if (existing) { await selectCard(existing); return; }
    setBusy(true);
    setLibraryError(null);
    try {
      applyView(await openCardPath(card.path), card);
    } catch (err) { setLibraryError(err instanceof CardError ? err.message : "Could not open that card backup. Refresh and try again."); }
    finally { setBusy(false); }
  }

  async function selectCard(card: OpenCard) {
    if (busy || readingRef.current) return;
    setBusy(true);
    try {
      if (card.view.sessionId) await activateCard(card.view.sessionId);
      applyView(card.view, card.backup);
    } catch (err) {
      const message = err instanceof CardError ? err.message : "Could not switch cards. Try opening the card again.";
      setError(message);
      setLibraryError(message);
    } finally { setBusy(false); }
  }

  async function closeVirtualCard(card: OpenCard) {
    if (busy || readingRef.current) return;
    const remaining = openCards.filter(open => open.id !== card.id);
    const next = card.id === activeCardId
      ? remaining.find(open => open.view.source === "file") ?? (connected ? physicalCard : undefined)
      : undefined;
    setBusy(true);
    try {
      if (card.view.sessionId) await closeCard(card.view.sessionId, next?.view.sessionId ?? null);
      setOpenCards(remaining);
      if (card.id === activeCardId) {
        if (next) {
          applyView(next.view, next.backup);
          setPage(page);
        }
        else {
          setView(null);
          setActiveCardId(null);
          setActiveBackup(null);
          setSelected([]);
          setFocus(null);
          setBackupEditing(false);
          setNotice(null);
          setSyncNotice(null);
          setError(null);
        }
      }
    } catch (err) {
      const message = err instanceof CardError ? err.message : "Could not close that card. Try again.";
      setError(message);
      setLibraryError(message);
    } finally { setBusy(false); }
  }

  async function saveCardLabel(path: string, name: string, color: CardColor) {
    if (busy) return;
    setBusy(true);
    setLibraryError(null);
    try {
      const next = await labelCardBackup(path, name, color);
      setLibrary(next);
      const label = next.cards.find(card => card.path === path) ?? null;
      if (activeBackup?.path === path) setActiveBackup(label);
      setOpenCards(current => current.map(card => card.backup?.path === path ? { ...card, backup: label } : card));
    } catch (err) { setLibraryError(err instanceof CardError ? err.message : "Could not save the card label. Try again."); }
    finally { setBusy(false); }
  }

  async function sync() {
    if (!view || busy) return;
    setBusy(true);
    setSyncing(true);
    setError(null);
    setNotice(null);
    setSyncNotice(null);
    try {
      const directory = await ensureCollection();
      if (!directory) return;
      const result = await syncCard(directory);
      setSyncNotice(result);
      if (result) await refreshLibrary();
    } catch (err) { setError(err instanceof CardError ? err.message : "Sync failed. Try Sync again."); }
    finally { setSyncing(false); setBusy(false); }
  }

  async function revealSync() {
    if (!syncNotice) return;
    try {
      await revealPath(syncNotice.path);
    } catch (err) {
      setError(err instanceof CardError ? err.message : "Could not reveal the sync directory.");
    }
  }

  async function revealBackup() {
    if (!notice) return;
    try {
      await revealPath(notice.path);
    } catch (err) {
      setError(err instanceof CardError ? err.message : "Could not reveal the backup in Finder.");
    }
  }


  return (
    <div
      className={`app${hasOverlayTitlebar ? " app-macos" : ""}`}
      onDragOver={(e) => {
        e.preventDefault();
        setDragging(true);
      }}
      onDragLeave={() => setDragging(false)}
      onDrop={(e) => {
        e.preventDefault();
        setDragging(false);
        onFiles(e.dataTransfer.files);
      }}
    >
      {hasOverlayTitlebar && <div className="window-drag-region" data-tauri-drag-region aria-hidden="true" />}
      <input
        ref={fileRef}
        className="ghost-file"
        type="file"
        accept=".mcr,.mcd,.bin,.mc,.ps,.psm,.srm,.vm1,.vmc,.sav,.ddf,.mci,.gme,.mem,.vgs"
        onChange={(e) => {
          onFiles(e.target.files);
          e.target.value = "";
        }}
      />

      <header className="topbar">
        <div className="brand">
          <IconPlayStation />
          <h1>memcard-viewer</h1>
        </div>
        <div className="topbar-actions">
          <button type="button" className="btn" disabled title={laterTitle("Import save")}><IconImport />Import save</button>
          <button type="button" className="btn" disabled={!view || busy || collectionOpen || libraryLoading || !isTauri()}
            title={isTauri() ? "Back up the whole card to your collection" : "Backup requires the desktop app"} onClick={beginBackup}>
            <IconBackup />Backup
          </button>
          <button type="button" className="btn" disabled={!view || busy || collectionOpen || libraryLoading || !isTauri()}
            title={isTauri() ? "Export active saves by game to your collection. Card is read-only." : "Sync requires the desktop app"}
            onClick={() => void sync()}><IconRefresh />{syncing ? "Syncing…" : "Sync"}</button>
        </div>
      </header>

      <aside className="sidebar">
        <div className="sidebar-sources">
        <section className="nav-section">
          <div className="nav-label">
            <span className="nav-label-name">
              <IconCard />
              Physical Cards
            </span>
            <span className="nav-count">{connected ? 1 : 0}</span>
          </div>
          {connected ? (
            <div className={`nav-item ${usbOpen && !collectionOpen ? "selected" : ""}`}>
              <IconCard />
              <button type="button" className="nav-copy" disabled={busy} onClick={() => physicalCard ? void selectCard(physicalCard) : void readUsb()} style={navReset}>
                <strong>Slot 1</strong>
                <span>
                  {reading ? "Reading…" : physicalCard
                    ? `${physicalCard.view.usedBlocks} blocks used`
                    : "Click to retry read"}
                </span>
              </button>
              <div className="nav-aside">
                <button
                  type="button"
                  className="refresh"
                  disabled={busy}
                  aria-label="Reload Slot 1"
                  onClick={(e) => {
                    e.stopPropagation();
                    void readUsb();
                  }}
                >
                  <IconRefresh />
                </button>
              </div>
            </div>
          ) : (
            <div className="nav-item">
              <IconCard />
              <span className="nav-copy">
                <strong>{hw?.state === "error" ? "Adaptor unavailable" : "No adaptor found"}</strong>
                <span>{isTauri() ? "Connect your PS3 adaptor" : "Requires the desktop app"}</span>
              </span>
            </div>
          )}
        </section>

        <section className="nav-section">
          <div className="nav-label">
            <span className="nav-label-name">
              <IconStack />
              Virtual Cards
            </span>
            <span className="nav-count">{virtualCards.length}</span>
          </div>
          {virtualCards.map(card => {
            const name = card.backup?.name ?? card.view.sourceName;
            return <div key={card.id} className={`nav-item ${card.id === activeCardId && !collectionOpen ? "selected" : ""}`}>
              <IconCard />
              <button type="button" className="nav-copy" style={navReset} disabled={busy}
                aria-current={card.id === activeCardId && !collectionOpen ? "page" : undefined}
                onClick={() => void selectCard(card)}>
                <strong>{name}</strong>
                <span>{card.view.usedBlocks} blocks used</span>
              </button>
              <button type="button" className="nav-close" disabled={busy} aria-label={`Close ${name}`}
                title={`Close ${name}`} onClick={() => void closeVirtualCard(card)}><IconClose /></button>
            </div>;
          })}
          <button type="button" className="nav-item nav-add" disabled={busy} onClick={() => void openNative()}>
            <IconFolder /><span className="nav-copy"><strong>Open card…</strong></span>
          </button>
          <button type="button" className="nav-item nav-add" disabled={!canCompose || !isTauri()} onClick={() => void compose()}
            title={canCompose ? "Compose a new .mcr from the selected saves" : "Select saves to compose a new card"}>
            <IconPlus /><span className="nav-copy"><strong>New card from selection</strong></span>
          </button>
        </section>


        <section className="nav-section">
          <div className="nav-label">
            <span className="nav-label-name">
              <IconCloud />
              All Saves
            </span>
            <span className="nav-count">{library?.saves.length ?? 0}</span>
          </div>
          <button type="button" className={`nav-item ${libraryOpen ? "selected" : ""}`} disabled={busy}
            onClick={() => { setPage("saves"); void refreshLibrary(); }}>
            <IconArchive />
            <span className="nav-copy">
              <strong>Local saves</strong>
              <span>{library?.directory ? `${library.saves.length} saves` : "Choose a folder"}</span>
            </span>
          </button>
          <button type="button" className="nav-item" disabled title={laterTitle("Cloud")}>
            <IconCloud />
            <span className="nav-copy">
              <strong>Cloud</strong>
              <span>Not connected</span>
            </span>
          </button>
        </section>

        <section className="nav-section">
          <button type="button" className={`nav-item ${cardsOpen ? "selected" : ""}`} disabled={busy}
            onClick={() => { setPage("backups"); void refreshLibrary(); }}>
            <IconCard /><span className="nav-copy"><strong>Card backups</strong><span>{library?.cards.length ?? 0} whole-card captures</span></span>
          </button>
        </section>

        <section className="nav-section">
          <button type="button" className="nav-item" disabled title={laterTitle("Auto-backup")}>
            <IconRefresh />
            <span className="nav-copy">
              <strong>Auto-backup</strong>
              <span>Not available yet</span>
            </span>
          </button>
        </section>
        </div>
        <section className="nav-section nav-settings">
          <button type="button" className={`nav-item ${settingsOpen ? "selected" : ""}`} disabled={busy}
            aria-current={settingsOpen ? "page" : undefined} onClick={openSettings}>
            <IconSettings /><span className="nav-copy"><strong>Settings</strong></span>
          </button>
        </section>
      </aside>

      {settingsOpen ? <Settings library={library} loading={libraryLoading} busy={busy} error={libraryError}
        onChoose={() => void configureLibrary()} onRefresh={() => void refreshLibrary()} onReveal={path => void revealLibrary(path)} /> : cardsOpen ? <CardBackups library={library} loading={libraryLoading} busy={busy} error={libraryError}
        onSettings={openSettings} onReveal={path => void revealLibrary(path)}
        onOpen={card => void openBackup(card)} onLabel={(path, name, color) => void saveCardLabel(path, name, color)} /> : libraryOpen ? <LocalBackups library={library} loading={libraryLoading} busy={busy}
        error={libraryError} frameTick={frameTick} onSettings={openSettings}
        onReveal={path => void revealLibrary(path)} /> : <main className={`workspace ${dragging ? "drop-active" : ""}`}>
        <div className="card-head">
          {activeBackup ? <MemoryCardThumbnail color={activeBackup.color} /> : <img className="card-thumb" src={cardThumb} alt="" width={64} height={72} />}
          <div className="card-id">
            <h2>{view ? (activeBackup?.name ?? (usbOpen ? "Memory Card" : view.sourceName)) : reading ? "Memory Card" : "No card open"}</h2>
            <p>
              {usbOpen
                ? "Slot 1"
                : view
                  ? view.format.toUpperCase()
                  : reading
                    ? "Reading Slot 1…"
                    : connected
                    ? "Adaptor ready"
                    : "Open a card or read Slot 1"}
            </p>
          </div>
          <div className="occupancy">
            <p className="occupancy-label">
              {view ? `${view.usedBlocks} / ${SLOT_COUNT} blocks used` : reading ? "Checking card contents…" : `0 / ${SLOT_COUNT} blocks used`}
            </p>
            <div className="occupancy-track" aria-hidden>
              {Array.from({ length: SLOT_COUNT }, (_, i) => {
                const slot = view?.slots[i];
                const current = focusedSave?.linkedSlots.includes(i) ?? false;
                const used = slot ? slot.type !== "formatted" : false;
                return (
                  <span key={i} className={`occ-cell ${current ? "current" : used ? "used" : ""}`}>
                    <i />
                    <b>{pad2(i + 1)}</b>
                  </span>
                );
              })}
            </div>
          </div>
          <div className="card-stats">
            {view ? `${view.saves.length} saves · 128 KB` : reading ? "128 KB" : "0 saves · 128 KB"}
          </div>
        </div>



        {error && (
          <div className="banner error" role="alert">
            {error}
          </div>
        )}

        {notice && (
          <div className="banner" role="status">
            <span>Backed up to {notice.displayPath}</span>
            <button type="button" onClick={() => void revealBackup()}>
              Reveal in Finder
            </button>
            <button type="button" onClick={() => { setPage("backups"); void refreshLibrary(); }}>View card backups</button>
          </div>
        )}

        {syncNotice && (
          <div className="banner" role="status">
            <span>Synced {syncNotice.written} save{syncNotice.written === 1 ? "" : "s"} · {syncNotice.unchanged} unchanged · {syncNotice.snapshotsAdded} new snapshot{syncNotice.snapshotsAdded === 1 ? "" : "s"} to {syncNotice.displayPath}</span>
            <button type="button" onClick={() => void revealSync()}>Reveal in Finder</button>
          </div>
        )}

        {backupEditing && <form className="backup-create" onSubmit={event => { event.preventDefault(); void backup(); }}>
          <MemoryCardThumbnail color={backupColor} />
          <div className="backup-create-fields">
            <h3>Back up this card</h3>
            <label htmlFor="new-backup-name">Card name</label>
            <input id="new-backup-name" value={backupName} onChange={event => setBackupName(event.target.value)} maxLength={80} required disabled={busy} autoFocus />
            <CardColorPicker value={backupColor} onChange={setBackupColor} disabled={busy} />
            <p className="muted">Choose a thumbnail color for this backup. The entire card is preserved, including deleted saves.</p>
            <p className="muted">{library?.collectionConfigured ? `Saved automatically in ${library.displayPath}/card-backups/` : libraryLoading ? "Checking your collection folder…" : "Choose your collection folder once; card backups will use its card-backups/ subfolder automatically."}</p>
          </div>
          <div className="backup-create-actions">
            <button className="btn btn-primary" disabled={busy || !backupName.trim() || !isTauri()} title={!isTauri() ? "Backup requires the desktop app" : undefined}>{busy ? "Backing up…" : "Back up card"}</button>
            <button className="btn" type="button" disabled={busy} onClick={() => setBackupEditing(false)}>Cancel</button>
          </div>
        </form>}
        <div className="workspace-body">
          <section className="board" aria-label="Memory card gallery">
            {view ? (
              view.slots.map((slot) => {
                const save = saveBySlot.get(slot.index) ?? null;
                const master = save?.masterSlot ?? slot.index;
                const chain = focusedSave?.linkedSlots.includes(slot.index) ?? false;
                return (
                  <SlotTile
                    key={slot.index}
                    slot={slot}
                    save={save}
                    selected={selected.includes(master)}
                    chain={chain}
                    frameTick={frameTick}
                    onSelect={toggleSelect}
                  />
                );
              })
            ) : reading ? null : (
              <div className="empty-board">
                <h2>
                  {connected ? "Read Slot 1 to see this card" : "Open a PlayStation 1 memory card"}
                </h2>
                <p className="muted">
                  Open a card from Virtual Cards in the sidebar, or drop a .mcr, .gme, .vgs, or .mem file here. Connecting a PS3 adaptor starts reading Slot 1 automatically.
                </p>

              </div>
            )}
          </section>
          <Inspector save={focusedSave} slot={focus != null && view ? view.slots[focus] : null} frameTick={frameTick} />
        </div>
      </main>}

      <footer className="footer">
        <p className="footer-tip">{settingsOpen ? "Collection settings" : collectionOpen ? "Local collection · Card stays read-only" : "Drop a card file anywhere to open it"}</p>
        {!collectionOpen && <div className="footer-keys">
          <span className="kbd live">
            <b>Ctrl</b>+<b>Click</b> Select multiple
          </span>
          <span className="kbd hint" title={laterTitle("Range select")}>
            <b>Shift</b> Range select
          </span>
          <span className="kbd hint" title={laterTitle("Delete")}>
            <b>Del</b> Delete
          </span>
        </div>}
      </footer>
      {reading && <ReadProgressDialog percent={readPercent} />}
    </div>
  );
}

function ReadProgressDialog({ percent }: { percent: number }) {
  const dialogRef = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    const keepOpen = (event: Event) => event.preventDefault();
    const keepEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") event.preventDefault();
    };
    dialog.addEventListener("cancel", keepOpen);
    dialog.addEventListener("keydown", keepEscape);
    dialog.showModal();
    return () => {
      dialog.removeEventListener("cancel", keepOpen);
      dialog.removeEventListener("keydown", keepEscape);
      dialog.close();
    };
  }, []);

  return (
    <dialog ref={dialogRef} className="read-dialog"
      aria-label="Reading memory card">
      <div className="read-dialog-bar" role="progressbar" aria-label="Reading Slot 1"
        tabIndex={-1} autoFocus
        aria-valuemin={0} aria-valuemax={100} aria-valuenow={percent}>
        <span style={{ transform: `scaleX(${percent / 100})` }} />
      </div>
    </dialog>
  );
}

const navReset: CSSProperties = {
  appearance: "none",
  border: 0,
  background: "transparent",
  padding: 0,
  textAlign: "left",
  color: "inherit",
  minWidth: 0,
  display: "flex",
  flexDirection: "column",
  gap: 2,
  cursor: "pointer",
};

function SlotTile({
  slot,
  save,
  selected,
  chain,
  frameTick,
  onSelect,
}: {
  slot: SlotInfo;
  save: SaveInfo | null;
  selected: boolean;
  chain: boolean;
  frameTick: number;
  onSelect: (master: number, event: MouseEvent) => void;
}) {
  const isLink = Boolean(save && save.masterSlot !== slot.index);
  const isEmpty = slot.type === "formatted";
  const isCorrupt = slot.type === "corrupted";
  const master = save?.masterSlot ?? slot.index;
  const clickable = Boolean(save) || isLink;
  const linkIndex = save ? save.linkedSlots.indexOf(slot.index) : -1;
  const linkCount = save?.linkedSlots.length ?? 0;
  const frameIndex = save && save.frameCount > 0 ? frameTick % save.frameCount : 0;

  let title = slotLabel(slot.index);
  let blocks = "empty";
  if (isCorrupt) {
    title = "Corrupt";
    blocks = "";
  } else if (isLink && save) {
    title = save.title;
    blocks = `${linkIndex + 1} of ${linkCount}`;
  } else if (save) {
    title = save.title;
    blocks = `${linkCount} block${linkCount === 1 ? "" : "s"}`;
  }

  return (
    <div
      className={[
        "tile",
        selected ? "selected" : "",
        chain && !selected ? "chain" : "",
        isEmpty ? "empty" : "",
        isLink ? "continuation" : "",
        save?.deleted ? "ghost" : "",
        isCorrupt ? "corrupt" : "",
        clickable ? "" : "empty",
      ].join(" ")}
    >
      <button
        type="button"
        className="tile-hit"
        disabled={!clickable}
        aria-pressed={clickable ? selected : undefined}
        aria-label={isLink ? `${pad2(slot.index + 1)} ${title}, ${blocks}` : undefined}
        title={save?.title}
        onClick={(event) => clickable && onSelect(master, event)}
      >
        <span className="tile-index">{pad2(slot.index + 1)}</span>
        {save && save.frames.length > 0 ? (
          <PixelIcon frames={save.frames} frameIndex={frameIndex} dim={save.deleted} />
        ) : (
          <div className="pixel-placeholder">{isCorrupt ? "!" : ""}</div>
        )}
        <span className="tile-title">{isEmpty ? "Empty" : title}</span>
        {!isEmpty && blocks && (
          <span className="tile-blocks">
            {isLink ? <IconLink /> : null}
            {blocks}
          </span>
        )}
      </button>
    </div>
  );
}

function Inspector({
  save,
  slot,
  frameTick,
}: {
  save: SaveInfo | null;
  slot: SlotInfo | null;
  frameTick: number;
}) {
  if (!save) {
    return (
      <aside className="inspector">
        <h3>Inspector</h3>
        <p className="muted">Select a save to inspect title, region, product code, and linked blocks.</p>
        {slot?.type === "formatted" && <p className="muted">{slotLabel(slot.index)} is empty.</p>}
        {slot?.type === "corrupted" && <p className="muted">{slotLabel(slot.index)} is corrupt.</p>}
      </aside>
    );
  }

  const frameIndex = save.frameCount > 0 ? frameTick % save.frameCount : 0;

  return (
    <aside className="inspector">
      <div className="inspector-hero">
        <PixelIcon frames={save.frames} frameIndex={frameIndex} dim={save.deleted} />
        <div className="inspector-title">
          <h2>{save.deleted ? "Deleted save" : save.kind === "software" ? "PocketStation" : save.title}</h2>
          <button type="button" className="icon-btn" disabled title={laterTitle("Rename")}>
            <IconPencil />
          </button>
        </div>
        <dl>
          <div>
            <dt>Title</dt>
            <dd>{save.title}</dd>
          </div>
          <div>
            <dt>Region</dt>
            <dd>
              {save.region}
              {save.regionRaw ? ` (${save.regionRaw})` : ""}
            </dd>
          </div>
          <div>
            <dt>Product</dt>
            <dd>{save.prodCode || "—"}</dd>
          </div>
          <div>
            <dt>Identifier</dt>
            <dd>{save.identifier || "—"}</dd>
          </div>
          {slot && <div>
            <dt>Directory XOR</dt>
            <dd>{slot.xorOk ? "OK" : "Checksum mismatch"}</dd>
          </div>}
        </dl>
      </div>
      <div className="inspector-chain">
        <h3>Blocks (linked)</h3>
        <div className="chips">
          {save.linkedSlots.map((s, i) => (
            <span key={s} style={{ display: "contents" }}>
              {i > 0 ? <span className="chip-arrow">→</span> : null}
              <span className="chip-block">
                {pad2(s + 1)}
                <span className="dot" style={{ width: 6, height: 6, boxShadow: "none", background: "var(--accent)" }} />
              </span>
            </span>
          ))}
        </div>
        <p className="inspector-size">
          {save.linkedSlots.length} block{save.linkedSlots.length === 1 ? "" : "s"} · {save.sizeKb} KB
        </p>
      </div>
      <GameDetailsButton save={save} />
      <div className="inspector-actions">
        <button type="button" className="btn" disabled title={laterTitle("Export")}>
          <IconBackup />
          Export .mcs
        </button>
        <button type="button" className="btn" disabled title={laterTitle("Duplicate")}>
          <IconCopy />
          Duplicate
        </button>
        <button type="button" className="btn btn-danger" disabled title={laterTitle("Delete")}>
          <IconTrash />
          Delete save
        </button>
      </div>
    </aside>
  );
}
