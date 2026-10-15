export function normalizeThreshold(value: string): string | null {
  const text = value.trim();
  if (!/^(?:\d+|\d{1,3}(?:,\d{3})+)$/.test(text)) return null;
  const n = BigInt(text.replaceAll(",", ""));
  return n > 0n && n <= 18446744073709551615n ? n.toString() : null;
}
