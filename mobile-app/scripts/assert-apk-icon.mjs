// 安卓图标同源与消费链静态锁。不依赖编译，纯 node 无第三方包。
//
// 调用方 .github/workflows/android.yml 的 严格冒烟·4 校验图标同源与消费链 步，
// 以及本地门禁与变异自证。
//
// 入参 两种形态。
//   自检 node scripts/assert-apk-icon.mjs --self-test
//   判定 node scripts/assert-apk-icon.mjs <APK路径> <安卓图标源图路径> <web logo 路径> <生成 mipmap 路径>
//        工作目录为 mobile-app，生成 mipmap 路径指 生成应用图标 步产出的
//        src-tauri/icons/android/mipmap-xxxhdpi/ic_launcher.png。
//
// 做什么 三段判据，缺任一段判红并以退出码 1 结束。
//   段一 同源，安卓图标源图与 web logo 必须字节同一，任一被单独改动即断链判红。
//   段二 消费链，APK 内 res/mipmap-xxxhdpi 的 ic_launcher.png 必须与生成步产出的
//        同名文件一致。分两档形态。
//        debug 档，原名条目存在时只与它比字节。已实测 aapt2 对 PNG 条目原样存储
//        不做二次编码，模板默认图与生成图字节不符，字节同一是比像素更强的硬锁，保留不动。
//        release 档，原名条目整体缺席（打包器改写资源存放路径，实测同一份注入产物
//        debug 判绿 release 报条目不存在），且 release 构建的 PNG crunch 会做 libpng
//        无损二次编码，IDAT 分块改写并剥掉 gAMA sRGB pHYs 等辅助块，条目名还被改成
//        两字符短名，字节等值在该档恒不成立。故 release 档改为像素等值。解 PNG 的
//        IHDR 拿宽高位深色彩类型，压缩与 filter 方法必须为 0、隔行必须为 0，拼合全部
//        IDAT 用 zlib inflate，按五种 filter（none sub up average paeth）逐行还原原始
//        scanline，取 sha256 与生成步产物同法摘要比对。crunch 无损，像素必等，
//        无阈值无容差，两形态都没命中即判红，不降级为警告。
//   段三 存在性，release 档下包内必须能找到 192x192 且 RGBA（色彩类型 6 位深 8）的
//        res PNG 条目，用来钉住图标尺寸族确实进了包。像素等值只证同图在包内，
//        尺寸族存在性防同字节流换了尺寸形态混过。debug 档字节同一已蕴含该断言。
// 传给谁 失败消息点名文件与实测字节数，并附包内 res PNG 条目数量与名字样例，供 CI 日志直接定位。
// 存哪里 只读判定，不落盘任何文件。
//
// 自检模式合成九枚 APK 夹具，debug 同字节判绿，debug 异字节判红，缺条目判红，
// 源分家判红，release 重编码同像素判绿，release 异像素判红，release 同像素但缺
// 192x192 RGBA 尺寸族判红，路径改写同字节判绿，路径改写内容不可解码判红，
// 九个判定方向都验才算判定器自身可信。

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import crypto from 'node:crypto';
import zlib from 'node:zlib';

function fail(message) {
  console.error('::error::' + message);
  throw new Error(message);
}

function info(message) {
  console.log(message);
}

// ---------- 只读 ZIP 解析，够吃 APK ----------

function parseZipEntries(apkBuffer, label) {
  let eocd = -1;
  const scanFrom = Math.max(0, apkBuffer.length - 66000);
  for (let i = apkBuffer.length - 22; i >= scanFrom; i--) {
    if (apkBuffer.readUInt32LE(i) === 0x06054b50) {
      eocd = i;
      break;
    }
  }
  if (eocd === -1) fail(label + ' 找不到 ZIP 中央目录结尾标记，不是有效 APK');
  const entryCount = apkBuffer.readUInt16LE(eocd + 10);
  let directoryOffset = apkBuffer.readUInt32LE(eocd + 16);
  const entries = [];
  for (let n = 0; n < entryCount; n++) {
    if (apkBuffer.readUInt32LE(directoryOffset) !== 0x02014b50) fail(label + ' 中央目录条目 ' + n + ' 签名不符');
    const method = apkBuffer.readUInt16LE(directoryOffset + 10);
    const compressedSize = apkBuffer.readUInt32LE(directoryOffset + 20);
    const nameLength = apkBuffer.readUInt16LE(directoryOffset + 28);
    const extraLength = apkBuffer.readUInt16LE(directoryOffset + 30);
    const commentLength = apkBuffer.readUInt16LE(directoryOffset + 32);
    const localHeaderOffset = apkBuffer.readUInt32LE(directoryOffset + 42);
    const name = apkBuffer.toString('utf8', directoryOffset + 46, directoryOffset + 46 + nameLength);
    entries.push({ name, method, compressedSize, localHeaderOffset });
    directoryOffset += 46 + nameLength + extraLength + commentLength;
  }
  return entries;
}

