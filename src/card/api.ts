import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  CardError,
  bytesFromIpc,
  viewFromIpc,
  type CardView,
  type SaveInfo,
  type CardViewDto,
} from "./engine";

const OPEN_FILTERS = [
  {
    name: "PlayStation Memory Card",
    extensions: [
      "mcr",
      "mcd",
      "bin",
      "mc",
      "ps",
      "psm",
      "srm",
      "vm1",
      "vmc",
      "sav",
      "ddf",
      "mci",
      "gme",
      "mem",
      "vgs",
    ],
  },
];

export interface AdaptorIdentity {
  vid: number;
  pid: number;
  bcdDevice: string;
  manufacturer: string;
  product: string;
  bus: number;
  address: number;
}

export interface HardwareStatus {
  state: string;
  message: string;
  identity: AdaptorIdentity | null;
  frame: number;
  total: number;
}

export interface ExportResult {
  bytes: Uint8Array;
  filename: string;
  view: CardView | null;
}

export interface BackupResult {
  path: string;
  displayPath: string;
  filename: string;
}

export interface SyncResult {
  path: string;
  displayPath: string;
  written: number;
  unchanged: number;
  snapshotsAdded: number;
}

export async function syncCard(directory: string): Promise<SyncResult> {
  try {
    return await invoke<SyncResult>("sync_card", { directory });
  } catch (err) {
    throw asError(err);
  }
}

export interface SnapshotSource {
  imageId: string;
  sourceName: string;
  source: string;
}

export interface LibrarySnapshot {
  path: string;
  contentId: string;
  capturedAt: string | null;
  sources: SnapshotSource[];
  current: boolean;
  save: SaveInfo;
}

export interface LibrarySave {
  path: string;
  relativePath: string;
  save: SaveInfo;
  snapshots: LibrarySnapshot[];
}

export interface LibraryView {
  directory: string | null;
  displayPath: string | null;
  saves: LibrarySave[];
  warnings: string[];
  collectionConfigured: boolean;
  cards: CardBackup[];
}

export type CardColor = "grey" | "black" | "white" | "blue" | "green" | "red";

export interface CardBackup {
  path: string;
  filename: string;
  name: string;
  color: CardColor;
  capturedAt: string | null;
  sourceName: string | null;
  source: "file" | "usb" | null;
  imageId: string;
  saveCount: number;
  usedBlocks: number;
}

type SaveDto = CardViewDto["saves"][number];
type SnapshotDto = Omit<LibrarySnapshot, "save"> & { save: SaveDto };
type LibraryDto = Omit<LibraryView, "saves"> & {
  saves: (Omit<LibrarySave, "save" | "snapshots"> & { save: SaveDto; snapshots: SnapshotDto[] })[];
};

function saveFromIpc(save: SaveDto): SaveInfo {
  return { ...save, frames: save.frames.map(frame => new Uint8ClampedArray(frame)) };
}

function libraryFromIpc(dto: LibraryDto): LibraryView {
  return { ...dto, saves: dto.saves.map(entry => ({
    ...entry,
    save: saveFromIpc(entry.save),
    snapshots: entry.snapshots.map(snapshot => ({ ...snapshot, save: saveFromIpc(snapshot.save) })),
  })) };
}

export async function readLocalBackups(): Promise<LibraryView> {
  try {
    return libraryFromIpc(await invoke<LibraryDto>("local_backups"));
  } catch (err) { throw asError(err); }
}

export async function chooseLocalBackups(defaultPath?: string | null): Promise<LibraryView | null> {
  try {
    const directory = await open({ directory: true, multiple: false,
      title: "Choose collection folder for saves and card backups", defaultPath: defaultPath ?? undefined });
    if (!directory || Array.isArray(directory)) return null;
    return libraryFromIpc(await invoke<LibraryDto>("configure_local_backups", { directory }));
  } catch (err) { throw asError(err); }
}

function asError(err: unknown): CardError {
  if (err instanceof CardError) return err;
  if (typeof err === "string") return new CardError(err);
  if (err instanceof Error) return new CardError(err.message);
  return new CardError("Unexpected error.");
}

export async function openCardBytes(bytes: Uint8Array, name: string): Promise<CardView> {
  try {
    const dto = await invoke<CardViewDto>("open_card", { bytes: Array.from(bytes), name });
    return viewFromIpc(dto);
  } catch (err) {
    throw asError(err);
  }
}

export async function openCardPath(path: string): Promise<CardView> {
  try {
    const dto = await invoke<CardViewDto>("open_path", { path });
    return viewFromIpc(dto);
  } catch (err) {
    throw asError(err);
  }
}

export async function pickAndOpenCard(): Promise<CardView | null> {
  const path = await open({ multiple: false, filters: OPEN_FILTERS });
  if (!path || Array.isArray(path)) return null;
  return openCardPath(path);
}

export async function activateCard(sessionId: string): Promise<void> {
  try { await invoke("activate_card", { sessionId }); }
  catch (err) { throw asError(err); }
}

export async function closeCard(sessionId: string, nextSessionId: string | null): Promise<void> {
  try { await invoke("close_card", { sessionId, nextSessionId }); }
  catch (err) { throw asError(err); }
}

export async function composeCard(masterSlots: number[]): Promise<ExportResult> {
  try {
    const result = await invoke<{ bytes: number[]; filename: string; view: CardViewDto | null }>(
      "compose_card",
      { masterSlots },
    );
    return {
      bytes: bytesFromIpc(result.bytes),
      filename: result.filename,
      view: result.view ? viewFromIpc(result.view) : null,
    };
  } catch (err) {
    throw asError(err);
  }
}

export async function backupCard(directory: string, name: string, color: CardColor): Promise<BackupResult> {
  try {
    return await invoke<BackupResult>("backup_card", { directory, name, color });
  } catch (err) {
    throw asError(err);
  }
}

export async function labelCardBackup(path: string, name: string, color: CardColor): Promise<LibraryView> {
  try {
    return libraryFromIpc(await invoke<LibraryDto>("label_card_backup", { path, name, color }));
  } catch (err) { throw asError(err); }
}

export async function revealPath(path: string): Promise<void> {
  try {
    await invoke("reveal_path", { path });
  } catch (err) {
    throw asError(err);
  }
}

export async function saveExport(result: ExportResult): Promise<boolean> {
  const ext = result.filename.split(".").pop() ?? "mcr";
  const path = await save({
    defaultPath: result.filename,
    filters: [{ name: "Memory Card", extensions: [ext] }],
  });
  if (!path) return false;
  await invoke("write_bytes", { path, bytes: Array.from(result.bytes) });
  return true;
}

export async function probeAdaptor(): Promise<HardwareStatus> {
  try {
    return await invoke<HardwareStatus>("probe_adaptor");
  } catch (err) {
    throw asError(err);
  }
}

export async function readAdaptor(): Promise<CardView> {
  try {
    const dto = await invoke<CardViewDto>("read_adaptor");
    return viewFromIpc(dto);
  } catch (err) {
    throw asError(err);
  }
}

export async function onUsbProgress(handler: (status: HardwareStatus) => void): Promise<UnlistenFn> {
  if (!isTauri()) return () => undefined;
  try {
    return await listen<HardwareStatus>("usb-progress", (event) => handler(event.payload));
  } catch {
    return () => undefined;
  }
}
