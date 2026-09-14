import { useEffect, useRef } from "react";
import type { RgbaFrame } from "../card";

export function PixelIcon({
  frames,
  frameIndex,
  dim,
  size = 64,
}: {
  frames: RgbaFrame[];
  frameIndex: number;
  dim?: boolean;
  size?: number;
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const frame = frames[Math.min(frameIndex, Math.max(frames.length - 1, 0))];

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !frame) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, 16, 16);
    ctx.putImageData(new ImageData(new Uint8ClampedArray(frame), 16, 16), 0, 0);
  }, [frame]);

  return (
    <canvas
      ref={canvasRef}
      width={16}
      height={16}
      className={dim ? "pixel-icon dim" : "pixel-icon"}
      style={{ width: size, height: size }}
      aria-hidden
    />
  );
}
