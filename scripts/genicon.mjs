// 生成 Elimitate 正式应用图标（纯 Node 实现，无依赖，4x 超采样抗锯齿）
// 输出: src-tauri/icons/{32x32.png,128x128.png,128x128@2x.png,icon.ico,icon.png(1024 母版)}
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
  ihdr[8] = 8;
  ihdr[9] = 6; // RGBA
  const raw = Buffer.alloc((width * 4 + 1) * height);
  for (let y = 0; y < height; y++) {
    raw[y * (width * 4 + 1)] = 0;
    rgba.copy(raw, y * (width * 4 + 1) + 1, y * width * 4, (y + 1) * width * 4);
  }
  return Buffer.concat([sig, chunk("IHDR", ihdr), chunk("IDAT", zlib.deflateSync(raw, { level: 9 })), chunk("IEND", Buffer.alloc(0))]);
}

// ---------- 绘制 ----------
const S = 2048; // 超采样画布
const px = new Float64Array(S * S * 4); // 线性 RGBA
const RAD = S * 0.205; // 圆角半径

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

// 圆角矩形覆盖率（SDF）
function roundRectCov(x, y, cx, cy, hw, hh, r) {
  const dx = Math.max(Math.abs(x - cx) - (hw - r), 0);
  const dy = Math.max(Math.abs(y - cy) - (hh - r), 0);
  const d = Math.hypot(dx, dy);
  return Math.max(0, Math.min(1, r - d + 0.5));
}

// 线段（圆头 + 覆盖度抗锯齿）
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
      const pxx = x + 0.5, pyy = y + 0.5;
      const ux = pxx - x0, uy = pyy - y0;
      const proj = (ux * dx + uy * dy) / d;
      const t = Math.min(Math.max(proj, 0), d);
      const dPerp = Math.abs(ux * nx + uy * ny) / half;
      const dAlong = Math.abs(proj - t);
      const cov = Math.max(0, Math.min(1, 1 - dPerp)) * Math.max(0, Math.min(1, 1 - dAlong));
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

// 四角星芒
function star(cx, cy, r) {
  circle(cx, cy, r * 0.52, 122, 184, 255, 42); // 柔光
  for (let k = 0; k < 4; k++) {
    const ang = ((k * 90 + 45) * Math.PI) / 180;
    line(cx, cy, cx + Math.cos(ang) * r, cy + Math.sin(ang) * r, Math.max(10, r * 0.16), 122, 184, 255, 255);
  }
  circle(cx, cy, Math.max(8, r * 0.17), 255, 255, 255, 255);
}

function draw() {
  const c = S / 2;
  // 背景：深蓝渐变圆角方块
  for (let y = 0; y < S; y++) {
    const t = y / S;
    const r = Math.round(46 + (16 - 46) * t);
    const g = Math.round(62 + (26 - 62) * t);
    const b = Math.round(112 + (40 - 112) * t);
    for (let x = 0; x < S; x++) {
      const cov = roundRectCov(x + 0.5, y + 0.5, c, c, c, c, RAD);
      if (cov > 0) setPx(x, y, r, g, b, 255 * cov);
    }
  }
  // 内描边（提亮）
  const ring = 16;
  for (let y = 0; y < S; y++) {
    for (let x = 0; x < S; x++) {
      const out = roundRectCov(x + 0.5, y + 0.5, c, c, c, c, RAD);
      const inn = roundRectCov(x + 0.5, y + 0.5, c, c, c - ring, c - ring, RAD - ring);
      const cov = Math.max(0, out - inn);
      if (cov > 0) setPx(x, y, 140, 165, 225, 26 * cov);
    }
  }
  // 扫帚柄（浅色，圆头）
  line(S * 0.085, S * 0.795, S * 0.586, S * 0.42, 56, 233, 238, 248, 255);
  // 手柄握持纹（两条深色短横线）
  for (const t of [0.32, 0.47]) {
    const ax = S * 0.085 + (S * 0.586 - S * 0.085) * t;
    const ay = S * 0.795 + (S * 0.42 - S * 0.795) * t;
    line(ax - 34, ay + 34, ax + 34, ay - 34, 13, 96, 116, 168, 235);
  }
  // 刷毛箍（连接处圆环）
  circle(S * 0.085, S * 0.795, 40, 79, 140, 255, 255);
  // 刷毛扇形（左下展开，长短交错）
  const bx = S * 0.085, by = S * 0.795;
  for (let i = 0; i < 11; i++) {
    const ang = ((168 + (i / 10) * 24) * Math.PI) / 180;
    const len = S * (0.115 + (i % 3) * 0.016);
    line(bx, by, bx + Math.cos(ang) * len, by + Math.sin(ang) * len, 21, 96 + i * 4, 152 + i * 3, 255, 255);
  }
  // 星芒（右上）
  star(S * 0.745, S * 0.225, S * 0.1);
  star(S * 0.852, S * 0.395, S * 0.052);
  star(S * 0.645, S * 0.128, S * 0.034);
}

// ---------- 缩放输出 ----------
function downsample(size) {
  const out = Buffer.alloc(size * size * 4);
  const scale = S / size;
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const x0 = Math.floor(x * scale), y0 = Math.floor(y * scale);
      const x1 = Math.min(Math.ceil((x + 1) * scale), S), y1 = Math.min(Math.ceil((y + 1) * scale), S);
      let r = 0, g = 0, b = 0, a = 0, n = 0;
      for (let sy = y0; sy < y1; sy++) {
        for (let sx = x0; sx < x1; sx++) {
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

const targets = [
  [32, "32x32.png"],
  [128, "128x128.png"],
  [256, "128x128@2x.png"],
  [1024, "icon.png"],
];
for (const [size, name] of targets) {
  const png = encodePNG(size, size, downsample(size));
  fs.writeFileSync(path.join(outDir, name), png);
  console.log(`write ${name} (${png.length} bytes)`);
}

// icon.ico：内嵌 256x256 PNG
{
  const size = 256;
  const png = encodePNG(size, size, downsample(size));
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(1, 4);
  const entry = Buffer.alloc(16);
  entry[0] = 0;
  entry[1] = 0;
  entry.writeUInt16LE(1, 4);
  entry.writeUInt16LE(32, 6);
  entry.writeUInt32LE(png.length, 8);
  entry.writeUInt32LE(22, 12);
  fs.writeFileSync(path.join(outDir, "icon.ico"), Buffer.concat([header, entry, png]));
  console.log(`write icon.ico (${png.length + 22} bytes)`);
}
console.log("done");