function readEntryData(apkBuffer, entry, label) {
  const local = entry.localHeaderOffset;
  if (apkBuffer.readUInt32LE(local) !== 0x04034b50) fail(label + ' 条目 ' + entry.name + ' 本地头签名不符');
  const localNameLength = apkBuffer.readUInt16LE(local + 26);
  const localExtraLength = apkBuffer.readUInt16LE(local + 28);
  const dataStart = local + 30 + localNameLength + localExtraLength;
  const raw = apkBuffer.slice(dataStart, dataStart + entry.compressedSize);
  if (entry.method === 0) return raw;
  if (entry.method === 8) return zlib.inflateRawSync(raw);
  fail(label + ' 条目 ' + entry.name + ' 压缩方式 ' + entry.method + ' 不支持');
  return null;
}

// ---------- 最小 ZIP 写出，仅自检夹具用，stored 无压缩 ----------

function crc32(buffer) {
  let table = crc32.table;
  if (!table) {
    table = crc32.table = new Int32Array(256);
    for (let n = 0; n < 256; n++) {
      let c = n;
      for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      table[n] = c;
    }
  }
  let x = -1;
  for (const byte of buffer) x = (x >>> 8) ^ table[(x ^ byte) & 255];
  return (x ^ -1) >>> 0;
}

function buildZip(entries) {
  const locals = [];
  const centrals = [];
  let offset = 0;
  for (const entry of entries) {
    const nameBuf = Buffer.from(entry.name, 'utf8');
    const crc = crc32(entry.data);
    const local = Buffer.alloc(30);
    local.writeUInt32LE(0x04034b50, 0);
    local.writeUInt16LE(20, 4);
    local.writeUInt32LE(crc, 14);
    local.writeUInt32LE(entry.data.length, 18);
    local.writeUInt32LE(entry.data.length, 22);
    local.writeUInt16LE(nameBuf.length, 26);
    locals.push(local, nameBuf, entry.data);
    const central = Buffer.alloc(46);
    central.writeUInt32LE(0x02014b50, 0);
    central.writeUInt16LE(20, 4);
    central.writeUInt16LE(20, 6);
    central.writeUInt32LE(crc, 16);
    central.writeUInt32LE(entry.data.length, 20);
    central.writeUInt32LE(entry.data.length, 24);
    central.writeUInt16LE(nameBuf.length, 28);
    central.writeUInt32LE(offset, 42);
    centrals.push(central, nameBuf);
    offset += 30 + nameBuf.length + entry.data.length;
  }
  const centralBuf = Buffer.concat(centrals);
  const eocd = Buffer.alloc(22);
  eocd.writeUInt32LE(0x06054b50, 0);
  eocd.writeUInt16LE(entries.length, 8);
  eocd.writeUInt16LE(entries.length, 10);
  eocd.writeUInt32LE(centralBuf.length, 12);
  eocd.writeUInt32LE(offset, 16);
  return Buffer.concat([...locals, centralBuf, eocd]);
}

// ---------- PNG 解码，release 档像素等值判定用 ----------

const PNG_SIG = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
const PNG_CHANNELS = { 0: 1, 2: 3, 3: 1, 4: 2, 6: 4 };
const PNG_BIT_DEPTHS = { 0: [1, 2, 4, 8, 16], 2: [1, 2, 4, 8, 16], 3: [1, 2, 4, 8], 4: [8, 16], 6: [8, 16] };

function paeth(a, b, c) {
  const p = a + b - c;
  const pa = Math.abs(p - a);
  const pb = Math.abs(p - b);
  const pc = Math.abs(p - c);
  return pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
}

