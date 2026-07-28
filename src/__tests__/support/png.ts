import { inflateSync } from 'node:zlib';

const PNG_SIGNATURE = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
const CHANNELS = 4;

export interface DecodedPng {
  width: number;
  height: number;
  /** Row-major RGBA bytes, 4 per pixel. */
  pixels: Buffer;
}

/**
 * Minimal decoder for 8-bit RGBA, non-interlaced PNGs - the only variant the
 * Tauri icon generator emits, and the variant embedded in the SVG source.
 */
export function decodePng(file: Buffer): DecodedPng {
  if (!file.subarray(0, 8).equals(PNG_SIGNATURE)) {
    throw new Error('not a PNG file');
  }

  let header: { width: number; height: number } | undefined;
  const imageData: Buffer[] = [];

  let offset = 8;
  while (offset + 8 <= file.length) {
    const length = file.readUInt32BE(offset);
    const type = file.toString('ascii', offset + 4, offset + 8);
    const body = file.subarray(offset + 8, offset + 8 + length);

    if (type === 'IHDR') {
      const bitDepth = body[8];
      const colorType = body[9];
      const interlace = body[12];
      if (bitDepth !== 8 || colorType !== 6 || interlace !== 0) {
        throw new Error(
          `unsupported PNG variant (bitDepth=${bitDepth}, colorType=${colorType}, interlace=${interlace})`,
        );
      }
      header = { width: body.readUInt32BE(0), height: body.readUInt32BE(4) };
    } else if (type === 'IDAT') {
      imageData.push(body);
    }

    offset += 12 + length;
  }

  if (!header) {
    throw new Error('PNG is missing an IHDR chunk');
  }

  const { width, height } = header;
  const raw = inflateSync(Buffer.concat(imageData));
  const stride = width * CHANNELS;
  const pixels = Buffer.alloc(stride * height);

  let read = 0;
  for (let y = 0; y < height; y++) {
    const filter = raw[read++];
    const line = raw.subarray(read, read + stride);
    read += stride;

    const row = pixels.subarray(y * stride, (y + 1) * stride);
    const previous =
      y > 0 ? pixels.subarray((y - 1) * stride, y * stride) : undefined;

    for (let x = 0; x < stride; x++) {
      const left = x >= CHANNELS ? (row[x - CHANNELS] ?? 0) : 0;
      const up = previous?.[x] ?? 0;
      const upLeft = (x >= CHANNELS ? previous?.[x - CHANNELS] : 0) ?? 0;
      const value = line[x] ?? 0;

      switch (filter) {
        case 0:
          row[x] = value;
          break;
        case 1:
          row[x] = value + left;
          break;
        case 2:
          row[x] = value + up;
          break;
        case 3:
          row[x] = value + ((left + up) >> 1);
          break;
        case 4: {
          const estimate = left + up - upLeft;
          const dLeft = Math.abs(estimate - left);
          const dUp = Math.abs(estimate - up);
          const dUpLeft = Math.abs(estimate - upLeft);
          const paeth =
            dLeft <= dUp && dLeft <= dUpLeft
              ? left
              : dUp <= dUpLeft
                ? up
                : upLeft;
          row[x] = value + paeth;
          break;
        }
        default:
          throw new Error(`unsupported PNG row filter ${filter}`);
      }
    }
  }

  return { width, height, pixels };
}

/**
 * Resolution-independent fingerprint: the image reduced to `size` x `size`
 * cells of alpha-weighted average RGB plus average alpha, so artwork can be
 * compared across the different sizes the icon generator emits.
 */
export function averageGrid(image: DecodedPng, size: number): Float64Array {
  const grid = new Float64Array(size * size * CHANNELS);

  for (let cellY = 0; cellY < size; cellY++) {
    for (let cellX = 0; cellX < size; cellX++) {
      const startX = Math.floor((cellX * image.width) / size);
      const endX = Math.floor(((cellX + 1) * image.width) / size);
      const startY = Math.floor((cellY * image.height) / size);
      const endY = Math.floor(((cellY + 1) * image.height) / size);

      let red = 0;
      let green = 0;
      let blue = 0;
      let alpha = 0;
      let count = 0;

      for (let y = startY; y < endY; y++) {
        for (let x = startX; x < endX; x++) {
          const index = (y * image.width + x) * CHANNELS;
          const opacity = image.pixels[index + 3] ?? 0;
          const weight = opacity / 255;
          red += (image.pixels[index] ?? 0) * weight;
          green += (image.pixels[index + 1] ?? 0) * weight;
          blue += (image.pixels[index + 2] ?? 0) * weight;
          alpha += opacity;
          count++;
        }
      }

      const cell = (cellY * size + cellX) * CHANNELS;
      grid[cell] = red / count;
      grid[cell + 1] = green / count;
      grid[cell + 2] = blue / count;
      grid[cell + 3] = alpha / count;
    }
  }

  return grid;
}

/** Largest per-channel difference between two grids of the same size. */
export function maxGridDifference(a: Float64Array, b: Float64Array): number {
  if (a.length !== b.length) {
    throw new Error('grids have different sizes');
  }

  let max = 0;
  for (let index = 0; index < a.length; index++) {
    max = Math.max(max, Math.abs((a[index] ?? 0) - (b[index] ?? 0)));
  }
  return max;
}
