// Merge i18n group*.json into zh-CN.ts and en-US.ts (deep merge)
// Batch 10: 从 keys.zh-CN 和 keys.en-US 字段提取翻译键
// 复用 batch7 的逗号修复逻辑：插入新命名空间前确保前一个属性末尾有逗号

const fs = require('fs');

const GROUPS = [
  '/tmp/i18n-batch10/groupA.json',
  '/tmp/i18n-batch10/groupB.json',
  '/tmp/i18n-batch10/groupC.json',
  '/tmp/i18n-batch10/groupD.json',
];

const mergedZh = {};
const mergedEn = {};

for (const f of GROUPS) {
  const data = JSON.parse(fs.readFileSync(f, 'utf8'));
  const keys = data.keys || {};
  if (keys['zh-CN']) deepMerge(mergedZh, keys['zh-CN']);
  if (keys['en-US']) deepMerge(mergedEn, keys['en-US']);
}

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

function serialize(obj, indent = 2) {
  const pad = ' '.repeat(indent);
  const lines = [];
  const keys = Object.keys(obj);
  for (let i = 0; i < keys.length; i++) {
    const k = keys[i];
    const v = obj[k];
    if (v && typeof v === 'object') {
      lines.push(`${pad}${k}: {`);
      lines.push(serialize(v, indent + 2));
      lines.push(`${pad}},`);
    } else {
      const s = String(v).replace(/\\/g, '\\\\').replace(/'/g, "\\'");
      lines.push(`${pad}${k}: '${s}',`);
    }
  }
  return lines.join('\n');
}

const zhSnippet = serialize(mergedZh, 2);
const enSnippet = serialize(mergedEn, 2);

function inject(filePath, snippet) {
  let content = fs.readFileSync(filePath, 'utf8');
  const lastBrace = content.lastIndexOf('\n};');
  if (lastBrace < 0) throw new Error(`Cannot find terminating '};' in ${filePath}`);
  const before = content.slice(0, lastBrace);
  const after = content.slice(lastBrace);
  const trimmedBefore = before.replace(/\s+$/, '');
  let prefix;
  if (trimmedBefore.endsWith('}')) {
    prefix = trimmedBefore + ',\n';
  } else {
    prefix = before.replace(/\s+$/, '\n');
  }
  const newContent = prefix + snippet + '\n' + after;
  fs.writeFileSync(filePath, newContent);
  return newContent.length;
}

const zhPath = '/workspace/frontend/src/locales/zh-CN.ts';
const enPath = '/workspace/frontend/src/locales/en-US.ts';

const zhBefore = fs.statSync(zhPath).size;
const enBefore = fs.statSync(enPath).size;

const zhAfter = inject(zhPath, zhSnippet);
const enAfter = inject(enPath, enSnippet);

console.log(`zh-CN.ts: ${zhBefore} -> ${zhAfter} bytes (+${zhAfter - zhBefore})`);
console.log(`en-US.ts: ${enBefore} -> ${enAfter} bytes (+${enAfter - enBefore})`);
console.log(`Merged ${GROUPS.length} group files`);
console.log(`Top-level zh-CN keys: ${Object.keys(mergedZh).length}`);
console.log(`Top-level en-US keys: ${Object.keys(mergedEn).length}`);
