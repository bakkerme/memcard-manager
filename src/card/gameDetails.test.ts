import { describe, expect, it } from "vitest";
import { formatPlaytime } from "./gameDetails";

describe("game playtime display", () => {
  it.each([
    [0, "00:00"], [1, "00:01"], [59, "00:59"], [60, "01:00"],
    [61, "01:01"], [121, "02:01"], [5999, "99:59"],
  ])("formats %i minutes as %s", (minutes, display) => {
    expect(formatPlaytime(minutes)).toBe(display);
  });
});
