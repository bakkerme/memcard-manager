import cardThumb from "../assets/plates/card-thumb.png";
import type { CardColor } from "../card";

// Shell colors depict the card label; selection and actions keep the application's blue.
const finishes = {
  grey: { label: "Grey", filter: "none" },
  black: { label: "Black", filter: "brightness(0.42) contrast(1.14)" },
  white: { label: "White", filter: "brightness(1.32) saturate(0.2)" },
  blue: { label: "Blue", filter: "sepia(0.9) saturate(3) hue-rotate(165deg) brightness(0.85)" },
  green: { label: "Green", filter: "sepia(0.8) saturate(1.7) hue-rotate(95deg) brightness(0.9)" },
  red: { label: "Red", filter: "sepia(1) saturate(2.8) hue-rotate(300deg) brightness(0.8)" },
} satisfies Record<CardColor, { label: string; filter: string }>;

export function MemoryCardThumbnail({ color, small = false }: { color: CardColor; small?: boolean }) {
  // Reuse the incumbent rendered plate: color treatment keeps its molded edges and material.
  return <img className={`memory-card-thumbnail ${small ? "small" : ""}`} src={cardThumb}
    style={{ filter: finishes[color].filter }} alt="" width={72} height={88} />;
}

export function CardColorPicker({ value, onChange, disabled = false }: { value: CardColor; onChange: (color: CardColor) => void; disabled?: boolean }) {
  return <fieldset className="card-color-picker" disabled={disabled}>
    <legend>Card color</legend>
    <div className="card-color-options">
      {(Object.keys(finishes) as CardColor[]).map(color => <button key={color} type="button"
        className={`card-color-option ${value === color ? "selected" : ""}`} aria-pressed={value === color}
        onClick={() => onChange(color)}>
        <MemoryCardThumbnail color={color} small /><span>{finishes[color].label}</span>
      </button>)}
    </div>
  </fieldset>;
}
