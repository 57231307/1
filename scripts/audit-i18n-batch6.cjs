// Audit i18n keys: scan Vue files for t()/$t() calls and verify keys exist in locales
// 用法：node audit-i18n-batch6.cjs

const fs = require('fs');
const path = require('path');

// 加载 locales
const zhPath = '/workspace/frontend/src/locales/zh-CN.ts';
const enPath = '/workspace/frontend/src/locales/en-US.ts';

function loadLocale(filePath) {
  const content = fs.readFileSync(filePath, 'utf8');
  const code = content.replace('export default', 'module.exports =');
  const tmpFile = '/tmp/_locale_tmp.cjs';
  fs.writeFileSync(tmpFile, code);
  delete require.cache[require.resolve(tmpFile)];
  return require(tmpFile);
}

const zhLocale = loadLocale(zhPath);
const enLocale = loadLocale(enPath);

// 收集所有存在的键
const existingKeys = new Set();
function collectKeys(obj, prefix = '') {
  for (const k of Object.keys(obj)) {
    const v = obj[k];
    const keyPath = prefix ? `${prefix}.${k}` : k;
    if (v && typeof v === 'object' && !Array.isArray(v)) {
      collectKeys(v, keyPath);
    } else {
      existingKeys.add(keyPath);
    }
  }
}
collectKeys(zhLocale);
console.log(`Loaded ${existingKeys.size} keys from zh-CN.ts`);

function keyExists(keyPath) {
  if (existingKeys.has(keyPath)) return true;
  const parts = keyPath.split('.');
  let cur = zhLocale;
  for (const p of parts) {
    if (cur && typeof cur === 'object' && p in cur) {
      cur = cur[p];
    } else {
      return false;
    }
  }
  return cur !== undefined && typeof cur !== 'object';
}

// 扫描 Batch 6 目录
const dirs = [
  '/workspace/frontend/src/views/api-gateway',
  '/workspace/frontend/src/views/bpm/approval',
  '/workspace/frontend/src/views/bpm/definitions',
  '/workspace/frontend/src/views/fabric',
  '/workspace/frontend/src/views/finance',
  '/workspace/frontend/src/views/system-update',
];

function findVueFiles(dir) {
  const results = [];
  const items = fs.readdirSync(dir, { withFileTypes: true });
  for (const item of items) {
    const fullPath = path.join(dir, item.name);
    if (item.isDirectory()) {
      results.push(...findVueFiles(fullPath));
    } else if (item.name.endsWith('.vue')) {
      results.push(fullPath);
    }
  }
  return results;
}

const vueFiles = [];
for (const dir of dirs) {
  if (fs.existsSync(dir)) {
    vueFiles.push(...findVueFiles(dir));
  }
}

console.log(`Scanning ${vueFiles.length} Vue files...\n`);

const tCallRegex = /\bt\(\s*['"]([^'"]+)['"]/g;
const dollarTCallRegex = /\$t\(\s*['"]([^'"]+)['"]/g;

const missingKeys = new Map();
let totalCalls = 0;
const allKeysUsed = new Set();

for (const file of vueFiles) {
  const content = fs.readFileSync(file, 'utf8');
  const relFile = path.relative('/workspace/frontend/src/views', file);

  let match;
  while ((match = tCallRegex.exec(content)) !== null) {
    const key = match[1];
    totalCalls++;
    allKeysUsed.add(key);
    if (!keyExists(key)) {
      if (!missingKeys.has(key)) missingKeys.set(key, []);
      missingKeys.get(key).push(relFile);
    }
  }
  while ((match = dollarTCallRegex.exec(content)) !== null) {
    const key = match[1];
    totalCalls++;
    allKeysUsed.add(key);
    if (!keyExists(key)) {
      if (!missingKeys.has(key)) missingKeys.set(key, []);
      missingKeys.get(key).push(relFile);
    }
  }
}

console.log(`Total t()/$t() calls: ${totalCalls}`);
console.log(`Unique keys referenced: ${allKeysUsed.size}`);
console.log(`Missing keys: ${missingKeys.size}`);

if (missingKeys.size > 0) {
  console.log('\n❌ Missing keys:');
  for (const [key, files] of missingKeys) {
    console.log(`  - ${key} (in ${files.join(', ')})`);
  }
  process.exit(1);
} else {
  console.log('\n✅ All keys exist in locales!');
}

try { fs.unlinkSync('/tmp/_locale_tmp.cjs'); } catch (e) { /* ignore */ }
