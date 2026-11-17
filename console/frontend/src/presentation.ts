/** Readable byte quantities for recording metrics and analysis presentation. */
export const bytesLabel = (value: string | number | { toString(): string } | null | undefined) => {
  if (value == null) return 'Unavailable';
  const bytes = Number(value);
  if (!Number.isFinite(bytes) || bytes < 0) return 'Unavailable';
  const unit = bytes >= 1024 ** 3 ? 3 : bytes >= 1024 ** 2 ? 2 : bytes >= 1024 ? 1 : 0;
  return `${(bytes / 1024 ** unit).toFixed(unit ? 1 : 0)} ${['B', 'KiB', 'MiB', 'GiB'][unit]}`;
};
