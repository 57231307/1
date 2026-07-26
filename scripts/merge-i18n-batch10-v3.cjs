// Merge i18n group*.json into zh-CN.ts and en-US.ts (v3 改进版)
// 使用 Node.js require 加载原文件，深度合并新键后写回
// 正确处理字符串中的 \n、'、\\ 等转义字符

const fs = require('fs');

const GROUPS = [
  '/tmp/i18n-batch10/groupA.json',
  '/tmp/i18n-batch10/groupB.json',
  '/tmp/i18n-batch10/groupC.json',
  '/tmp/i18n-batch10/groupD.json',
];

function deepMerge(target, source) {
  for (const k of Object.keys(source)) {
    const v = source[k];
    if (v && typeof v === 'object' && !Array.isArray(v)) {
      if (!target[k] || typeof target[k] !== 'object') target[k] = {};
      deepMerge(target[k], v);
    } else if (typeof v === 'string') {
      target[k] = v;
    }
  }
}

function loadLocale(filePath) {
  const content = fs.readFileSync(filePath, 'utf8');
  const code = content.replace('export default', 'module.exports =');
  const tmpFile = '/tmp/_locale_merge_v3.cjs';
  fs.writeFileSync(tmpFile, code);
  delete require.cache[require.resolve(tmpFile)];
  return require(tmpFile);
}

function serialize(obj, indent = 2) {
  const pad = ' '.repeat(indent);
  const lines = [];
  const keys = Object.keys(obj);
  for (let i = 0; i < keys.length; i++) {
    const k = keys[i];
    const v = obj[k];
    // 键名转义：如果不是合法 JS 标识符（如以数字开头、含特殊字符），用单引号包裹
    const isIdent = /^[a-zA-Z_$][a-zA-Z0-9_$]*$/.test(k);
    const kStr = isIdent ? k : `'${k}'`;
    if (v && typeof v === 'object') {
      lines.push(`${pad}${kStr}: {`);
      lines.push(serialize(v, indent + 2));
      lines.push(`${pad}},`);
    } else {
      // 正确转义字符串
      let s = String(v);
      s = s.replace(/\\/g, '\\\\');  // \ -> \\
      s = s.replace(/'/g, "\\'");    // ' -> \'
      s = s.replace(/\n/g, '\\n');   // 真正换行 -> \n 字面量
      s = s.replace(/\r/g, '\\r');   // 真正回车 -> \r 字面量
      s = s.replace(/\t/g, '\\t');   // 真正 tab -> \t 字面量
      lines.push(`${pad}${kStr}: '${s}',`);
    }
  }
  return lines.join('\n');
}

function writeLocale(filePath, obj) {
  const content = fs.readFileSync(filePath, 'utf8');
  // 找到 export default { 的位置
  const m = content.match(/export\s+default\s*\{/);
  if (!m) throw new Error(`Cannot find 'export default {' in ${filePath}`);
  
  const headerEnd = m.index + m[0].length;
  const header = content.slice(0, headerEnd);
  
  // 序列化对象
  const body = serialize(obj, 2);
  
  // 写回文件
  const newContent = header + '\n' + body + '\n};\n';
  fs.writeFileSync(filePath, newContent);
}

// 加载所有 group 文件并合并
const mergedZh = {};
const mergedEn = {};

for (const f of GROUPS) {
  const data = JSON.parse(fs.readFileSync(f, 'utf8'));
  const keys = data.keys || {};
  if (keys['zh-CN']) deepMerge(mergedZh, keys['zh-CN']);
  if (keys['en-US']) deepMerge(mergedEn, keys['en-US']);
}

console.log(`合并后顶层 zh-CN 命名空间: ${Object.keys(mergedZh).join(', ')}`);
console.log(`合并后顶层 en-US 命名空间: ${Object.keys(mergedEn).join(', ')}`);

function countLeaves(obj) {
  let cnt = 0;
  for (const v of Object.values(obj)) {
    if (v && typeof v === 'object') cnt += countLeaves(v);
    else cnt += 1;
  }
  return cnt;
}

console.log(`zh-CN 新增翻译键数: ${countLeaves(mergedZh)}`);
console.log(`en-US 新增翻译键数: ${countLeaves(mergedEn)}`);

// 加载现有 locales 文件
console.log('\n加载 zh-CN.ts...');
const zhObj = loadLocale('/workspace/frontend/src/locales/zh-CN.ts');
console.log(`  zh-CN 原有顶层命名空间: ${Object.keys(zhObj).length} 个, ${countLeaves(zhObj)} 个翻译键`);

console.log('加载 en-US.ts...');
const enObj = loadLocale('/workspace/frontend/src/locales/en-US.ts');
console.log(`  en-US 原有顶层命名空间: ${Object.keys(enObj).length} 个, ${countLeaves(enObj)} 个翻译键`);

// 深度合并新键到现有对象
console.log('\n深度合并新键到 zh-CN...');
const beforeZh = countLeaves(zhObj);
deepMerge(zhObj, mergedZh);
const afterZh = countLeaves(zhObj);
console.log(`  zh-CN: ${beforeZh} -> ${afterZh} (+${afterZh - beforeZh})`);

console.log('深度合并新键到 en-US...');
const beforeEn = countLeaves(enObj);
deepMerge(enObj, mergedEn);
const afterEn = countLeaves(enObj);
console.log(`  en-US: ${beforeEn} -> ${afterEn} (+${afterEn - beforeEn})`);

// 写回文件
console.log('\n写回 zh-CN.ts...');
writeLocale('/workspace/frontend/src/locales/zh-CN.ts', zhObj);
console.log('写回 en-US.ts...');
writeLocale('/workspace/frontend/src/locales/en-US.ts', enObj);

console.log('\n✅ 合并完成');

// 清理临时文件
try { fs.unlinkSync('/tmp/_locale_merge_v3.cjs'); } catch (e) { /* ignore */ }
