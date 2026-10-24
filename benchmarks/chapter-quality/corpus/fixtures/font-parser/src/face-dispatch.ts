export type OpenResult =
  | { ok: true; format: string; bytes: Uint8Array }
  | { ok: false; error: string };

declare function hasPrefix(bytes: Uint8Array, prefix: string): boolean;
declare function pcfFace(bytes: Uint8Array): OpenResult;
declare function pfrFace(bytes: Uint8Array): OpenResult;
declare function isBdfText(bytes: Uint8Array): boolean;
declare function parseBdfConstructorError(bytes: Uint8Array): string | null;
declare function bdfFace(bytes: Uint8Array): OpenResult;
declare function trueTypeFace(bytes: Uint8Array): OpenResult;
declare function readTable(bytes: Uint8Array, tag: string): Uint8Array;

export function openFace(bytes: Uint8Array): OpenResult {
  if (hasPrefix(bytes, "PCF")) return pcfFace(bytes);
  if (hasPrefix(bytes, "PFR0")) return pfrFace(bytes);
  if (isBdfText(bytes)) {
    const error = parseBdfConstructorError(bytes);
    if (error) return { ok: false, error };
    return bdfFace(bytes);
  }
  return trueTypeFace(bytes);
}

export function readTables(bytes: Uint8Array) {
  const loca_data = readTable(bytes, "loca");
  const glyf_data = readTable(bytes, "glyf");
  return { loca_data, glyf_data };
}
