/**
 * A size in bytes, said the way a person would say it.
 *
 * Rounded to one decimal above a megabyte and to none below it: "340 KB" and
 * "1.2 GB" are what somebody wants to know about disk space, and "347,891
 * bytes" is a number they then have to do arithmetic on.
 */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "0 bytes";
  if (bytes < 1024) return `${Math.round(bytes)} ${bytes === 1 ? "byte" : "bytes"}`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  // One decimal only where it says something: 1.2 GB is a different amount
  // from 1.9 GB, while 512.3 KB and 512 KB are the same fact.
  const rounded = value >= 100 || unit === 0 ? Math.round(value) : Math.round(value * 10) / 10;
  return `${rounded} ${units[unit]}`;
}
