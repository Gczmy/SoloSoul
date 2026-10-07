/** 固定公开首屏：独立核验 PNG CRC、尺寸和正文掩码，禁止自动更新参考。 */
import { createHash } from 'node:crypto';
import { inflateSync } from 'node:zlib';
export const PDF_ASSET_SHA = 'ca60313e25ffa64f848d86780201a9570a0dbcb3bf733bfb5e4a65ed73f4fdfe';
export const PDF_MASK_SHA = '6f307e964c1dba0a23593241d6d193823cdf8feebb3946ca80b5af9a4a165044';
export const PDF_VIEWPORT = Object.freeze({ width: 1028, height: 749 });
export const PDF_ROI = Object.freeze({ x: 180, y: 225, width: 430, height: 36 });
export function pngCrc(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let i = 0; i < 8; i++) crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0);
  }
  return (crc ^ 0xffffffff) >>> 0;
}
export function inspectPdfPixels(bytes) {
  if (
    !Buffer.isBuffer(bytes) ||
    bytes.length < 33 ||
    bytes.length > 256 * 1024 ||
    !bytes.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]))
  )
    throw new Error('PDF PNG bounds or signature rejected');
  let at = 8,
    header = false,
    end = false,
    parts = [],
    chunks = 0;
  while (at < bytes.length) {
    if (++chunks > 64 || at + 12 > bytes.length) throw new Error('PDF PNG chunks rejected');
    const length = bytes.readUInt32BE(at),
      next = at + 12 + length;
    if (
      next > bytes.length ||
      pngCrc(bytes.subarray(at + 4, next - 4)) !== bytes.readUInt32BE(next - 4)
    )
      throw new Error('PDF PNG CRC or length rejected');
    const kind = bytes.toString('ascii', at + 4, at + 8),
      data = bytes.subarray(at + 8, next - 4);
    if (kind === 'IHDR') {
      if (
        header ||
        at !== 8 ||
        length !== 13 ||
        data.readUInt32BE(0) !== 1028 ||
        data.readUInt32BE(4) !== 749 ||
        !data.subarray(8).equals(Buffer.from([8, 6, 0, 0, 0]))
      )
        throw new Error('PDF fixed viewport or RGBA format changed');
      header = true;
    } else if (!header) throw new Error('PDF PNG missing header');
    else if (kind === 'IDAT') parts.push(data);
    else if (kind === 'IEND') {
      if (length !== 0 || next !== bytes.length || parts.length === 0)
        throw new Error('PDF PNG end rejected');
      end = true;
    } else if (kind[0] === kind[0].toUpperCase()) throw new Error('PDF PNG unknown critical chunk');
    at = next;
  }
  if (!end) throw new Error('PDF PNG incomplete');
  const stride = 1028 * 4,
    expected = (stride + 1) * 749,
    raw = inflateSync(Buffer.concat(parts), { maxOutputLength: expected });
  if (raw.length !== expected) throw new Error('PDF PNG decoded size rejected');
  let previous = Buffer.alloc(stride),
    mask = Buffer.alloc(PDF_ROI.width * PDF_ROI.height),
    dark = 0;
  const paeth = (a, b, c) => {
    const p = a + b - c,
      pa = Math.abs(p - a),
      pb = Math.abs(p - b),
      pc = Math.abs(p - c);
    return pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
  };
  for (let y = 0; y < 749; y++) {
    const filter = raw[y * (stride + 1)];
    if (filter > 4) throw new Error('PDF PNG filter rejected');
    const row = Buffer.allocUnsafe(stride);
    for (let i = 0; i < stride; i++) {
      const a = i >= 4 ? row[i - 4] : 0,
        b = previous[i],
        c = i >= 4 ? previous[i - 4] : 0;
      row[i] =
        (raw[y * (stride + 1) + 1 + i] +
          [0, a, b, Math.floor((a + b) / 2), paeth(a, b, c)][filter]) &
        255;
    }
    if (y >= PDF_ROI.y && y < PDF_ROI.y + PDF_ROI.height)
      for (let x = PDF_ROI.x; x < PDF_ROI.x + PDF_ROI.width; x++) {
        const i = x * 4,
          m = Number(row[i + 3] >= 240 && row[i] <= 64 && row[i + 1] <= 64 && row[i + 2] <= 64);
        mask[(y - PDF_ROI.y) * PDF_ROI.width + x - PDF_ROI.x] = m;
        dark += m;
      }
    previous = row;
  }
  const hash = createHash('sha256').update(mask).digest('hex');
  return {
    ...PDF_VIEWPORT,
    darkPixels: dark,
    maskSha256: hash,
    matches: dark === 530 && hash === PDF_MASK_SHA,
  };
}
