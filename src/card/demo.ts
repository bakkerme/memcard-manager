import { SLOT_COUNT, type CardView, type SaveInfo, type SlotInfo, type SlotType } from "./engine";

/** Dev-only synthetic frames for `?demo=1`. Not a real memory card. */
function paint(draw: (set: (x: number, y: number, rgb: [number, number, number]) => void) => void): Uint8ClampedArray {
  const data = new Uint8ClampedArray(16 * 16 * 4);
  for (let i = 0; i < 256; i++) {
    const p = i * 4;
    data[p] = 12;
    data[p + 1] = 14;
    data[p + 2] = 22;
    data[p + 3] = 255;
  }
  const set = (x: number, y: number, rgb: [number, number, number]) => {
    if (x < 0 || y < 0 || x > 15 || y > 15) return;
    const p = (y * 16 + x) * 4;
    data[p] = rgb[0];
    data[p + 1] = rgb[1];
    data[p + 2] = rgb[2];
  };
  draw(set);
  return data;
}

function rect(
  set: (x: number, y: number, rgb: [number, number, number]) => void,
  x: number,
  y: number,
  w: number,
  h: number,
  rgb: [number, number, number],
) {
  for (let yy = y; yy < y + h; yy++) for (let xx = x; xx < x + w; xx++) set(xx, yy, rgb);
}

const ICONS: Record<string, () => Uint8ClampedArray> = {
  pac: () =>
    paint((set) => {
      rect(set, 3, 3, 10, 10, [248, 208, 32]);
      rect(set, 5, 5, 3, 3, [24, 24, 32]);
      rect(set, 8, 7, 5, 3, [12, 14, 22]);
    }),
  kula: () =>
    paint((set) => {
      rect(set, 4, 2, 8, 6, [96, 176, 232]);
      rect(set, 5, 8, 6, 6, [232, 120, 168]);
      set(6, 4, [20, 20, 28]);
      set(9, 4, [20, 20, 28]);
    }),
  ape: () =>
    paint((set) => {
      rect(set, 3, 5, 10, 8, [200, 96, 56]);
      rect(set, 5, 2, 6, 5, [232, 168, 96]);
      set(6, 4, [32, 20, 16]);
      set(9, 4, [32, 20, 16]);
    }),
  blank: () =>
    paint((set) => {
      rect(set, 2, 2, 12, 12, [72, 140, 48]);
      rect(set, 6, 6, 4, 4, [248, 232, 64]);
    }),
  crash: () =>
    paint((set) => {
      rect(set, 4, 4, 8, 8, [232, 120, 40]);
      rect(set, 5, 2, 6, 3, [40, 32, 28]);
      set(6, 6, [16, 12, 12]);
      set(9, 6, [16, 12, 12]);
    }),
  klonoa: () =>
    paint((set) => {
      rect(set, 3, 4, 10, 9, [48, 96, 216]);
      rect(set, 5, 2, 6, 4, [248, 248, 252]);
      set(6, 5, [16, 16, 24]);
      set(9, 5, [16, 16, 24]);
    }),
  ctr: () =>
    paint((set) => {
      rect(set, 2, 8, 12, 5, [176, 48, 40]);
      rect(set, 4, 5, 8, 4, [48, 48, 56]);
      rect(set, 6, 3, 4, 3, [232, 200, 64]);
    }),
  digimon: () =>
    paint((set) => {
      rect(set, 4, 3, 8, 10, [56, 88, 184]);
      rect(set, 6, 1, 4, 3, [248, 220, 72]);
      set(6, 6, [248, 248, 252]);
      set(9, 6, [248, 248, 252]);
    }),
  bomb: () =>
    paint((set) => {
      rect(set, 4, 5, 8, 8, [248, 248, 252]);
      rect(set, 6, 7, 4, 4, [32, 32, 40]);
      rect(set, 10, 3, 2, 3, [232, 80, 64]);
    }),
  gt: () =>
    paint((set) => {
      rect(set, 2, 8, 12, 5, [200, 40, 48]);
      rect(set, 4, 6, 8, 3, [24, 24, 28]);
      rect(set, 3, 10, 3, 2, [16, 16, 20]);
      rect(set, 10, 10, 3, 2, [16, 16, 20]);
    }),
  sui: () =>
    paint((set) => {
      rect(set, 5, 2, 6, 12, [176, 136, 56]);
      rect(set, 3, 5, 10, 3, [120, 40, 40]);
    }),
  worms: () =>
    paint((set) => {
      rect(set, 4, 6, 8, 7, [168, 176, 72]);
      rect(set, 6, 3, 4, 4, [232, 220, 160]);
      set(7, 5, [32, 24, 16]);
      set(9, 5, [32, 24, 16]);
      rect(set, 10, 8, 3, 2, [72, 72, 80]);
    }),
  mega: () =>
    paint((set) => {
      rect(set, 4, 3, 8, 10, [48, 120, 208]);
      rect(set, 6, 6, 4, 3, [232, 48, 48]);
      set(6, 5, [248, 248, 252]);
      set(9, 5, [248, 248, 252]);
    }),
};

