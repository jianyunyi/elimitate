// 生成 Elimitate 应用图标（纯 Node 实现，无依赖）
// 输出: src-tauri/icons/{32x32.png,128x128.png,128x128@2x.png,icon.ico}
import zlib from "node:zlib";
import fs from "node:fs";
import path from "node:path";

// ---------- PNG 编码 ----------
const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();

function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = CRC_TABLE[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const typeBuf = Buffer.from(type, "ascii");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])), 0);
  return Buffer.concat([len, typeBuf, data, crc]);
}

function encodePNG(width, height, rgba) {
  const sig = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // color type RGBA
  const raw = Buffer.alloc((width * 4 + 1) * height);
  for (let y = 0; y < height; y++) {
    raw[y * (width * 4 + 1)] = 0; // filter: none
    rgba.copy(raw, y * (width * 4 + 1) + 1, y * width * 4, (y + 1) * width * 4);
  }
  const idat = zlib.deflateSync(raw, { level: 9 });
  return Buffer.concat([sig, chunk("IHDR", ihdr), chunk("IDAT", idat), chunk("IEND", Buffer.alloc(0))]);
}

// ---------- 绘制 ----------
const S = 1024; // 超采样画布
const px = new Float64Array(S * S * 4); // 线性 RGBA

function setPx(x, y, r, g, b, a) {
  if (x < 0 || y < 0 || x >= S || y >= S) return;
  const i = (y * S + x) * 4;
  const na = a / 255;
  const oa = px[i + 3] / 255;
  const outA = na + oa * (1 - na);
  if (outA <= 0) return;
  px[i] = (r * na + px[i] * oa * (1 - na)) / outA;
  px[i + 1] = (g * na + px[i + 1] * oa * (1 - na)) / outA;
  px[i + 2] = (b * na + px[i + 2] * oa * (1 - na)) / outA;
  px[i + 3] = outA * 255;
}

// 圆角矩形遮罩
function roundedRect(x, y, w, h, r) {
  const cx = Math.min(Math.max(x, r), S - r);
  const cy = Math.min(Math.max(y, r), S - r);
  const dx = Math.max(Math.abs(x - cx) - (w / 2 - r), 0);
  const dy = Math.max(Math.abs(y - cy) - (h / 2 - r), 0);
  return dx * dx + dy * dy <= r * r;
}

// 线段（含宽度与圆头，覆盖度近似抗锯齿）
function line(x0, y0, x1, y1, width, r, g, b, a) {
  const dx = x1 - x0, dy = y1 - y0;
  const d = Math.hypot(dx, dy) || 1;
  const nx = (-dy / d) * (width / 2);
  const ny = (dx / d) * (width / 2);
  const half = width / 2;
  const minX = Math.floor(Math.min(x0, x1) - half);
  const maxX = Math.ceil(Math.max(x0, x1) + half);
  const minY = Math.floor(Math.min(y0, y1) - half);
  const maxY = Math.ceil(Math.max(y0, y1) + half);
  for (let y = minY; y <= maxY; y++) {
    for (let x = minX; x <= maxX; x++) {
      const px0 = x + 0.5, py0 = y + 0.5;
      const ux = px0 - x0, uy = py0 - y0;
      const proj = (ux * dx + uy * dy) / d; // 沿线段投影
      const t = Math.min(Math.max(proj, 0), d); // 夹取到线段内
      const dPerp = Math.abs(ux * nx + uy * ny) / half; // 垂直距离（归一化）
      const dAlong = Math.abs(proj - t); // 超出端点距离（像素）
      const covX = Math.max(0, Math.min(1, 1 - dPerp));
      const covY = Math.max(0, Math.min(1, 1 - dAlong));
      const cov = covX * covY;
      if (cov > 0) setPx(x, y, r, g, b, a * cov);
    }
  }
}

