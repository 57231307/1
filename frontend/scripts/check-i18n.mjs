#!/usr/bin/env node
/**
 * i18n 缺失 key 检测（用户报障："还有好多地方显示英文"）
 *
 * 扫描 .vue/.ts 中 t('a.b.c') / $t('a.b.c') 调用的 key，
 * 对照 zh-CN.ts 嵌套结构，缺失即列出。
 * 首轮产出缺失清单（ALLOWLIST 挂账），新增缺失即 CI fail。
 */
import { readFileSync, readdirSync } from 'fs';
import { join, dirname, resolve } from 'path';
import { fileURLToPath } from 'url';
import { execSync } from 'child_process';

const __dirname = dirname(fileURLToPath(import.meta.url));
const FRONTEND = resolve(__dirname, '..');

// 读取 zh-CN.ts（TS 源码，用 ts-node 代价高——转成可 eval 的 JSON 不可行，改用 babel 简化：
// 直接正则抓 key 层级不可靠。此处借 vite 环境缺失，采用运行时 import 转换：
// zh-CN.ts 是 `export default { ... }` 形式，用正则剥掉 export 头尾后 eval）
function loadZhCN() {
  const src = readFileSync(join(FRONTEND, 'src', 'locales', 'zh-CN.ts'), 'utf-8');
  // 仅剥离真正的 import 语句（含 from），避免误吞对象键 import: '...'
  const stripped = src
    .replace(/^\s*import\s+[^;\n]*\bfrom\s+['"][^'"]*['"];?\s*$/gm, '')
    .replace(/\bexport\s+default\s*/, 'module.exports = ')
    .replace(/\s*as const\s*(?=[;}])/g, '')
    .replace(/\s*satisfies\s+\w+/g, '');
  const fn = new Function('module', 'exports', stripped);
  const mod = { exports: {} };
  fn(mod, mod.exports);
  return mod.exports;
}

function hasKey(obj, keyPath) {
  let cur = obj;
  for (const part of keyPath.split('.')) {
    if (cur === undefined || cur === null || typeof cur !== 'object') return false;
    cur = cur[part];
  }
  return cur !== undefined;
}

// 扫描 src 下所有 vue/ts 的 t() 调用
function collectKeys(dir, set) {
  for (const f of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, f.name);
    if (f.isDirectory()) {
      if (f.name === 'node_modules' || f.name === 'dist') continue;
      collectKeys(p, set);
    } else if (/\.(vue|ts)$/.test(f.name) && !/locales/.test(p)) {
      const src = readFileSync(p, 'utf-8');
      for (const m of src.matchAll(/[$\s.(]t\(\s*'([^']+)'/g)) set.add(m[1]);
      for (const m of src.matchAll(/[$\s.(]t\(\s*"([^"]+)"/g)) set.add(m[1]);
    }
  }
}

const zh = loadZhCN();
const used = new Set();
collectKeys(join(FRONTEND, 'src'), used);

// 存量挂账清单：修复一条删一条
const ALLOWLIST = new Set();
const missing = [...used].filter(k => !hasKey(zh, k) && !k.includes('${') && !k.includes('$'));
const realMissing = missing.filter(k => !ALLOWLIST.has(k));

console.log(
  `[i18n] 使用 key 总数: ${used.size}, 缺失: ${realMissing.length}, 挂账: ${missing.length - realMissing.length}`
);
if (realMissing.length > 0) {
  console.error(
    `[i18n] ❌ 新增缺失 key（将显示英文原始 key）:\n${realMissing.map(k => '  - ' + k).join('\n')}`
  );
  process.exit(1);
}
