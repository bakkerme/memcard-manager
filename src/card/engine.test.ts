import { describe, expect, it } from "vitest";
import { viewFromIpc, type CardViewDto } from "./engine";

describe("viewFromIpc", () => {
  it("turns IPC number frames into RGBA clamped arrays", () => {
    const dto: CardViewDto = {
      sourceName: "blue",
      imageId: "a".repeat(64),
      format: "raw",
      source: "file",
      usedBlocks: 1,
      slots: [{ index: 0, type: "initial", next: null, xorOk: true }],
      saves: [
        {
          masterSlot: 0,
          linkedSlots: [0],
          title: "TEST",
          region: "America",
          regionRaw: "BA",
          prodCode: "SLUS-00000",
          identifier: "SAVE",
          sizeKb: 8,
          kind: "save",
          deleted: false,
          frameCount: 1,
          frames: [Array(16 * 16 * 4).fill(255)],
        },
      ],
    };
    const view = viewFromIpc(dto);
    expect(view.saves[0].frames[0]).toBeInstanceOf(Uint8ClampedArray);
    expect(view.saves[0].frames[0]).toHaveLength(1024);
    expect(view.saves[0].frames[0][0]).toBe(255);
  });
});