function decodePngPixels(buffer, label) {
  if (buffer.length < 8 || !buffer.subarray(0, 8).equals(PNG_SIG)) throw new Error(label + ' 无 PNG 签名');
  let ihdr = null;
  const idatParts = [];
  let off = 8;
  while (off + 12 <= buffer.length) {
    const len = buffer.readUInt32BE(off);
    const type = buffer.toString('latin1', off + 4, off + 8);
    if (off + 12 + len > buffer.length) throw new Error(label + ' 块 ' + type + ' 越界');
    const data = buffer.subarray(off + 8, off + 8 + len);
    const crc = buffer.readUInt32BE(off + 8 + len);
    if (crc32(Buffer.concat([Buffer.from(type, 'latin1'), data])) !== crc) throw new Error(label + ' 块 ' + type + ' CRC 不符');
    if (type === 'IHDR') {
      if (ihdr !== null) throw new Error(label + ' IHDR 重复');
      if (len !== 13) throw new Error(label + ' IHDR 长度应为 13');
      ihdr = Buffer.from(data);
    } else if (type === 'IDAT') {
      idatParts.push(Buffer.from(data));
    } else if (type === 'IEND') {
      break;
    }
    off += 12 + len;
  }
  if (ihdr === null) throw new Error(label + ' 缺 IHDR');
  if (idatParts.length === 0) throw new Error(label + ' 缺 IDAT');
  const width = ihdr.readUInt32BE(0);
  const height = ihdr.readUInt32BE(4);
  const bitDepth = ihdr[8];
  const colorType = ihdr[9];
  if (ihdr[10] !== 0 || ihdr[11] !== 0) throw new Error(label + ' 压缩或 filter 方法非 0');
  if (ihdr[12] !== 0) throw new Error(label + ' 隔行扫描不支持');
  const channels = PNG_CHANNELS[colorType];
  if (!channels) throw new Error(label + ' 色彩类型 ' + colorType + ' 未知');
  if (!PNG_BIT_DEPTHS[colorType].includes(bitDepth)) throw new Error(label + ' 色彩类型 ' + colorType + ' 位深 ' + bitDepth + ' 非法');
  const inflated = zlib.inflateSync(Buffer.concat(idatParts));
  const rawLen = Math.ceil((width * channels * bitDepth) / 8);
  const stride = rawLen + 1;
  if (inflated.length !== stride * height) throw new Error(label + ' IDAT 解压长度与宽高不符');
  const bpp = Math.max(1, Math.floor((channels * bitDepth + 7) / 8));
  const raw = Buffer.alloc(rawLen * height);
  let prev = Buffer.alloc(rawLen);
  for (let y = 0; y < height; y++) {
    const base = y * stride;
    const ft = inflated[base];
    if (ft > 4) throw new Error(label + ' 第 ' + y + ' 行 filter ' + ft + ' 非法');
    const cur = Buffer.allocUnsafe(rawLen);
    for (let x = 0; x < rawLen; x++) {
      const v = inflated[base + 1 + x];
      const a = x >= bpp ? cur[x - bpp] : 0;
      const b = prev[x];
      const c = x >= bpp ? prev[x - bpp] : 0;
      let rec;
      switch (ft) {
        case 0: rec = v; break;
        case 1: rec = v + a; break;
        case 2: rec = v + b; break;
        case 3: rec = v + ((a + b) >> 1); break;
        default: rec = v + paeth(a, b, c);
      }
      cur[x] = rec & 0xff;
    }
    cur.copy(raw, y * rawLen);
    prev = cur;
  }
  return { width, height, bitDepth, colorType, raw };
}

function pixelDigest(buffer, label) {
  const decoded = decodePngPixels(buffer, label);
  return {
    width: decoded.width,
    height: decoded.height,
    bitDepth: decoded.bitDepth,
    colorType: decoded.colorType,
    digest: crypto.createHash('sha256').update(decoded.raw).digest('hex'),
  };
}

// ---------- 判定主流程 ----------

const XXXHDPI_ICON = /^res\/mipmap-xxxhdpi[^/]*\/ic_launcher\.png$/;
const FAMILY_WIDTH = 192;
const FAMILY_HEIGHT = 192;
const FAMILY_COLOR_TYPE = 6;
const FAMILY_BIT_DEPTH = 8;

