/** Zoom around a plot-relative pointer position, preserving its timestamp. */
export function zoomTimeline(
  seconds: number,
  right: number,
  fraction: number,
  delta: number,
) {
  const anchor = Math.max(0, Math.min(1, fraction));
  const next = Math.max(
    0.001,
    Math.min(
      60,
      seconds * Math.exp(Math.max(-1000, Math.min(1000, delta)) * 0.002),
    ),
  );
  const time = right - seconds + anchor * seconds;
  return { seconds: next, right: Math.max(next, time + (1 - anchor) * next) };
}
