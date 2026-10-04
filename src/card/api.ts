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

export async function syncCard(configuredDirectory?: string | null): Promise<SyncResult | null> {
  try {
    const directory = configuredDirectory ?? await open({ directory: true, multiple: false, title: "Sync saves to directory" });
    if (!directory || Array.isArray(directory)) return null;
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
      title: "Choose local backups directory", defaultPath: defaultPath ?? undefined });
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

export async function backupCard(): Promise<BackupResult> {
  try {
    return await invoke<BackupResult>("backup_card");
  } catch (err) {
    throw asError(err);
  }
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