function assertApkIcon(apkPath, sourceIconPath, webLogoPath, generatedMipmapPath) {
  const sourceBuffer = fs.readFileSync(sourceIconPath);
  const webLogoBuffer = fs.readFileSync(webLogoPath);

  if (!sourceBuffer.equals(webLogoBuffer)) {
    fail(
      '图标断链，安卓图标源图与 web logo 不是同一份文件，' +
        path.basename(sourceIconPath) +
        ' 共 ' + sourceBuffer.length + ' 字节，' +
        path.basename(webLogoPath) +
        ' 共 ' + webLogoBuffer.length + ' 字节，需从同一枚品牌主图取源'
    );
  }
  info('同源通过，安卓图标源图与 web logo 字节同一，共 ' + sourceBuffer.length + ' 字节');

  const apkBuffer = fs.readFileSync(apkPath);
  const entries = parseZipEntries(apkBuffer, apkPath);
  const generatedBuffer = fs.readFileSync(generatedMipmapPath);

  const direct = entries.find((entry) => XXXHDPI_ICON.test(entry.name));
  if (direct) {
    const data = readEntryData(apkBuffer, direct, apkPath);
    if (!data.equals(generatedBuffer)) {
      fail(
        '消费链断，APK 内 ' + direct.name + ' 共 ' + data.length + ' 字节，与生成步产物 ' +
          generatedMipmapPath + ' 共 ' + generatedBuffer.length +
          ' 字节不符，构建没有吃进 生成应用图标 的 mipmap，包内仍是旧图或模板默认图'
      );
    }
    info('消费链通过，debug 档 APK 条目 ' + direct.name + ' 与生成 mipmap 字节同一，共 ' + data.length + ' 字节');
    return;
  }

  let generated;
  try {
    generated = pixelDigest(generatedBuffer, '生成步产物 ' + generatedMipmapPath);
  } catch (error) {
    fail('消费链断，生成步产物 ' + generatedMipmapPath + ' 无法解码为 PNG，' + error.message + '，无法建立像素基准');
  }

  const resPngs = entries.filter((entry) => entry.name.startsWith('res/') && entry.name.endsWith('.png'));
  let pixelHit = null;
  let familyHit = null;
  let undecodable = 0;
  for (const entry of resPngs) {
    const data = readEntryData(apkBuffer, entry, apkPath);
    let px;
    try {
      px = pixelDigest(data, '条目 ' + entry.name);
    } catch {
      undecodable++;
      continue;
    }
    if (familyHit === null && px.width === FAMILY_WIDTH && px.height === FAMILY_HEIGHT && px.colorType === FAMILY_COLOR_TYPE && px.bitDepth === FAMILY_BIT_DEPTH) {
      familyHit = entry.name;
    }
    if (pixelHit === null && px.digest === generated.digest) {
      pixelHit = entry.name;
    }
  }

  if (pixelHit !== null && familyHit !== null) {
    info(
      '消费链通过，release 档 APK 条目 ' + pixelHit +
        ' 与生成 mipmap 像素同一（unfilter 后 scanline sha256 一致，生成侧 ' + generatedBuffer.length +
        ' 字节），尺寸族存在性通过，包内有 ' + FAMILY_WIDTH + 'x' + FAMILY_HEIGHT +
        ' RGBA 位深 ' + FAMILY_BIT_DEPTH + ' 条目 ' + familyHit +
        '，原名条目缺席与字节改写均不影响该档判定'
    );
    return;
  }

  const sampleNames = resPngs.slice(0, 12).map((entry) => entry.name).join('，');
  if (pixelHit === null) {
    fail(
      '消费链断，APK 内未找到 res/mipmap-xxxhdpi 的 ic_launcher.png 条目，' +
        'res 下 ' + resPngs.length + ' 个 PNG 条目内（' + undecodable + ' 个不可解码）没有与生成步产物 ' +
        generatedMipmapPath + ' 共 ' + generatedBuffer.length + ' 字节像素同一的条目，安卓图标未进包，条目样例 ' +
        (sampleNames || '无')
    );
  }
  fail(
    '消费链断，像素同一已命中条目 ' + pixelHit + '，但 res 下 ' + resPngs.length +
      ' 个 PNG 条目内没有 ' + FAMILY_WIDTH + 'x' + FAMILY_HEIGHT + ' RGBA 位深 ' + FAMILY_BIT_DEPTH +
      ' 的图标尺寸族条目，图标族没有进包，条目样例 ' + (sampleNames || '无')
  );
}

// ---------- 自检，内存合成夹具验判定器双向可信，夹具只落系统临时目录 ----------

const SELF_W = 192;
const SELF_H = 192;

