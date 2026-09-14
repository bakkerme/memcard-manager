import { useCallback, useEffect, useMemo, useState, type MouseEvent } from "react";
import {
  CardError,
  SLOT_COUNT,
  backupCard,
  composeCard,
  onUsbProgress,
  openCardBytes,
  pickAndOpenCard,
  probeAdaptor,
  readAdaptor,
  saveExport,
  type CardView,
  type HardwareStatus,
  type SaveInfo,
  type SlotInfo,
} from "./card";
import { PixelIcon } from "./components/PixelIcon";
import "./App.css";

function slotLabel(index: number): string {
  return `Slot ${index + 1}`;
}

function hardwareLabel(hw: HardwareStatus | null, hasView: boolean, source?: string): string {
  if (hw?.state === "reading") return "reading";
  if (hw?.state === "live") return "live";
  if (hw?.state === "error") return "error";
  if (hasView && source === "usb") return "live";
  if (hasView) return "from disk";
  if (hw?.state === "adaptor") return "adaptor";
  if (hw?.state === "searching") return "searching";
  return "no source";
}

export default function App() {
  const [view, setView] = useState<CardView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<number[]>([]);
  const [focus, setFocus] = useState<number | null>(null);
  const [frameTick, setFrameTick] = useState(0);
  const [dragging, setDragging] = useState(false);
  const [busy, setBusy] = useState(false);
  const [hw, setHw] = useState<HardwareStatus | null>(null);

  useEffect(() => {
    const id = window.setInterval(() => setFrameTick((n) => n + 1), 500);
    return () => window.clearInterval(id);
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void onUsbProgress((status) => setHw(status)).then((fn) => {
      unlisten = fn;
    });
    void probeAdaptor()
      .then(setHw)
      .catch(() => {
        /* libusb may be missing in a browser preview */
      });
    return () => unlisten?.();
  }, []);

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

  const focusedSave = focus != null ? saveByMaster.get(focus) ?? null : null;
  const selectedBlocks = selected.reduce((sum, slot) => {
    return sum + (saveByMaster.get(slot)?.linkedSlots.length ?? 0);
  }, 0);

  const applyView = useCallback((next: CardView) => {
    setView(next);
    setSelected([]);
    setFocus(null);
    setError(null);
  }, []);

  const loadBytes = useCallback(
    async (file: File) => {
      setError(null);
      setBusy(true);
      try {
        applyView(await openCardBytes(new Uint8Array(await file.arrayBuffer()), file.name));
      } catch (err) {
        setView(null);
        setSelected([]);
        setFocus(null);
        setError(err instanceof CardError ? err.message : "Could not open that file.");
      } finally {
        setBusy(false);
      }
    },
    [applyView],
  );

  function onFiles(files: FileList | null) {
    const file = files?.[0];
    if (file) void loadBytes(file);
  }

  async function openNative() {
    setError(null);
    setBusy(true);
    try {
      const opened = await pickAndOpenCard();
      if (opened) applyView(opened);
    } catch (err) {
      setView(null);
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
      await saveExport(dest);
    } catch (err) {
      setError(err instanceof CardError ? err.message : "Compose failed.");
    } finally {
      setBusy(false);
    }
  }

  async function backup() {
    if (!view) return;
    setBusy(true);
    try {
      const dest = await backupCard();
      await saveExport(dest);
    } catch (err) {
      setError(err instanceof CardError ? err.message : "Backup failed.");
    } finally {
      setBusy(false);
    }
  }

  async function readUsb() {
    setError(null);
    setBusy(true);
    try {
      applyView(await readAdaptor());
    } catch (err) {
      setError(err instanceof CardError ? err.message : "Could not read the adaptor.");
      void probeAdaptor().then(setHw).catch(() => undefined);
    } finally {
      setBusy(false);
    }
  }

  const chip = hardwareLabel(hw, Boolean(view), view?.source);
  const reading = hw?.state === "reading";

  return (
    <div
      className="app"
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
      <header className="toolbar">
        <div className="brand">
          <h1>memcard-viewer</h1>
          <span className={`chip ${chip}`}>{chip}</span>
        </div>
        <div className="toolbar-actions">
          <button type="button" disabled={busy} onClick={() => void openNative()}>
            Open card
          </button>
          <button type="button" disabled={busy} onClick={() => void readUsb()}>
            Read adaptor
          </button>
          <button type="button" disabled={!view || busy} onClick={() => void backup()}>
            Backup
          </button>
          <button type="button" disabled={!view || selected.length === 0 || busy} onClick={() => void compose()}>
            Compose new card
          </button>
        </div>
      </header>

      {reading && (
        <div className="usage" aria-label="USB read progress">
          <span>{hw?.message}</span>
          <span>
            {hw?.frame ?? 0} / {hw?.total ?? 1024}
          </span>
          <div className="usage-track">
            <div
              className="usage-sel"
              style={{ width: `${((hw?.frame ?? 0) / (hw?.total || 1024)) * 100}%` }}
            />
          </div>
        </div>
      )}

      {view && !reading && (
        <div className="usage" aria-label="Card usage">
          <span>
            {view.sourceName} · {view.format} · {view.usedBlocks} of {SLOT_COUNT} blocks used
          </span>
          <span>
            {selected.length} selected · {selectedBlocks} blocks
          </span>
          <div className="usage-track">
            <div className="usage-sel" style={{ width: `${(selectedBlocks / SLOT_COUNT) * 100}%` }} />
            <div
              className="usage-other"
              style={{ width: `${((view.usedBlocks - selectedBlocks) / SLOT_COUNT) * 100}%` }}
            />
          </div>
        </div>
      )}

      {error && (
        <div className="banner error" role="alert">
          {error}
        </div>
      )}

      {!view && (
        <div className={`dropzone ${dragging ? "active" : ""}`}>
          <p>Drop a PlayStation 1 Memory Card, open one from disk, or read the PS3 adaptor.</p>
          <p className="muted">
            v1 reads raw .mcr, DexDrive .gme, and VGS .vgs/.mem. Hardware is read-only: dump and compose to files.
          </p>
          <div className="toolbar-actions">
            <button type="button" disabled={busy} onClick={() => void openNative()}>
              Choose file
            </button>
            <button type="button" disabled={busy} onClick={() => void readUsb()}>
              Read adaptor
            </button>
          </div>
        </div>
      )}

      {view && (
        <div className="workspace">
          <section className="board" aria-label="Memory card gallery">
            {view.slots.map((slot) => {
              const save = saveBySlot.get(slot.index) ?? null;
              const master = save?.masterSlot ?? slot.index;
              return (
                <SlotTile
                  key={slot.index}
                  slot={slot}
                  save={save}
                  selected={selected.includes(master)}
                  focused={focus === master}
                  frameTick={frameTick}
                  onSelect={toggleSelect}
                />
              );
            })}
          </section>
          <Inspector save={focusedSave} slot={focus != null ? view.slots[focus] : null} />
        </div>
      )}
    </div>
  );
}

