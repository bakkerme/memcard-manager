import type { GameDetails } from "./gameDetails";

export const SLOT_COUNT = 15;
export const CARD_SIZE = 131072;
export const HEADER_SIZE = 128;
export const BLOCK_SIZE = 8192;
export const MCS_HEADER_SIZE = 128;

export type SlotType =
  | "formatted"
  | "initial"
  | "middle_link"
  | "end_link"
  | "deleted_initial"
  | "deleted_middle_link"
  | "deleted_end_link"
  | "corrupted";

export type DataKind = "save" | "software";
export type CardFormat = "raw" | "gme" | "vgs";
export type CardSource = "file" | "usb";
export type RgbaFrame = Uint8ClampedArray;

export interface SlotInfo {
  index: number;
  type: SlotType;
  next: number | null;
  xorOk: boolean;
}

export interface SaveInfo {
  masterSlot: number;
  linkedSlots: number[];
  title: string;
  region: string;
  regionRaw: string;
  prodCode: string;
  identifier: string;
  sizeKb: number;
  kind: DataKind;
  deleted: boolean;
  frameCount: number;
  frames: RgbaFrame[];
  gameDetails?: GameDetails | null;
  gameDetailsError?: string | null;
}

export interface CardView {
  sessionId?: string;
  sourceName: string;
  imageId: string;
  format: CardFormat;
  source: CardSource;
  slots: SlotInfo[];
  saves: SaveInfo[];
  usedBlocks: number;
}

export class CardError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "CardError";
  }
}

interface SaveDto extends Omit<SaveInfo, "frames"> {
  frames: number[][];
}

export interface CardViewDto extends Omit<CardView, "saves"> {
  saves: SaveDto[];
}

export function viewFromIpc(dto: CardViewDto): CardView {
  return {
    ...dto,
    saves: dto.saves.map((save) => ({
      ...save,
      frames: save.frames.map((frame) => new Uint8ClampedArray(frame)),
    })),
  };
}

export function bytesFromIpc(bytes: number[] | Uint8Array): Uint8Array {
  return bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
}
