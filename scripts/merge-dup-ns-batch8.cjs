// Deep merge duplicate namespaces from group C/D/E JSON into existing namespaces in locales
// 将 dashboard/inventoryBatch/advancedModule/inventoryTransfer 的翻译键合并到已存在命名空间内部
const fs = require('fs');

const GROUPS = [
  '/tmp/i18n-batch8/groupC.json',
  '/tmp/i18n-batch8/groupD.json',
  '/tmp/i18n-batch8/groupE.json',
];

// 需要合并到已存在命名空间的（这些命名空间在 locales 中已存在）
const DUPLICATE_NAMESPACES = ['dashboard', 'inventoryBatch', 'advancedModule', 'inventoryTransfer'];

const mergedZh = {};
const mergedEn = {};

for (const f of GROUPS) {
  const data = JSON.parse(fs.readFileSync(f, 'utf8'));
  const keys = data.keys || {};
  if (keys['zh-CN']) {
    for (const ns of DUPLICATE_NAMESPACES) {
      if (keys['zh-CN'][ns]) {
        if (!mergedZh[ns]) mergedZh[ns] = {};
        deepMerge(mergedZh[ns], keys['zh-CN'][ns]);
      }
    }
  }
  if (keys['en-US']) {
    for (const ns of DUPLICATE_NAMESPACES) {
      if (keys['en-US'][ns]) {
        if (!mergedEn[ns]) mergedEn[ns] = {};
        deepMerge(mergedEn[ns], keys['en-US'][ns]);
      }
    }
  }
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

// 生成要插入到每个命名空间内部的代码片段
// 格式：在命名空间的最后一个 } 之前插入新键
function injectIntoNamespace(filePath, namespace, newKeysObj) {
  let content = fs.readFileSync(filePath, 'utf8');
  // 找到命名空间的起始位置：`  namespace: {`
  const nsPattern = `  ${namespace}: {`;
  const nsStart = content.indexOf(nsPattern);
  if (nsStart < 0) {
    console.log(`  ⚠️ Namespace ${namespace} not found in ${filePath}, skipping`);
    return 0;
  }

  // 找到命名空间的结束位置（匹配的 }）
  // 从 nsStart 开始，找到 `{` 后的位置，追踪括号深度
  let depth = 0;
  let i = nsStart;
  let braceStart = -1;
  for (; i < content.length; i++) {
    if (content[i] === '{') {
      braceStart = i;
      break;
    }
  }
  if (braceStart < 0) return 0;

  depth = 1;
  i = braceStart + 1;
  // 简单括号追踪（locales 文件中字符串不含 { }）
  let inString = false;
  let stringChar = '';
  for (; i < content.length && depth > 0; i++) {
    const c = content[i];
    if (inString) {
      if (c === '\\') { i++; continue; }
      if (c === stringChar) inString = false;
    } else {
      if (c === "'" || c === '"') { inString = true; stringChar = c; }
      else if (c === '{') depth++;
      else if (c === '}') depth--;
    }
  }
  // i-1 是结束 } 的位置
  const nsEnd = i - 1;

  // 在 nsEnd 之前插入新键
  const before = content.slice(0, nsEnd);
  const after = content.slice(nsEnd);

  // 确保 before 末尾有逗号
  const trimmedBefore = before.replace(/\s+$/, '');
  let prefix;
  if (trimmedBefore.endsWith('}')) {
    prefix = trimmedBefore + ',\n';
  } else {
    // 找到最后一行内容，补逗号
    const lastNewline = trimmedBefore.lastIndexOf('\n');
    const lastLine = trimmedBefore.slice(lastNewline + 1);
    if (lastLine.trim() && !lastLine.trim().endsWith(',')) {
      prefix = trimmedBefore + ',\n';
    } else {
      prefix = trimmedBefore + '\n';
    }
  }

  const snippet = serialize(newKeysObj, 4); // 4 空格缩进（命名空间内部）
  const newContent = prefix + '    ' + snippet.split('\n').join('\n    ') + '\n  ' + after;
  fs.writeFileSync(filePath, newContent);
  return newContent.length;
}

console.log('合并重复命名空间翻译键到已存在命名空间内部...\n');

const zhPath = '/workspace/frontend/src/locales/zh-CN.ts';
const enPath = '/workspace/frontend/src/locales/en-US.ts';

for (const ns of DUPLICATE_NAMESPACES) {
  if (mergedZh[ns]) {
    console.log(`zh-CN: 合并 ${ns} (${countKeys(mergedZh[ns])} 键)`);
    injectIntoNamespace(zhPath, ns, mergedZh[ns]);
  }
  if (mergedEn[ns]) {
    console.log(`en-US: 合并 ${ns} (${countKeys(mergedEn[ns])} 键)`);
    injectIntoNamespace(enPath, ns, mergedEn[ns]);
  }
}

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

console.log('\n✓ 完成');
