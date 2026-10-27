#!/usr/bin/env node
// 安卓图标同源与消费链静态锁。不依赖编译，纯 node 无第三方包。
//
// 调用方 .github/workflows/android.yml 的 严格冒烟·4 校验图标同源与消费链 步，
// 以及本地门禁与变异自证。
//
// 入参 两种形态。
//   自检 node scripts/assert-apk-icon.mjs --self-test
//   判定 node scripts/assert-apk-icon.mjs <APK路径> <安卓图标源图路径> <web logo 路径>
//        工作目录为 mobile-app。
//
// 做什么 三段判据，缺任一段判红并以退出码 1 结束。
//   段一 同源，安卓图标源图与 web logo 必须字节同一，任一被单独改动即断链判红。
//   段二 消费链，APK 内 res/mipmap-xxxhdpi 的 ic_launcher.png 与源图做 16x16
//        白底合成签名距离，超过阈值说明生成步产物没有进到包里。
//   段三 存在性，APK 内必须能解析出 xxxhdpi ic_launcher.png 条目，缺失即判红。
// 传给谁 失败消息点名文件与实测值，供 CI 日志直接定位。
// 存哪里 只读判定，不落盘任何文件。
//
// 自检模式在内存里合成两枚 APK 夹具，一枚图标与源图同字节应判绿，
// 一枚图标被替换应判红，两向都验才算判定器自身可信。

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import zlib from 'node:zlib';

const DIST_THRESHOLD = 30;
const SIGNATURE_GRID = 16;

function fail(message) {
  console.error('::error::' + message);
  process.exitCode = 1;
  throw new Error(message);
}

function info(message) {
  console.log(message);
}

// ---------- PNG 解码，支持 8 位深非隔行，灰度与带色带α 四种颜色类型 ----------

function decodePng(buffer, label) {
  const signature = [137, 80, 78, 71, 13, 10, 26, 10];
  for (let i = 0; i < 8; i++) {
    if (buffer[i] !== signature[i]) fail(label + ' 不是 PNG 文件');
  }
  let offset = 8;
  let width = 0;
  let height = 0;
  let bitDepth = 0;
  let colorType = 0;
  let interlace = 0;
  const idatParts = [];
  for (;;) {
    if (offset + 12 > buffer.length) fail(label + ' PNG 块头越界，文件被截断');
    const length = buffer.readUInt32BE(offset);
    const type = buffer.toString('ascii', offset + 4, offset + 8);
    const dataStart = offset + 8;
    if (dataStart + length + 4 > buffer.length) fail(label + ' PNG 块 ' + type + ' 长度越界，文件被截断');
    if (type === 'IHDR') {
      width = buffer.readUInt32BE(dataStart);
      height = buffer.readUInt32BE(dataStart + 4);
      bitDepth = buffer[dataStart + 8];
      colorType = buffer[dataStart + 9];
      interlace = buffer[dataStart + 12];
    } else if (type === 'IDAT') {
      idatParts.push(buffer.slice(dataStart, dataStart + length));
    } else if (type === 'IEND') {
      break;
    }
    offset = dataStart + length + 4;
  }
  if (interlace !== 0) fail(label + ' 是隔行 PNG，判定器不支持，需换非隔行源图');
  if (bitDepth !== 8) fail(label + ' 位深为 ' + bitDepth + '，判定器只支持 8 位深');
  const channels = colorType === 6 ? 4 : colorType === 2 ? 3 : colorType === 0 ? 1 : colorType === 4 ? 2 : -1;
  if (channels === -1) fail(label + ' 颜色类型 ' + colorType + ' 不支持，需 8 位深非调色板 PNG');
  const inflated = zlib.inflateSync(Buffer.concat(idatParts));
  const stride = width * channels;
  const pixels = Buffer.alloc(stride * height);
  let previous = Buffer.alloc(stride);
  for (let y = 0; y < height; y++) {
    const filterType = inflated[y * (stride + 1)];
    const line = inflated.slice(y * (stride + 1) + 1, (y + 1) * (stride + 1));
    const current = Buffer.alloc(stride);
    for (let x = 0; x < stride; x++) {
      const a = x >= channels ? current[x - channels] : 0;
      const b = previous[x];
      const c = x >= channels ? previous[x - channels] : 0;
      let value = line[x];
      if (filterType === 1) value = (value + a) & 255;
      else if (filterType === 2) value = (value + b) & 255;
      else if (filterType === 3) value = (value + ((a + b) >> 1)) & 255;
      else if (filterType === 4) {
        const p = a + b - c;
        const pa = Math.abs(p - a);
        const pb = Math.abs(p - b);
        const pc = Math.abs(p - c);
        const predictor = pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
        value = (value + predictor) & 255;
      } else if (filterType !== 0) fail(label + ' 含未知滤波类型 ' + filterType);
      current[x] = value;
    }
    current.copy(pixels, y * stride);
    previous = current;
  }
  return { width, height, channels, colorType, pixels };
}

// ---------- 16x16 分块平均，透明像素按白底合成，尺寸无关 ----------

