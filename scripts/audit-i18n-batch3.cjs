// Audit i18n: scan t()/$t() calls in target Vue files and verify keys exist in zh-CN.ts
const fs = require('fs');
const path = require('path');

const TARGETS = [
  '/workspace/frontend/src/views/customer/tabs/CustomerFormTab.vue',
  '/workspace/frontend/src/views/customer/index.vue',
  '/workspace/frontend/src/views/customerCredit/tabs/AdjustDialogTab.vue',
  '/workspace/frontend/src/views/customerCredit/tabs/AmountDialogTab.vue',
  '/workspace/frontend/src/views/customerCredit/tabs/RatingDialogTab.vue',
  '/workspace/frontend/src/views/customerCredit/index.vue',
  '/workspace/frontend/src/views/supplier/SupplierDialog.vue',
  '/workspace/frontend/src/views/supplier/SupplierList.vue',
  '/workspace/frontend/src/views/supplier/index.vue',
  '/workspace/frontend/src/views/supplierEvaluation/index.vue',
  '/workspace/frontend/src/views/quotations/components/ApprovalProgress.vue',
  '/workspace/frontend/src/views/quotations/components/QuotationItemEditor.vue',
  '/workspace/frontend/src/views/quotations/components/TermEditor.vue',
  '/workspace/frontend/src/views/quotations/approval.vue',
  '/workspace/frontend/src/views/quotations/create.vue',
  '/workspace/frontend/src/views/quotations/detail.vue',
  '/workspace/frontend/src/views/quotations/list.vue',
];

// 1. 解析 zh-CN.ts，提取所有键路径
const zhContent = fs.readFileSync('/workspace/frontend/src/locales/zh-CN.ts', 'utf8');
const keys = new Set();

// 提取每行形如 `  key: 'value'` 的 key，并按缩进构建路径
const lines = zhContent.split('\n');
const stack = []; // [{indent, key}]
for (const line of lines) {
  const m = line.match(/^(\s*)([a-zA-Z_][\w]*):\s*'[^']*'(?:,)?\s*$/);
  if (m) {
    const indent = m[1].length;
    const key = m[2];
    // 弹出栈顶 indent >= 当前
    while (stack.length && stack[stack.length - 1].indent >= indent) stack.pop();
    stack.push({ indent, key });
    const fullKey = stack.map(s => s.key).join('.');
    keys.add(fullKey);
    continue;
  }
  const m2 = line.match(/^(\s*)([a-zA-Z_][\w]*):\s*\{/);
  if (m2) {
    const indent = m2[1].length;
    const key = m2[2];
    while (stack.length && stack[stack.length - 1].indent >= indent) stack.pop();
    stack.push({ indent, key });
  }
  // 行 `},` 表示栈顶出栈
  if (/^\s*\},?\s*$/.test(line) && stack.length) {
    // 仅在非叶子行后才弹出；这里简化：pop 当前 indent 的栈顶
    // 实际叶子行 `  key: 'v',` 不会推入对象，所以这里 pop 是安全的
    // 但叶子行后面没有 `},`，所以不会误 pop
    // 注意：`},` 出现意味着上一个非叶子对象结束
    // 为避免误 pop 叶子，我们改在下次迭代时通过 indent 比较 pop
    // 此处不 pop，靠 indent 自动管理
  }
}

console.log(`已加载 zh-CN.ts 翻译键：${keys.size} 个`);

// 2. 扫描每个 Vue 文件的 t('key') / $t('key') 调用
const missing = [];
let totalCalls = 0;
const usedKeys = new Set();

for (const f of TARGETS) {
  if (!fs.existsSync(f)) {
    console.log(`⚠ 文件不存在: ${f}`);
    continue;
  }
  const content = fs.readFileSync(f, 'utf8');
  // 匹配 t('...') 和 $t('...')，忽略 t(`...`) 模板字符串
  const re = /\bt\(\s*'([a-zA-Z_][\w.]*)'/g;
  const reDollar = /\$t\(\s*'([a-zA-Z_][\w.]*)'/g;
  let m;
  while ((m = re.exec(content)) !== null) {
    totalCalls++;
    usedKeys.add(m[1]);
    if (!keys.has(m[1])) {
      missing.push({ file: f, key: m[1], type: 't()' });
    }
  }
  while ((m = reDollar.exec(content)) !== null) {
    totalCalls++;
    usedKeys.add(m[1]);
    if (!keys.has(m[1])) {
      missing.push({ file: f, key: m[1], type: '$t()' });
    }
  }
}

console.log(`\n扫描完成：${totalCalls} 个 t()/$t() 调用，引用 ${usedKeys.size} 个不同键\n`);

if (missing.length === 0) {
  console.log('✓ 无缺失键，全部翻译键存在');
} else {
  console.log(`✗ 发现 ${missing.length} 个缺失键：`);
  const byFile = {};
  for (const m of missing) {
    if (!byFile[m.file]) byFile[m.file] = [];
    byFile[m.file].push(m.key);
  }
  for (const f of Object.keys(byFile)) {
    console.log(`  ${f}:`);
    for (const k of byFile[f]) console.log(`    - ${k}`);
  }
}
