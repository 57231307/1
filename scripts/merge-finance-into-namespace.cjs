// Merge groupC.json (finance) into the first finance namespace in zh-CN.ts and en-US.ts
// 策略：在第一个 finance 命名空间的 '  },' 结束行前插入 Group C 的翻译键
const fs = require('fs');

const GROUPS = ['/tmp/i18n-batch6/groupC.json'];

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

function serialize(obj, indent = 4) {
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

const zhSnippet = serialize(zhObj.finance || {}, 4);
const enSnippet = serialize(enObj.finance || {}, 4);

function injectIntoFinance(filePath, snippet) {
  let content = fs.readFileSync(filePath, 'utf8');
  // 找到第一个 "  finance: {" 的位置
  const financeStart = content.indexOf('\n  finance: {');
  if (financeStart < 0) throw new Error(`Cannot find 'finance: {' in ${filePath}`);
  
  // 从 financeStart 开始，找到匹配的 "  }," 结束行
  let depth = 1;
  let pos = financeStart + '\n  finance: {'.length;
  while (pos < content.length && depth > 0) {
    const ch = content[pos];
    if (ch === '{') depth++;
    else if (ch === '}') depth--;
    if (depth === 0) break;
    pos++;
  }
  // pos 现在指向 finance 块的结束 '}'
  // 找到 '}' 后的 ',' 或 '\n'
  const insertPos = pos;
  // 在 '}' 前插入 snippet
  const before = content.slice(0, insertPos);
  const after = content.slice(insertPos);
  
  // 确保 before 末尾有逗号和换行
  const trimmedBefore = before.replace(/\s+$/, '');
  let prefix;
  if (trimmedBefore.endsWith(',') || trimmedBefore.endsWith('{')) {
    prefix = trimmedBefore + '\n';
  } else {
    prefix = trimmedBefore + ',\n';
  }
  
  const newContent = prefix + snippet + '\n  ' + after;
  fs.writeFileSync(filePath, newContent);
  return newContent.length;
}

const zhPath = '/workspace/frontend/src/locales/zh-CN.ts';
const enPath = '/workspace/frontend/src/locales/en-US.ts';

const zhLen = injectIntoFinance(zhPath, zhSnippet);
const enLen = injectIntoFinance(enPath, enSnippet);

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
console.log(`\n新增 finance 翻译键：${countKeys(zhObj.finance || {})} 键`);
