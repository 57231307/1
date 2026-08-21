// Audit i18n: 扫描 Vue 文件中的 t()/$t() 调用，验证翻译键是否存在于 zh-CN.ts
const fs = require('fs');
const path = require('path');
const vm = require('vm');

// 加载 zh-CN.ts（通过 vm 执行 export default）
function loadLocale(filePath) {
  const content = fs.readFileSync(filePath, 'utf8');
  // 移除 export default，改为变量赋值
  const code = content.replace(/export\s+default\s+/, 'var __locale = ');
  const sandbox = {};
  vm.createContext(sandbox);
  vm.runInContext(code, sandbox);
  return sandbox.__locale;
}

const locale = loadLocale('/workspace/frontend/src/locales/zh-CN.ts');

// 收集所有存在的键路径
function collectKeys(obj, prefix, keys) {
  for (const k of Object.keys(obj)) {
    const v = obj[k];
    const path = prefix ? `${prefix}.${k}` : k;
    if (v && typeof v === 'object') {
      collectKeys(v, path, keys);
    } else {
      keys.add(path);
    }
  }
}
const existingKeys = new Set();
collectKeys(locale, '', existingKeys);
console.log(`zh-CN.ts 共 ${existingKeys.size} 个翻译键`);

// 检查键是否存在
function keyExists(keyPath) {
  // 直接查找
  if (existingKeys.has(keyPath)) return true;
  // 尝试模板键（含 ${...} 或 {var}）
  // 检查是否是某个父对象的子键
  const parts = keyPath.split('.');
  let cur = locale;
  for (const p of parts) {
    if (cur && typeof cur === 'object' && p in cur) {
      cur = cur[p];
    } else {
      return false;
    }
  }
  return cur !== undefined && typeof cur !== 'object';
}

// 扫描 Vue 文件中的 t()/$t() 调用
const targetDirs = [
  '/workspace/frontend/src/views/scheduling',
  '/workspace/frontend/src/views/security',
  '/workspace/frontend/src/views/system',
];

function walkDir(dir, files) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) walkDir(full, files);
    else if (entry.name.endsWith('.vue')) files.push(full);
  }
}

const vueFiles = [];
for (const d of targetDirs) walkDir(d, vueFiles);

const missingKeys = new Map(); // key -> [files]
let totalCalls = 0;

// 匹配 t('...') 或 t("...") 或 $t('...') 等
const callRegex = /\bt\(\s*['"`]([^'"`]+)['"`]/g;
const dollarCallRegex = /\$t\(\s*['"`]([^'"`]+)['"`]/g;

for (const f of vueFiles) {
  const content = fs.readFileSync(f, 'utf8');
  // 跳过未接入 useI18n 的文件
  if (!content.includes('useI18n') && !content.includes('$t(')) continue;

  const basename = path.relative('/workspace/frontend/src/views', f);

  for (const regex of [callRegex, dollarCallRegex]) {
    let m;
    while ((m = regex.exec(content)) !== null) {
      const key = m[1];
      // 跳过动态键（含 ${} 或变量拼接）
      if (key.includes('${') || key.includes('{') && !key.match(/^[a-zA-Z0-9_.]+$/)) {
        // 含 {var} 是参数化翻译，键本身是字面量，继续检查
        // 但 ${var} 是模板字符串拼接，跳过
        if (key.includes('${')) continue;
      }
      totalCalls++;
      if (!keyExists(key)) {
        if (!missingKeys.has(key)) missingKeys.set(key, []);
        missingKeys.get(key).push(basename);
      }
    }
  }
}

console.log(`\n扫描 ${vueFiles.length} 个 Vue 文件，共 ${totalCalls} 个 t()/$t() 调用`);
console.log(`缺失键：${missingKeys.size} 个`);

if (missingKeys.size > 0) {
  console.log('\n缺失键列表：');
  for (const [key, files] of missingKeys) {
    console.log(`  - ${key} (in ${files.join(', ')})`);
  }
  process.exit(1);
} else {
  console.log('\n✅ 所有翻译键均存在，无缺失');
}
