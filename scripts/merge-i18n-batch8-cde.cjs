// Merge i18n group{C,D,E}.json into zh-CN.ts and en-US.ts (deep merge)
// Batch 8 Group C/D/E: 深度合并到已存在命名空间 + 追加新命名空间
// 用法：node scripts/merge-i18n-batch8-cde.cjs

const fs = require('fs');

const GROUPS = [
  '/tmp/i18n-batch8/groupC.json',
  '/tmp/i18n-batch8/groupD.json',
  '/tmp/i18n-batch8/groupE.json',
];

// 分别合并 zh 和 en
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
  for (const k of keys) {
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

const zhSnippet = serialize(mergedZh, 2);
const enSnippet = serialize(mergedEn, 2);

const zhLen = inject(zhPath, zhSnippet);
const enLen = inject(enPath, enSnippet);

console.log(`✓ zh-CN.ts updated (${zhLen} chars)`);
console.log(`✓ en-US.ts updated (${enLen} chars)`);

function countKeys(obj) {
  let count = 0;
  function walk(o) {
    for (const k of Object.keys(o)) {
      if (o[k] && typeof o[k] === 'object') walk(o[k]);
      else count++;
    }
  }
  walk(obj);
  return count;
}

console.log(`\n新增翻译键统计：`);
console.log(`  - zh 命名空间：${countKeys(mergedZh)} 键`);
console.log(`  - en 命名空间：${countKeys(mergedEn)} 键`);
console.log(`\n顶层命名空间：`);
for (const k of Object.keys(mergedZh)) {
  console.log(`  - ${k}: ${countKeys(mergedZh[k])} 键`);
}