function circle(cx, cy, rad, r, g, b, a) {
  for (let y = Math.floor(cy - rad); y <= Math.ceil(cy + rad); y++) {
    for (let x = Math.floor(cx - rad); x <= Math.ceil(cx + rad); x++) {
      const d = Math.hypot(x + 0.5 - cx, y + 0.5 - cy);
      const cov = Math.max(0, Math.min(1, rad - d + 0.5));
      if (cov > 0) setPx(x, y, r, g, b, a * cov);
    }
  }
}

function draw() {
  // 背景圆角方块（深蓝渐变）
  for (let y = 0; y < S; y++) {
    for (let x = 0; x < S; x++) {
      if (!roundedRect(x + 0.5, y + 0.5, S, S, 200)) continue;
      const t = (x + y) / (2 * S);
      const r = 15 + t * 12, g = 20 + t * 15, b = 32 + t * 23;
      setPx(x, y, r, g, b, 255);
    }
  }
  // 扫帚：斜向手柄（浅色）
  line(S * 0.16, S * 0.84, S * 0.78, S * 0.22, 46, 230, 235, 245, 255);
  // 扫帚头（底部扇形刷毛）
  const hx = S * 0.16, hy = S * 0.84;
  for (let i = -5; i <= 5; i++) {
    const ang = Math.PI * 0.25 + (i * Math.PI) / 28;
    line(hx, hy, hx + Math.cos(ang) * 150, hy + Math.sin(ang) * 150, 20, 79, 140, 255, 255);
  }
  // 高亮：右上角小星星
  circle(S * 0.74, S * 0.26, 26, 255, 255, 255, 255);
  for (let i = 0; i < 4; i++) {
    const ang = (i * Math.PI) / 2 + Math.PI / 4;
    line(S * 0.74, S * 0.26, S * 0.74 + Math.cos(ang) * 95, S * 0.26 + Math.sin(ang) * 95, 18, 122, 184, 255, 255);
  }
}

// ---------- 缩放输出 ----------
function downsample(size) {
  const out = Buffer.alloc(size * size * 4);
  const scale = S / size;
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const x0 = x * scale, y0 = y * scale;
      let r = 0, g = 0, b = 0, a = 0, n = 0;
      for (let sy = Math.floor(y0); sy < Math.ceil(y0 + scale) && sy < S; sy++) {
        for (let sx = Math.floor(x0); sx < Math.ceil(x0 + scale) && sx < S; sx++) {
          const i = (sy * S + sx) * 4;
          r += px[i]; g += px[i + 1]; b += px[i + 2]; a += px[i + 3];
          n++;
        }
      }
      const o = (y * size + x) * 4;
      if (n > 0) {
        out[o] = Math.round(r / n);
        out[o + 1] = Math.round(g / n);
        out[o + 2] = Math.round(b / n);
        out[o + 3] = Math.round(a / n);
      }
    }
  }
  return out;
}

draw();
const outDir = path.resolve("src-tauri/icons");
fs.mkdirSync(outDir, { recursive: true });

for (const size of [32, 128, 256]) {
  const rgba = downsample(size);
  const png = encodePNG(size, size, rgba);
  const name = size === 256 ? "128x128@2x.png" : `${size}x${size}.png`;
  fs.writeFileSync(path.join(outDir, name), png);
  console.log(`write ${name} (${png.length} bytes)`);
}

// icon.ico：内嵌 256x256 PNG（Vista+ 格式）
{
  const size = 256;
  const rgba = downsample(size);
  const png = encodePNG(size, size, rgba);
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0); // reserved
  header.writeUInt16LE(1, 2); // type icon
  header.writeUInt16LE(1, 4); // count
  const entry = Buffer.alloc(16);
  entry[0] = 0; // width 256
  entry[1] = 0; // height 256
  entry[2] = 0;
  entry[3] = 0;
  entry.writeUInt16LE(1, 4); // planes
  entry.writeUInt16LE(32, 6); // bpp
  entry.writeUInt32LE(png.length, 8);
  entry.writeUInt32LE(22, 12); // offset
  fs.writeFileSync(path.join(outDir, "icon.ico"), Buffer.concat([header, entry, png]));
  console.log(`write icon.ico (${png.length + 22} bytes)`);
}
console.log("done");
