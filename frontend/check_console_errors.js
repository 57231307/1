import { chromium } from 'playwright';

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage();

const consoleErrors = [];
const consoleWarnings = [];
const jsErrors = [];

page.on('console', msg => {
  if (msg.type() === 'error') {
    consoleErrors.push(msg.text());
  } else if (msg.type() === 'warning') {
    consoleWarnings.push(msg.text());
  }
});

page.on('pageerror', error => {
  jsErrors.push({
    message: error.message,
    stack: error.stack
  });
});

page.on('response', response => {
  if (response.status() >= 400) {
    consoleErrors.push(`HTTP ${response.status()}: ${response.url()}`);
  }
});

console.log('正在访问前端页面...');

await page.goto('http://localhost:3000/', { waitUntil: 'networkidle', timeout: 30000 });
await page.waitForTimeout(3000);

const pagesToTest = [
  { url: '/login', name: '登录页' },
  { url: '/dashboard', name: '仪表盘' },
  { url: '/system', name: '系统管理' },
  { url: '/customer', name: '客户管理' },
  { url: '/supplier', name: '供应商管理' },
  { url: '/fabric', name: '面料管理' },
  { url: '/sales', name: '销售管理' },
  { url: '/purchase', name: '采购管理' },
  { url: '/inventory', name: '库存管理' },
  { url: '/finance', name: '财务管理' },
  { url: '/ap', name: '应付管理' },
  { url: '/ar', name: '应收管理' },
];

for (const p of pagesToTest) {
  try {
    await page.goto(`http://localhost:3000${p.url}`, { waitUntil: 'networkidle', timeout: 15000 });
    await page.waitForTimeout(2000);
    console.log(`✓ 已访问: ${p.name} (${p.url})`);
  } catch (e) {
    console.log(`✗ 访问失败: ${p.name} (${p.url}) - ${e.message}`);
  }
}

console.log('\n========== 控制台错误汇总 ==========');
if (consoleErrors.length > 0) {
  console.log(`\n发现 ${consoleErrors.length} 个错误:`);
  consoleErrors.forEach((err, i) => console.log(`  ${i + 1}. ${err}`));
} else {
  console.log('无控制台错误');
}

if (consoleWarnings.length > 0) {
  console.log(`\n发现 ${consoleWarnings.length} 个警告:`);
  consoleWarnings.forEach((warn, i) => console.log(`  ${i + 1}. ${warn}`));
}

if (jsErrors.length > 0) {
  console.log(`\n发现 ${jsErrors.length} 个 JavaScript 运行时错误:`);
  jsErrors.forEach((err, i) => console.log(`  ${i + 1}. ${err.message}`));
}

await browser.close();

console.log('\n========== 汇总报告 ==========');
console.log(`控制台错误: ${consoleErrors.length}`);
console.log(`控制台警告: ${consoleWarnings.length}`);
console.log(`JS 运行时错误: ${jsErrors.length}`);
