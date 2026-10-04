/** Read-only data decoded by the Rust backend. Only original US DW2 saves are supported. */
export interface DigimonWorld2Details {
  game: "digimon-world-2";
  checksumOk: boolean;
  profiles: DigimonWorld2Profile[];
}
export type GameDetails = DigimonWorld2Details;

export interface DigimonWorld2Profile {
  number: number;
  empty: boolean;
  tamer: string;
  rank: string;
  location: string;
  bits: number;
  beetle: string;
  /** Hours/minutes shown by the game, truncated to minutes and capped at 99:59. */
  playtimeMinutes: number | null;
  digimon: DigimonWorld2Digimon[];
}

export function formatPlaytime(minutes: number): string {
  const hours = Math.floor(minutes / 60);
  return `${String(hours).padStart(2, "0")}:${String(minutes % 60).padStart(2, "0")}`;
}
export interface DigimonWorld2Digimon {
  rosterSlot: number;
  status: number;
  species: string;
  name: string;
  level: number;
  maxLevel: number;
  dp: number;
  experience: number;
  hp: number;
  maxHp: number;
  mp: number;
  maxMp: number;
  attack: number;
  defense: number;
  speed: number;
  techniques: string[];
  inheritedTechniques: string[];
}
