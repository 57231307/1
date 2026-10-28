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
//        同名文件字节同一。已实测 aapt2 对 PNG 条目原样存储不做二次编码，
//        模板默认图与生成图字节不符，比对即可判消费与否。
//        debug 包保留原名条目，实测条目形如 res/mipmap-xxxhdpi-v4/ic_launcher.png；
//        release 包由打包器改写资源存放路径，原名条目整体缺席（本仓 CI 日志实测
//        同一份注入产物 debug 判绿 release 报条目不存在）。故段二分两形态，原名条目
//        存在时只与它比字节，不存在时对包内全部 res 下 PNG 条目逐个比字节，命中即通过
//        并打出实际条目名，两形态都没命中同一字节即判红，不降级为警告。
//   段三 存在性，生成 mipmap 的字节必须以某个 res PNG 条目的形态存在于 APK 内，
//        缺失即判红。
// 传给谁 失败消息点名文件与实测字节数，并附包内 res PNG 条目数量与名字样例，供 CI 日志直接定位。
// 存哪里 只读判定，不落盘任何文件。
//
// 自检模式在内存里合成五枚 APK 夹具，图标与生成 mipmap 同字节应判绿，
// 图标被替换与条目缺失都应判红，路径被改写且字节同一应判绿，路径被改写且无同字节条目应判红，
// 六个判定方向都验才算判定器自身可信。

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
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

// ---------- 判定主流程 ----------

const XXXHDPI_ICON = /^res\/mipmap-xxxhdpi[^/]*\/ic_launcher\.png$/;

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
    info('消费链通过，APK 条目 ' + direct.name + ' 与生成 mipmap 字节同一，共 ' + data.length + ' 字节');
    return;
  }

  const resPngs = entries.filter((entry) => entry.name.startsWith('res/') && entry.name.endsWith('.png'));
  for (const entry of resPngs) {
    const data = readEntryData(apkBuffer, entry, apkPath);
    if (data.equals(generatedBuffer)) {
      info(
        '消费链通过，APK 条目 ' + entry.name +
          ' 与生成 mipmap 字节同一，共 ' + data.length +
          ' 字节，原名条目缺席说明打包器改写了资源路径，按字节同一兜底命中'
      );
      return;
    }
  }

  const sampleNames = resPngs.slice(0, 12).map((entry) => entry.name).join('，');
  fail(
    '消费链断，APK 内未找到 res/mipmap-xxxhdpi 的 ic_launcher.png 条目，' +
      'res 下 ' + resPngs.length + ' 个 PNG 条目内也没有与生成步产物 ' + generatedMipmapPath +
      ' 共 ' + generatedBuffer.length + ' 字节字节同一的条目，安卓图标未进包，条目样例 ' +
      (sampleNames || '无')
  );
}

// ---------- 自检，内存合成夹具验判定器双向可信，夹具只落系统临时目录 ----------

function selfTest() {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'apk-icon-selftest-'));
  const filePath = (name) => path.join(tmpDir, name);
  const source = Buffer.from('BRAND-MASTER-ICON-BYTES');
  const other = Buffer.from('TEMPLATE-DEFAULT-ICON-BYTES');
  const zipWith = (iconBytes) => buildZip([
    { name: 'AndroidManifest.xml', data: Buffer.from('<manifest/>') },
    { name: 'res/mipmap-xxxhdpi-v4/ic_launcher.png', data: iconBytes },
  ]);
  const zipShortened = (iconBytes) => buildZip([
    { name: 'AndroidManifest.xml', data: Buffer.from('<manifest/>') },
    { name: 'res/aC.png', data: other },
    { name: 'res/aD.png', data: iconBytes },
  ]);
  fs.writeFileSync(filePath('source.png'), source);
  fs.writeFileSync(filePath('good.apk'), zipWith(source));
  fs.writeFileSync(filePath('bad.apk'), zipWith(other));
  fs.writeFileSync(filePath('noicon.apk'), buildZip([{ name: 'AndroidManifest.xml', data: Buffer.from('<manifest/>') }]));
  fs.writeFileSync(filePath('shortgood.apk'), zipShortened(source));
  fs.writeFileSync(filePath('shortbad.apk'), zipShortened(Buffer.from('OTHER-RES-ID-DIFFERENT-BYTES')));
  try {
    runQuiet(() => assertApkIcon(filePath('good.apk'), filePath('source.png'), filePath('source.png'), filePath('source.png')));
    let badDetected = false;
    try {
      runQuiet(() => assertApkIcon(filePath('bad.apk'), filePath('source.png'), filePath('source.png'), filePath('source.png')));
    } catch {
      badDetected = true;
    }
    if (!badDetected) fail('自检失败，包内图标与生成产物不同字节必须判红，判定器不可信');
    let missingDetected = false;
    try {
      runQuiet(() => assertApkIcon(filePath('noicon.apk'), filePath('source.png'), filePath('source.png'), filePath('source.png')));
    } catch {
      missingDetected = true;
    }
    if (!missingDetected) fail('自检失败，缺 ic_launcher 条目的 APK 必须判红但没有判红');
    let splitDetected = false;
    try {
      runQuiet(() => assertApkIcon(filePath('good.apk'), filePath('source.png'), filePath('bad.apk'), filePath('source.png')));
    } catch {
      splitDetected = true;
    }
    if (!splitDetected) fail('自检失败，源图与 web logo 分家必须判同源断链红');
    runQuiet(() => assertApkIcon(filePath('shortgood.apk'), filePath('source.png'), filePath('source.png'), filePath('source.png')));
    let shortBadDetected = false;
    try {
      runQuiet(() => assertApkIcon(filePath('shortbad.apk'), filePath('source.png'), filePath('source.png'), filePath('source.png')));
    } catch {
      shortBadDetected = true;
    }
    if (!shortBadDetected) fail('自检失败，路径被改写且包内无同字节条目的 APK 必须判红但没有判红');
    info('自检通过 6 条，覆盖同字节判绿、异字节判红、缺条目判红、源分家判红、路径改写同字节判绿、路径改写无同字节判红');
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