function signatureOf(decoded) {
  const { width, height, channels, pixels } = decoded;
  // channels 与像素布局对应 1 灰度 2 灰度加α 3 RGB 4 RGBA，透明像素先按白底合成再分块
  const cells = [];
  for (let cellY = 0; cellY < SIGNATURE_GRID; cellY++) {
    for (let cellX = 0; cellX < SIGNATURE_GRID; cellX++) {
      const x0 = Math.floor((cellX * width) / SIGNATURE_GRID);
      const x1 = Math.max(x0 + 1, Math.floor(((cellX + 1) * width) / SIGNATURE_GRID));
      const y0 = Math.floor((cellY * height) / SIGNATURE_GRID);
      const y1 = Math.max(y0 + 1, Math.floor(((cellY + 1) * height) / SIGNATURE_GRID));
      let r = 0;
      let g = 0;
      let b = 0;
      let count = 0;
      for (let y = y0; y < y1; y++) {
        for (let x = x0; x < x1; x++) {
          const base = y * width * channels + x * channels;
          const gray = pixels[base];
          const red = channels === 1 || channels === 2 ? gray : pixels[base];
          const green = channels === 1 || channels === 2 ? gray : pixels[base + 1];
          const blue = channels === 1 || channels === 2 ? gray : pixels[base + 2];
          const alpha = channels === 2 ? pixels[base + 1] : channels === 4 ? pixels[base + 3] : 255;
          r += (red * alpha + 255 * (255 - alpha)) / 255;
          g += (green * alpha + 255 * (255 - alpha)) / 255;
          b += (blue * alpha + 255 * (255 - alpha)) / 255;
          count++;
        }
      }
      cells.push([Math.round(r / count), Math.round(g / count), Math.round(b / count)]);
    }
  }
  return cells;
}

function signatureDistance(first, second) {
  let sum = 0;
  for (let i = 0; i < first.length; i++) {
    sum += (first[i][0] - second[i][0]) ** 2 + (first[i][1] - second[i][1]) ** 2 + (first[i][2] - second[i][2]) ** 2;
  }
  return Math.sqrt(sum / first.length);
}

// ---------- 只读 ZIP 解析，够吃 APK ----------

