// Audit i18n keys: scan Vue files for t()/$t() calls and verify keys exist in locales
// Batch 10: components/ 目录 20 个文件
// 用法：node audit-i18n-batch10.cjs

const fs = require('fs');
const path = require('path');

// 加载 locales
const zhPath = '/workspace/frontend/src/locales/zh-CN.ts';
const enPath = '/workspace/frontend/src/locales/en-US.ts';

function loadLocale(filePath) {
  const content = fs.readFileSync(filePath, 'utf8');
  const code = content.replace('export default', 'module.exports =');
  const tmpFile = '/tmp/_locale_tmp_b10.cjs';
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

// Batch 10 扫描范围：components/ + App.vue
const scanFiles = [
  '/workspace/frontend/src/App.vue',
  '/workspace/frontend/src/components/Layout/MainLayout.vue',
  '/workspace/frontend/src/components/AdvancedFilter.vue',
  '/workspace/frontend/src/components/QualityCheck.vue',
  '/workspace/frontend/src/components/AfterSalesPanel.vue',
  '/workspace/frontend/src/components/BatchActions.vue',
  '/workspace/frontend/src/components/PasswordStrengthMeter.vue',
  '/workspace/frontend/src/components/ProcessFlow.vue',
  '/workspace/frontend/src/components/IssueRecordTimeline.vue',
  '/workspace/frontend/src/components/V2Table/index.vue',
  '/workspace/frontend/src/components/ColorCardGrid.vue',
  '/workspace/frontend/src/components/ai/AiPredictionChart.vue',
  '/workspace/frontend/src/components/ColorCardIssueForm.vue',
  '/workspace/frontend/src/components/Charts/BaseChart.vue',
  '/workspace/frontend/src/components/Charts/BarChart.vue',
  '/workspace/frontend/src/components/Charts/PieChart.vue',
  '/workspace/frontend/src/components/Charts/LineChart.vue',
  '/workspace/frontend/src/components/ColorItemEditor.vue',
  '/workspace/frontend/src/components/PriceHistoryChart.vue',
  '/workspace/frontend/src/components/ColorCardIssueDetail.vue',
];

const vueFiles = scanFiles.filter(f => fs.existsSync(f));
console.log(`Scanning ${vueFiles.length} Vue files...\n`);

const tCallRegex = /\bt\(\s*['"]([^'"]+)['"]/g;
const dollarTCallRegex = /\$t\(\s*['"]([^'"]+)['"]/g;

const missingKeys = new Map();
let totalCalls = 0;
const allKeysUsed = new Set();

for (const file of vueFiles) {
  const content = fs.readFileSync(file, 'utf8');
  const relFile = path.relative('/workspace/frontend/src', file);

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

try { fs.unlinkSync('/tmp/_locale_tmp_b10.cjs'); } catch (e) { /* ignore */ }