function buildRawPixels(width, height, variant) {
  const raw = Buffer.alloc(width * height * 4);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const i = (y * width + x) * 4;
      raw[i] = (x * 3 + y * 5 + variant) & 0xff;
      raw[i + 1] = (x ^ (y * 2)) & 0xff;
      raw[i + 2] = (y * 7 + x) & 0xff;
      raw[i + 3] = (x + y) & 0xff;
    }
  }
  return raw;
}

function pngChunk(type, data) {
  const head = Buffer.alloc(4);
  head.writeUInt32BE(data.length, 0);
  const typeBuf = Buffer.from(type, 'latin1');
  const tail = Buffer.alloc(4);
  tail.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])), 0);
  return Buffer.concat([head, typeBuf, data, tail]);
}

function encodeFilterRow(cur, prev, bpp, ft) {
  const out = Buffer.allocUnsafe(cur.length);
  for (let x = 0; x < cur.length; x++) {
    const a = x >= bpp ? cur[x - bpp] : 0;
    const b = prev[x];
    const c = x >= bpp ? prev[x - bpp] : 0;
    let pred;
    switch (ft) {
      case 0: pred = 0; break;
      case 1: pred = a; break;
      case 2: pred = b; break;
      case 3: pred = (a + b) >> 1; break;
      default: pred = paeth(a, b, c);
    }
    out[x] = (cur[x] - pred) & 0xff;
  }
  return out;
}

function buildSelfPng(width, height, raw, filterFor, idatSize, level, withAncillary) {
  const rawLen = width * 4;
  if (raw.length !== rawLen * height) fail('自检构造 raw 长度与宽高不符');
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  ihdr[10] = 0;
  ihdr[11] = 0;
  ihdr[12] = 0;
  const rows = [];
  let prev = Buffer.alloc(rawLen);
  for (let y = 0; y < height; y++) {
    const cur = raw.subarray(y * rawLen, (y + 1) * rawLen);
    const ft = filterFor(y);
    rows.push(Buffer.concat([Buffer.from([ft]), encodeFilterRow(cur, prev, 4, ft)]));
    prev = cur;
  }
  const idat = zlib.deflateSync(Buffer.concat(rows), { level });
  const chunks = [PNG_SIG, pngChunk('IHDR', ihdr)];
  if (withAncillary) {
    const gama = Buffer.from([0x00, 0x00, 0xb1, 0x30]);
    const srgb = Buffer.from([0x00]);
    const phys = Buffer.from([0x00, 0x00, 0x2e, 0x23, 0x00, 0x00, 0x2e, 0x23, 0x01]);
    chunks.push(pngChunk('gAMA', gama), pngChunk('sRGB', srgb), pngChunk('pHYs', phys));
  }
  for (let o = 0; o < idat.length; o += idatSize) {
    chunks.push(pngChunk('IDAT', idat.subarray(o, Math.min(o + idatSize, idat.length))));
  }
  chunks.push(pngChunk('IEND', Buffer.alloc(0)));
  return Buffer.concat(chunks);
}

