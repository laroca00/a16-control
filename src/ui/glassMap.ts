// Bakes a refraction map for the glass surfaces. Inside a band along the
// rounded edge, each pixel points back toward the interior, strongest at the
// rim and fading out with depth; the SVG displacement filter then pulls the
// blurred backdrop inward there, which reads as a curved glass lip.

export type DisplacementMap = {
  href: string;
  scale: number;
};

// Band width as a share of the shorter side, and the displacement in px per
// unit of strength at the rim, as a share of the band.
const BAND = 0.18;
const PULL = 0.6;

/** Signed distance to a w×h rectangle with corner radius r (negative inside). */
function roundedRectDistance(x: number, y: number, w: number, h: number, r: number): number {
  const qx = Math.abs(x - w / 2) - (w / 2 - r);
  const qy = Math.abs(y - h / 2) - (h / 2 - r);
  const outside = Math.hypot(Math.max(qx, 0), Math.max(qy, 0));
  const inside = Math.min(Math.max(qx, qy), 0);
  return outside + inside - r;
}

/** Inward displacement at (x, y), each component in -1..1; zero beyond the edge band. */
export function displacementAt(x: number, y: number, w: number, h: number, r: number): [number, number] {
  const radius = Math.min(r, w / 2, h / 2);
  const band = Math.min(w, h) * BAND;
  const depth = -roundedRectDistance(x, y, w, h, radius);
  if (depth >= band) return [0, 0];

  const e = 0.5;
  const gx = roundedRectDistance(x + e, y, w, h, radius) - roundedRectDistance(x - e, y, w, h, radius);
  const gy = roundedRectDistance(x, y + e, w, h, radius) - roundedRectDistance(x, y - e, w, h, radius);
  const len = Math.hypot(gx, gy);
  if (len === 0) return [0, 0];

  const falloff = 1 - Math.max(depth, 0) / band;
  const magnitude = falloff * falloff;
  // The distance gradient points outward; the pull goes the other way.
  return [(-gx / len) * magnitude, (-gy / len) * magnitude];
}

export function createDisplacementMap(width: number, height: number, radius: number, strength = 1): DisplacementMap {
  const w = Math.max(1, Math.round(width));
  const h = Math.max(1, Math.round(height));
  const canvas = document.createElement("canvas");
  canvas.width = w;
  canvas.height = h;
  const ctx = canvas.getContext("2d")!;
  const image = ctx.createImageData(w, h);
  const px = image.data;

  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const [dx, dy] = displacementAt(x + 0.5, y + 0.5, w, h, radius);
      const i = (y * w + x) * 4;
      px[i] = 128 + Math.round(dx * 127);
      px[i + 1] = 128 + Math.round(dy * 127);
      px[i + 2] = 128;
      px[i + 3] = 255;
    }
  }
  ctx.putImageData(image, 0, 0);

  return {
    href: canvas.toDataURL("image/png"),
    scale: Math.min(w, h) * BAND * PULL * strength,
  };
}
