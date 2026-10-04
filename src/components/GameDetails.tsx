import { useEffect, useId, useRef, useState } from "react";
import type { SaveInfo } from "../card";
import { formatPlaytime, type DigimonWorld2Digimon } from "../card/gameDetails";

const numbers = new Intl.NumberFormat();
function position(status: number): string {
  return ["None", "Server", "Bench", "Battle 1", "Battle 2", "Battle 3"][status]
    ?? `Unknown position (${status})`;
}

/** Shared by the card and backup inspectors; changing the inspected save closes the dialog. */
export function GameDetailsButton({ save }: { save: SaveInfo }) {
  const [open, setOpen] = useState(false);
  const [profileIndex, setProfileIndex] = useState(0);
  const dialog = useRef<HTMLDialogElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const titleId = useId();
  const descriptionId = useId();
  const details = save.gameDetails;
  const available = details?.game === "digimon-world-2" || !!save.gameDetailsError;
  const profile = details?.profiles[profileIndex];

  useEffect(() => {
    setOpen(false);
    setProfileIndex(Math.max(0, save.gameDetails?.profiles.findIndex(p => !p.empty) ?? 0));
  }, [save]);

  useEffect(() => {
    const element = dialog.current;
    if (!element) return;
    if (open && !element.open) element.showModal();
    else if (!open && element.open) element.close();
    return () => { if (element.open) element.close(); };
  }, [open]);

  function close() {
    setOpen(false);
    trigger.current?.focus();
  }

  if (!available) return null;
  return <section className="inspector-game-details">
    <button ref={trigger} type="button" className="btn" onClick={() => setOpen(true)} aria-haspopup="dialog">
      View game details
    </button>
    <p className="muted">Digimon World 2 · US</p>
    <dialog ref={dialog} className="game-dialog" aria-labelledby={titleId} aria-describedby={descriptionId}
      onCancel={event => { event.preventDefault(); close(); }} onClose={() => setOpen(false)}>
      <div className="game-dialog-heading">
        <div><h2 id={titleId}>Digimon World 2</h2><p id={descriptionId}>US save · Read-only game details{save.deleted ? " · Deleted save" : ""}</p></div>
        <button type="button" className="btn" autoFocus onClick={close}>Close</button>
      </div>
      {save.gameDetailsError ? <p className="game-details-warning" role="alert">{save.gameDetailsError}</p> : details && <>
        {!details.checksumOk && <p className="game-details-warning" role="status">The game checksum does not match. These values may be unreliable; try another backup of this save.</p>}
        <div className="game-profiles" role="group" aria-label="In-game saves">
          {details.profiles.map((p, index) => <button key={p.number} type="button" className="game-profile"
            aria-pressed={profileIndex === index} onClick={() => setProfileIndex(index)}>
            <span>Save {p.number}</span><strong>{p.empty ? "Empty" : p.tamer || "Unnamed tamer"}</strong>
          </button>)}
        </div>
        {!profile || profile.empty ? <div className="game-profile-empty"><h3>No in-game save</h3><p>This profile has no saved adventure.</p></div> : <>
          <dl className="game-summary">
            <div><dt>Tamer</dt><dd>{profile.tamer || "Unnamed tamer"}</dd></div>
            <div><dt>Rank</dt><dd>{profile.rank}</dd></div>
            <div><dt>Bits</dt><dd>{numbers.format(profile.bits)}</dd></div>
            <div><dt>Digi-Beetle</dt><dd>{profile.beetle || "Unnamed Digi-Beetle"}</dd></div>
            <div><dt>Playtime <span>(hh:mm)</span></dt><dd>{profile.playtimeMinutes == null ? "Unknown" : formatPlaytime(profile.playtimeMinutes)}</dd></div>
            <div className="game-location"><dt>Saved location</dt><dd>{profile.location}</dd></div>
          </dl>
          <section className="game-roster" aria-label="Digimon roster">
            <h3>Digimon <span>({profile.digimon.length})</span></h3>
            {profile.digimon.length ? profile.digimon.map(d => <DigimonRow key={`${profile.number}-${d.rosterSlot}`} digimon={d} />)
              : <p className="muted">No Digimon are recorded in this profile.</p>}
          </section>
        </>}
        <p className="game-details-footnote">Pilot decoder for the original US release. Inventory and story progress are not decoded yet.</p>
      </>}
    </dialog>
  </section>;
}

function DigimonRow({ digimon: d }: { digimon: DigimonWorld2Digimon }) {
  return <details className="game-digimon" open>
    <summary>
      <span className="game-digimon-name"><strong>{d.name || d.species}</strong><span>{d.species}</span></span>
      <span className="game-digimon-position">{position(d.status)}</span>
      <span className="game-digimon-level" title={`Level ${d.level}, maximum level ${d.maxLevel}`} aria-label={`Level ${d.level}, maximum level ${d.maxLevel}`}>Lv. {d.level}<span> / {d.maxLevel}</span></span>
    </summary>
    <dl className="game-digimon-stats">
      <div><dt>HP</dt><dd>{d.hp} / {d.maxHp}</dd></div>
      <div><dt>MP</dt><dd>{d.mp} / {d.maxMp}</dd></div>
      <div><dt>Attack</dt><dd>{d.attack}</dd></div>
      <div><dt>Defense</dt><dd>{d.defense}</dd></div>
      <div><dt>Speed</dt><dd>{d.speed}</dd></div>
      <div><dt>DP</dt><dd>{d.dp}</dd></div>
      <div><dt>Experience</dt><dd>{numbers.format(d.experience)}</dd></div>
    </dl>
    <div className="game-techniques"><span>Techniques</span><p>{d.techniques.join(" · ") || "None recorded"}</p></div>
    {d.inheritedTechniques.length > 0 && <div className="game-techniques"><span>Inherited</span><p>{d.inheritedTechniques.join(" · ")}</p></div>}
  </details>;
}
