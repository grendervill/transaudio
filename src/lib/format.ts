const UNITS = ["Б", "КБ", "МБ", "ГБ", "ТБ"];

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 Б";
  let value = bytes;
  let unit = 0;
  while (value >= 1000 && unit < UNITS.length - 1) {
    value /= 1000;
    unit += 1;
  }
  const rounded =
    unit === 0 ? Math.round(value).toString() : value.toFixed(1).replace(".", ",").replace(",0", "");
  return `${rounded} ${UNITS[unit]}`;
}

export function formatDuration(seconds: number): string {
  const total = Math.max(0, Math.floor(seconds || 0));
  const secs = total % 60;
  const minutes = Math.floor(total / 60) % 60;
  const hours = Math.floor(total / 3600);
  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, "0")}:${String(secs).padStart(2, "0")}`
    : `${minutes}:${String(secs).padStart(2, "0")}`;
}

export function formatTimecode(seconds: number): string {
  const total = Math.max(0, Math.floor(seconds || 0));
  const secs = total % 60;
  const minutes = Math.floor(total / 60) % 60;
  const hours = Math.floor(total / 3600);
  return hours > 0
    ? `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}:${String(secs).padStart(2, "0")}`
    : `${String(minutes).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
}
