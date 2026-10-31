import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { GameDetailsButton } from "./GameDetails";
import { demoView } from "../card/demo";
import type { GameSummary } from "../card/gameDetails";

function render(state: "occupied" | "empty" | "unknown", many = false, count = many ? 50 : 5) {
  const save = demoView().saves[0];
  const details: GameSummary = {
    game: "bomberman-fantasy-race", title: "Bomberman Fantasy Race", release: "NTSC-U · SLUS-00823",
    checksumOk: state !== "unknown", notes: [],
    profiles: Array.from({ length: count }, (_, index) => ({
      number: index + 1, name: "Blb", empty: state === "empty", status: state,
      checksumOk: state !== "unknown", records: [],
      // Prove unknown/empty rendering withholds details even if stale fields arrive.
      fields: [{ label: "Normal course tickets", value: "1 of 8" }],
    })),
  };
  return renderToStaticMarkup(<GameDetailsButton save={{ ...save, gameDetails: details, gameDetailsError: undefined }} />);
}

describe("Bomberman profile states", () => {
  it("shows unknown separately from empty and withholds stale progress", () => {
    const html = render("unknown");
    expect(html).toContain("Slot status unknown");
    expect(html).toContain("Name and course tickets are withheld.");
    expect(html).toContain(">Unknown</strong>");
    expect(html).not.toContain("No in-game save");
    expect(html).not.toContain("1 of 8");
  });
  it("keeps known empty and occupied states distinct", () => {
    expect(render("empty")).toContain("No in-game save");
    expect(render("empty")).not.toContain("1 of 8");
    expect(render("occupied")).toContain("Normal course tickets");
    expect(render("occupied")).toContain("1 of 8");
  });
  it("does not infer an empty adventure from zero declared slots", () => {
    const html = render("occupied", false, 0);
    expect(html).toContain("No profile details");
    expect(html).not.toContain("No in-game save");
  });
  it("keeps all fifty slots present in the bounded selector", () => {
    const html = render("occupied", true);
    expect(html).toContain('data-many="true"');
    expect(html).toContain("Save 50");
  });
});

describe("Metal Gear Solid summary", () => {
  const save = demoView().saves[0];
  const details: GameSummary = {
    game: "metal-gear-solid",
    title: "Metal Gear Solid",
    release: "NTSC-U · SLUS-00594",
    checksumOk: null,
    notes: ["The three bank copies do not agree, so there is no single checksum verdict."],
    profiles: [{
      number: 1,
      name: "MGS [EZ] 00:00 Dock",
      empty: false,
      checksumOk: null,
      fields: [
        { label: "Difficulty", value: "Easy" },
        { label: "Stage", value: "Cmnd room (s16d)" },
        { label: "Radar", value: "Hidden" },
        { label: "Max rations", value: "7" },
        { label: "Bank data", value: "Withheld (no single bank verdict)" },
      ],
      records: [{
        name: "SOCOM",
        fields: [
          { label: "Carried", value: "Yes" },
          { label: "Ammo", value: "122" },
          { label: "Capacity", value: "217" },
        ],
      }],
    }],
  };

  it("labels weapon records and an unresolved bank checksum", () => {
    const html = renderToStaticMarkup(
      <GameDetailsButton save={{ ...save, gameDetails: details, gameDetailsError: undefined }} />,
    );
    expect(html).toContain("Metal Gear Solid");
    expect(html).toContain("NTSC-U · SLUS-00594");
    expect(html).toContain("Game checksum");
    expect(html).toContain("Unresolved");
    expect(html).toContain("Difficulty");
    expect(html).toContain("Easy");
    expect(html).toContain("Cmnd room (s16d)");
    expect(html).toContain("Weapons and items <span>");
    expect(html).toContain("Radar");
    expect(html).toContain("Hidden");
    expect(html).toContain("Max rations");
    expect(html).toContain("SOCOM");
    expect(html).toContain("122");
    expect(html).not.toContain("Characters <span>");
    expect(html).toContain("no single checksum verdict");
  });

  it("warns when the shared bank CRC does not match", () => {
    const html = renderToStaticMarkup(
      <GameDetailsButton save={{
        ...save,
        gameDetails: { ...details, checksumOk: false, profiles: [{ ...details.profiles[0], records: [] }] },
        gameDetailsError: undefined,
      }} />,
    );
    expect(html).toContain("Does not match");
    expect(html).toContain("The game checksum does not match");
    expect(html).not.toContain("SOCOM");
  });
});