function SlotTile({
  slot,
  save,
  selected,
  focused,
  frameTick,
  onSelect,
}: {
  slot: SlotInfo;
  save: SaveInfo | null;
  selected: boolean;
  focused: boolean;
  frameTick: number;
  onSelect: (master: number, event: MouseEvent) => void;
}) {
  const isLink = slot.type.includes("link") && !slot.type.includes("initial");
  const isEmpty = slot.type === "formatted";
  const isCorrupt = slot.type === "corrupted";
  const master = save?.masterSlot ?? slot.index;
  const clickable = Boolean(save) || isLink;

  let title = slotLabel(slot.index);
  let caption = "empty";
  if (isCorrupt) {
    title = "Corrupt";
    caption = slotLabel(slot.index);
  } else if (isLink && save) {
    title = "Linked";
    caption = `${save.linkedSlots.length} blocks`;
  } else if (save) {
    title = save.title;
    caption = `${save.prodCode || save.region} · ${save.linkedSlots.length} block${save.linkedSlots.length === 1 ? "" : "s"}`;
  }

  const frameIndex = save && save.frameCount > 0 ? frameTick % save.frameCount : 0;

  return (
    <button
      type="button"
      className={[
        "tile",
        selected ? "selected" : "",
        focused ? "focused" : "",
        isEmpty ? "empty" : "",
        save?.deleted ? "ghost" : "",
        isLink ? "link" : "",
        isCorrupt ? "corrupt" : "",
      ].join(" ")}
      disabled={!clickable}
      onClick={(event) => clickable && onSelect(master, event)}
    >
      {save && save.frames.length > 0 && !isLink ? (
        <PixelIcon frames={save.frames} frameIndex={frameIndex} dim={save.deleted} />
      ) : (
        <div className="pixel-placeholder">{isLink ? "link" : isCorrupt ? "!" : ""}</div>
      )}
      <span className="tile-title">{title}</span>
      <span className="tile-caption">{caption}</span>
    </button>
  );
}

function Inspector({ save, slot }: { save: SaveInfo | null; slot: SlotInfo | null }) {
  if (!save) {
    return (
      <aside className="inspector">
        <h2>Inspector</h2>
        <p className="muted">Select a save to inspect title, region, and block links.</p>
        {slot?.type === "formatted" && <p className="muted">{slotLabel(slot.index)} is empty.</p>}
        {slot?.type === "corrupted" && <p className="error-text">{slotLabel(slot.index)} is corrupt.</p>}
      </aside>
    );
  }

  return (
    <aside className="inspector">
      <h2>{save.deleted ? "Deleted save" : save.kind === "software" ? "PocketStation" : "Save"}</h2>
      <PixelIcon frames={save.frames} frameIndex={0} size={96} dim={save.deleted} />
      <dl>
        <div>
          <dt>Title</dt>
          <dd>{save.title}</dd>
        </div>
        <div>
          <dt>Region</dt>
          <dd>
            {save.region} ({save.regionRaw || "—"})
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
        <div>
          <dt>Blocks</dt>
          <dd>
            {save.linkedSlots.map((s) => s + 1).join(" → ")} ({save.sizeKb} KB)
          </dd>
        </div>
        <div>
          <dt>Icon frames</dt>
          <dd>{save.frameCount}</dd>
        </div>
        <div>
          <dt>XOR</dt>
          <dd>{slot?.xorOk ? "ok" : "mismatch"}</dd>
        </div>
      </dl>
    </aside>
  );
}