function selfTest() {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'apk-icon-selftest-'));
  const filePath = (name) => path.join(tmpDir, name);
  const manifest = { name: 'AndroidManifest.xml', data: Buffer.from('<manifest/>') };
  const directName = 'res/mipmap-xxxhdpi-v4/ic_launcher.png';

  const genRaw = buildRawPixels(SELF_W, SELF_H, 0);
  const genPng = buildSelfPng(SELF_W, SELF_H, genRaw, (y) => y % 5, 1024, 9, true);
  const crunchPng = buildSelfPng(SELF_W, SELF_H, genRaw, () => 0, 8192, 6, false);
  if (crunchPng.equals(genPng)) fail('自检失败，重编码夹具字节未变，编码差异没造出来');
  const flippedRaw = Buffer.from(genRaw);
  flippedRaw[0] ^= 0xff;
  const pixelDiffPng = buildSelfPng(SELF_W, SELF_H, flippedRaw, () => 0, 8192, 6, false);
  const wrongSizePng = buildSelfPng(96, 384, genRaw, () => 0, 8192, 6, false);
  const decoyRaw = buildRawPixels(48, 48, 7);
  const decoyPng = buildSelfPng(48, 48, decoyRaw, (y) => y % 5, 1024, 9, true);

  fs.writeFileSync(filePath('source.png'), genPng);
  fs.writeFileSync(filePath('webbad.png'), pixelDiffPng);

  const zip = (entries2) => buildZip([manifest, ...entries2]);
  fs.writeFileSync(filePath('good.apk'), zip([{ name: directName, data: genPng }]));
  fs.writeFileSync(filePath('bad.apk'), zip([{ name: directName, data: pixelDiffPng }]));
  fs.writeFileSync(filePath('noicon.apk'), zip([]));
  fs.writeFileSync(filePath('crunchgreen.apk'), zip([
    { name: 'res/aC.png', data: decoyPng },
    { name: 'res/rK.png', data: crunchPng },
  ]));
  fs.writeFileSync(filePath('pixelred.apk'), zip([
    { name: 'res/aC.png', data: decoyPng },
    { name: 'res/rK.png', data: pixelDiffPng },
  ]));
  fs.writeFileSync(filePath('nosize.apk'), zip([{ name: 'res/aB.png', data: wrongSizePng }]));
  fs.writeFileSync(filePath('shortgood.apk'), zip([
    { name: 'res/aC.png', data: decoyPng },
    { name: 'res/aD.png', data: genPng },
  ]));
  fs.writeFileSync(filePath('shortbad.apk'), zip([
    { name: 'res/aC.png', data: Buffer.from('OTHER-RES-ID-DIFFERENT-BYTES') },
    { name: 'res/aD.png', data: Buffer.from('TEMPLATE-DEFAULT-ICON-BYTES') },
  ]));

  const expectGreen = (apk, note) => {
    try {
      runQuiet(() => assertApkIcon(filePath(apk), filePath('source.png'), filePath('source.png'), filePath('source.png')));
    } catch {
      fail('自检失败，' + note + ' 必须判绿但判定器判了红，判定器不可信');
    }
  };
  const expectRed = (apk, web, note) => {
    let detected = false;
    try {
      runQuiet(() => assertApkIcon(filePath(apk), filePath('source.png'), filePath(web), filePath('source.png')));
    } catch {
      detected = true;
    }
    if (!detected) fail('自检失败，' + note + ' 必须判红但没有判红，判定器不可信');
  };

  try {
    expectGreen('good.apk', 'debug 档同字节');
    expectRed('bad.apk', 'source.png', 'debug 档异字节');
    expectRed('noicon.apk', 'source.png', '缺 ic_launcher 条目');
    let splitDetected = false;
    try {
      runQuiet(() => assertApkIcon(filePath('good.apk'), filePath('source.png'), filePath('webbad.png'), filePath('source.png')));
    } catch {
      splitDetected = true;
    }
    if (!splitDetected) fail('自检失败，源图与 web logo 分家必须判同源断链红，判定器不可信');
    expectGreen('crunchgreen.apk', 'release 档重编码同像素');
    expectRed('pixelred.apk', 'source.png', 'release 档异像素');
    expectRed('nosize.apk', 'source.png', 'release 档像素同一但缺 192x192 RGBA 尺寸族');
    expectGreen('shortgood.apk', '路径改写同字节');
    expectRed('shortbad.apk', 'source.png', 'release 档条目内容不可解码');
    info('自检通过 9 条，覆盖 debug 同字节判绿、debug 异字节判红、缺条目判红、源分家判红、release 重编码同像素判绿、release 异像素判红、release 同像素缺尺寸族判红、路径改写同字节判绿、路径改写不可解码判红');
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
}

function runQuiet(fn) {
  const originalLog = console.log;
  const originalError = console.error;
  const originalExit = process.exitCode;
  console.log = () => {};
  console.error = () => {};
  try {
    return fn();
  } finally {
    console.log = originalLog;
    console.error = originalError;
    process.exitCode = originalExit;
  }
}

function main() {
  const args = process.argv.slice(2);
  if (args[0] === '--self-test') {
    selfTest();
    return;
  }
  if (args.length !== 4) {
    fail('用法 assert-apk-icon.mjs <APK路径> <安卓图标源图路径> <web logo 路径> <生成 mipmap 路径> 或 --self-test');
  }
  assertApkIcon(args[0], args[1], args[2], args[3]);
}

try {
  main();
} catch (error) {
  // fail 已打印 ::error:: 行，这里只负责以非零码退出，不重复堆栈
  if (!error.message.startsWith('图标断链') && !error.message.startsWith('消费链断') && !error.message.startsWith('自检失败') && !error.message.startsWith('用法')) {
    console.error('::error::判定器异常 ' + error.message);
  }
  process.exit(1);
}