function save(
  master: number,
  linked: number[],
  title: string,
  prod: string,
  icon: keyof typeof ICONS,
): SaveInfo {
  return {
    masterSlot: master,
    linkedSlots: linked,
    title,
    region: "America",
    regionRaw: "BA",
    prodCode: prod,
    identifier: title.replace(/\s+/g, "").slice(0, 8).toLowerCase(),
    sizeKb: linked.length * 8,
    kind: "save",
    deleted: false,
    frameCount: 1,
    frames: [ICONS[icon]()],
  };
}

export function demoView(): CardView {
  const saves = [
    save(0, [0], "PAC-MAN WORLD", "SLUS-00564", "pac"),
    save(1, [1], "Kula World", "SLUS-00699", "kula"),
    save(2, [2], "Ape Escape", "SCUS-94423", "ape"),
    save(3, [3], "Point Blank 2", "SCUS-01057", "blank"),
    save(4, [4], "Crash Bandicoot 3", "SCUS-94426", "crash"),
    save(5, [5], "Klonoa", "SLUS-00617", "klonoa"),
    save(6, [6], "CTR Saved Games", "SLUS-94425", "ctr"),
    save(7, [7], "Digimon World 2", "SCUS-01167", "digimon"),
    save(8, [8], "Bomberman", "SLUS-00092", "bomb"),
    save(9, [9], "Gran Turismo", "SCUS-94194", "gt"),
    save(10, [10], "Suikoden", "SLUS-00292", "sui"),
    save(11, [11, 12, 14], "Worms World Party", "SLUS-01448", "worms"),
    save(13, [13], "Mega Man Legends", "SLUS-00603", "mega"),
  ];
  const bySlot = new Map<number, SaveInfo>();
  for (const item of saves) for (const slot of item.linkedSlots) bySlot.set(slot, item);

  const slots: SlotInfo[] = Array.from({ length: SLOT_COUNT }, (_, index) => {
    const item = bySlot.get(index);
    let type: SlotType = "formatted";
    let next: number | null = null;
    if (item) {
      const pos = item.linkedSlots.indexOf(index);
      if (pos === 0) type = "initial";
      else if (pos === item.linkedSlots.length - 1) type = "end_link";
      else type = "middle_link";
      next = item.linkedSlots[pos + 1] ?? null;
    }
    return { index, type, next, xorOk: true };
  });

  return {
    sourceName: "demo (synthetic)",
    imageId: "0123456789abcdef".repeat(4),
    format: "raw",
    source: "file",
    slots,
    saves,
    usedBlocks: saves.reduce((sum, item) => sum + item.linkedSlots.length, 0),
  };
}