function readZipEntryByName(apkBuffer, matcher, label) {
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
  const target = entries.find((entry) => matcher(entry.name));
  if (!target) return null;
  const local = target.localHeaderOffset;
  if (apkBuffer.readUInt32LE(local) !== 0x04034b50) fail(label + ' 条目 ' + target.name + ' 本地头签名不符');
  const nameLength = apkBuffer.readUInt16LE(local + 26);
  const extraLength = apkBuffer.readUInt16LE(local + 28);
  const dataStart = local + 30 + nameLength + extraLength;
  const raw = apkBuffer.slice(dataStart, dataStart + target.compressedSize);
  if (target.method === 0) return { name: target.name, data: raw };
  if (target.method === 8) return { name: target.name, data: zlib.inflateRawSync(raw) };
  fail(label + ' 条目 ' + target.name + ' 压缩方式 ' + target.method + ' 不支持');
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
    local.writeUInt16LE(0, 6);
    local.writeUInt16LE(0, 8);
    local.writeUInt16LE(0, 10);
    local.writeUInt16LE(0, 12);
    local.writeUInt32LE(crc, 14);
    local.writeUInt32LE(entry.data.length, 18);
    local.writeUInt32LE(entry.data.length, 22);
    local.writeUInt16LE(nameBuf.length, 26);
    local.writeUInt16LE(0, 28);
    locals.push(local, nameBuf, entry.data);
    const central = Buffer.alloc(46);
    central.writeUInt32LE(0x02014b50, 0);
    central.writeUInt16LE(20, 4);
    central.writeUInt16LE(20, 6);
    central.writeUInt16LE(0, 8);
    central.writeUInt16LE(0, 10);
    central.writeUInt16LE(0, 12);
    central.writeUInt16LE(0, 14);
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

// ---------- 判定主流程 ----------

const XXXHDPI_ICON = /^res\/mipmap-xxxhdpi[^/]*\/ic_launcher\.png$/;

function assertApkIcon(apkPath, sourceIconPath, webLogoPath) {
  const sourceBuffer = fs.readFileSync(sourceIconPath);
  const webLogoBuffer = fs.readFileSync(webLogoPath);

  if (!sourceBuffer.equals(webLogoBuffer)) {
    fail(
      '图标断链，安卓图标源图与 web logo 不是同一份文件，' +
        path.basename(sourceIconPath) +
        ' 与 ' +
        path.basename(webLogoPath) +
        ' 字节不同，需从同一枚品牌主图取源'
    );
  }
  info('同源通过，安卓图标源图与 web logo 字节同一');

  const apkBuffer = fs.readFileSync(apkPath);
  const entry = readZipEntryByName(apkBuffer, (name) => XXXHDPI_ICON.test(name), apkPath);
  if (!entry) {
    fail('消费链断，APK 内未找到 res/mipmap-xxxhdpi 的 ic_launcher.png 条目，安卓图标未进包');
  }
  info('存在性通过，APK 条目 ' + entry.name + ' 共 ' + entry.data.length + ' 字节');

  const apkSignature = signatureOf(decodePng(entry.data, entry.name));
  const sourceSignature = signatureOf(decodePng(sourceBuffer, sourceIconPath));
  const distance = signatureDistance(apkSignature, sourceSignature);
  if (distance > DIST_THRESHOLD) {
    fail(
      '消费链断，APK 内 ic_launcher 与图标源图签名距离 ' +
        distance.toFixed(1) +
        ' 超过阈值 ' +
        DIST_THRESHOLD +
        '，生成步产物未被构建消费，包内仍是旧图或模板默认图'
    );
  }
  info('消费链通过，APK 内 ic_launcher 与源图签名距离 ' + distance.toFixed(1));
  return distance;
}

// ---------- 自检，内存合成夹具验判定器双向可信，夹具只落系统临时目录 ----------

function selfTest() {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'apk-icon-selftest-'));
  const filePath = (name) => path.join(tmpDir, name);
  const png = buildPng(64, 64, (x) => (x < 32 ? [255, 0, 0, 255] : [0, 0, 255, 255]));
  const other = buildPng(64, 64, () => [0, 255, 0, 255]);
  const goodZip = buildZip([
    { name: 'AndroidManifest.xml', data: Buffer.from('<manifest/>') },
    { name: 'res/mipmap-xxxhdpi-v4/ic_launcher.png', data: png },
  ]);
  const badZip = buildZip([
    { name: 'AndroidManifest.xml', data: Buffer.from('<manifest/>') },
    { name: 'res/mipmap-xxxhdpi-v4/ic_launcher.png', data: other },
  ]);
  const noIconZip = buildZip([{ name: 'AndroidManifest.xml', data: Buffer.from('<manifest/>') }]);
  fs.writeFileSync(filePath('source.png'), png);
  fs.writeFileSync(filePath('other.png'), other);
  fs.writeFileSync(filePath('good.apk'), goodZip);
  fs.writeFileSync(filePath('bad.apk'), badZip);
  fs.writeFileSync(filePath('noicon.apk'), noIconZip);
  try {
    const goodDistance = runQuiet(() => assertApkIcon(filePath('good.apk'), filePath('source.png'), filePath('source.png')));
    if (!(goodDistance >= 0 && goodDistance < 1)) fail('自检失败，同图夹具应判近零距离，实测 ' + goodDistance);
    let badDetected = false;
    try {
      runQuiet(() => assertApkIcon(filePath('bad.apk'), filePath('source.png'), filePath('source.png')));
    } catch {
      badDetected = true;
    }
    if (!badDetected) fail('自检失败，异图夹具必须判红但没有判红，判定器不可信');
    let missingDetected = false;
    try {
      runQuiet(() => assertApkIcon(filePath('noicon.apk'), filePath('source.png'), filePath('source.png')));
    } catch {
      missingDetected = true;
    }
    if (!missingDetected) fail('自检失败，缺 ic_launcher 条目的 APK 必须判红但没有判红');
    let splitDetected = false;
    try {
      runQuiet(() => assertApkIcon(filePath('good.apk'), filePath('source.png'), filePath('other.png')));
    } catch {
      splitDetected = true;
    }
    if (!splitDetected) fail('自检失败，源图与 web logo 分家必须判同源断链红');
    info('自检通过 4 条，覆盖同图判绿、异图判红、缺条目判红、源分家判红');
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
}

function buildPng(width, height, pixelAt) {
  const stride = width * 4;
  const rows = [];
  for (let y = 0; y < height; y++) {
    const row = Buffer.alloc(1 + stride);
    row[0] = 0;
    for (let x = 0; x < width; x++) {
      const [r, g, b, a] = pixelAt(x, y);
      row[1 + x * 4] = r;
      row[2 + x * 4] = g;
      row[3 + x * 4] = b;
      row[4 + x * 4] = a;
    }
    rows.push(row);
  }
  const idat = zlib.deflateSync(Buffer.concat(rows));
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  function chunk(type, data) {
    const head = Buffer.alloc(4);
    head.writeUInt32BE(data.length, 0);
    const body = Buffer.concat([Buffer.from(type, 'ascii'), data]);
    const tail = Buffer.alloc(4);
    tail.writeUInt32BE(crc32(body), 0);
    return Buffer.concat([head, body, tail]);
  }
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ihdr), chunk('IDAT', idat), chunk('IEND', Buffer.alloc(0))]);
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
  if (args.length !== 3) {
    fail('用法 assert-apk-icon.mjs <APK路径> <安卓图标源图路径> <web logo 路径> 或 --self-test');
  }
  assertApkIcon(args[0], args[1], args[2]);
}

try {
  main();
} catch (error) {
  // fail 已打印 ::error:: 行，这里只负责以非零码退出，不重复堆栈
  if (!String(error.message).startsWith('图标断链') && !String(error.message).startsWith('消费链断')) {
    console.error('::error::判定器异常 ' + error.message);
  }
  process.exit(1);
}
