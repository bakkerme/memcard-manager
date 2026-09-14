import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  CardError,
  bytesFromIpc,
  viewFromIpc,
  type CardFormat,
  type CardView,
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

export async function backupCard(format?: CardFormat | "mcr"): Promise<ExportResult> {
  try {
    const result = await invoke<{ bytes: number[]; filename: string; view: CardViewDto | null }>(
      "backup_card",
      { format: format ?? null },
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

export function onUsbProgress(handler: (status: HardwareStatus) => void): Promise<UnlistenFn> {
  return listen<HardwareStatus>("usb-progress", (event) => handler(event.payload));
}
