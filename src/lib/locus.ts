/**
 * Locus arithmetic for the rsnap viewer: parsing, panning and zooming.
 *
 * Kept out of the component so the gesture maths can be exercised on its own -
 * a wrong anchor or an off-by-one span is invisible in a rendered image until
 * you are already lost in the genome.
 */

export interface Locus {
  chr: string;
  start: number;
  end: number;
}

/** Narrowest window worth rendering; below this reads have no room. */
export const MIN_SPAN = 40;

/** Wider than any chromosome, so a zoom-out cannot run away. */
export const MAX_SPAN = 250_000_000;

/** Parse "chr1:1,000-2,000" (or with an underscore) into coordinates. */
export function parseLocus(locusStr: string): Locus | null {
  const clean = locusStr.trim().replace(/,/g, '');
  const match = clean.match(/^([^:]+):(\d+)[-_](\d+)$/);
  if (!match) return null;
  return { chr: match[1], start: parseInt(match[2], 10), end: parseInt(match[3], 10) };
}

/** Format coordinates back, clamped to a sane, positive window. */
export function formatLocus(chr: string, start: number, end: number): string {
  const safeStart = Math.max(1, Math.round(start));
  const safeEnd = Math.max(safeStart + MIN_SPAN, Math.round(end));
  return `${chr}:${safeStart}-${safeEnd}`;
}

/** Move the window sideways by a fraction of its own span. */
export function pannedLocus(locus: Locus, fraction: number): string {
  const span = locus.end - locus.start;
  const shift = Math.round(span * fraction);
  return formatLocus(locus.chr, locus.start + shift, locus.end + shift);
}

/**
 * Scale the window around a point in it.
 *
 * `anchor` is where the gesture happened, 0 at the left edge and 1 at the
 * right; the base under that point stays put, so zooming goes where you point
 * rather than always to the middle. `factor` below 1 zooms in.
 */
export function zoomedLocus(locus: Locus, factor: number, anchor = 0.5): string {
  const span = locus.end - locus.start;
  const newSpan = Math.min(Math.max(span * factor, MIN_SPAN), MAX_SPAN);
  const clampedAnchor = Math.min(Math.max(anchor, 0), 1);
  const anchorBase = locus.start + span * clampedAnchor;
  return formatLocus(
    locus.chr,
    anchorBase - newSpan * clampedAnchor,
    anchorBase + newSpan * (1 - clampedAnchor),
  );
}
