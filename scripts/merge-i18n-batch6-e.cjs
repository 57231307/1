// Merge groupE.json (bpm.definitions) into zh-CN.ts and en-US.ts
const fs = require('fs');

const GROUPS = [
  '/tmp/i18n-batch6/groupE.json',
];

const merged = {};
for (const f of GROUPS) {
  const data = JSON.parse(fs.readFileSync(f, 'utf8'));
  deepMerge(merged, data);
}

function deepMerge(target, source) {
  for (const k of Object.keys(source)) {
    const v = source[k];
    if (v && typeof v === 'object' && !Array.isArray(v)) {
      if (v.zhCN !== undefined || v['zh-CN'] !== undefined) {
        target[k] = { zhCN: v['zh-CN'] ?? v.zhCN, enUS: v['en-US'] ?? v.enUS };
      } else {
        if (!target[k] || typeof target[k] !== 'object') target[k] = {};
        deepMerge(target[k], v);
      }
    }
  }
}

function buildLocale(merged, locale) {
  const result = {};
  function walk(obj, target) {
    for (const k of Object.keys(obj)) {
      const v = obj[k];
      if (v && typeof v === 'object' && v.zhCN !== undefined) {
        target[k] = locale === 'zh' ? v.zhCN : v.enUS;
      } else if (v && typeof v === 'object') {
        target[k] = {};
        walk(v, target[k]);
      }
    }
  }
  walk(merged, result);
  return result;
}

const zhObj = buildLocale(merged, 'zh');
const enObj = buildLocale(merged, 'en');

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

const zhSnippet = serialize(zhObj, 2);
const enSnippet = serialize(enObj, 2);

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
console.log(`  - zh 命名空间：${countKeys(zhObj)} 键`);
console.log(`  - en 命名空间：${countKeys(enObj)} 键`);
