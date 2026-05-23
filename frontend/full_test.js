import { chromium } from 'playwright';

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext();
const page = await context.newPage();

const allErrors = [];
const apiErrors = [];
const consoleErrors = [];
const pageErrors = [];

// 监听控制台消息
page.on('console', msg => {
  if (msg.type() === 'error') {
    consoleErrors.push({
      page: page.url(),
      text: msg.text()
    });
  }
});

// 监听页面错误
page.on('pageerror', error => {
  pageErrors.push({
    page: page.url(),
    message: error.message,
    stack: error.stack?.split('\n').slice(0, 3).join(' ')
  });
});

// 监听 API 响应
page.on('response', response => {
  const url = response.url();
  if (url.includes('/api/') && response.status() >= 400) {
    apiErrors.push({
      page: page.url(),
      url: url,
      status: response.status()
    });
  }
});

// 先登录
console.log('=== 登录 ===');
await page.goto('http://111.230.99.236/login', { waitUntil: 'networkidle', timeout: 30000 });
await page.waitForTimeout(2000);

// 填写登录表单
const usernameInput = await page.$('input[type="text"], input[placeholder*="用户"], input[placeholder*="账号"]');
const passwordInput = await page.$('input[type="password"]');

if (usernameInput && passwordInput) {
  await usernameInput.fill('admin');
  await passwordInput.fill('admin123');
  
  const loginButton = await page.$('button[type="submit"], button:has-text("登录")');
  if (loginButton) {
    await loginButton.click();
    await page.waitForTimeout(3000);
    console.log('登录成功');
  }
}

// 测试所有页面
const pagesToTest = [
  { url: '/dashboard', name: '仪表盘' },
  { url: '/customer', name: '客户管理' },
  { url: '/supplier', name: '供应商管理' },
  { url: '/product', name: '产品管理' },
  { url: '/warehouse', name: '仓库管理' },
  { url: '/fabric', name: '面料管理' },
  { url: '/greige-fabrics', name: '坯布管理' },
  { url: '/sales', name: '销售管理' },
  { url: '/purchase', name: '采购管理' },
  { url: '/inventory', name: '库存管理' },
  { url: '/finance', name: '财务管理' },
  { url: '/ap', name: '应付管理' },
  { url: '/ar', name: '应收管理' },
  { url: '/system', name: '系统管理' },
  { url: '/departments', name: '部门管理' },
  { url: '/fund', name: '资金管理' },
  { url: '/budget', name: '预算管理' },
  { url: '/cost', name: '成本管理' },
  { url: '/quality', name: '质量管理' },
  { url: '/bpm', name: 'BPM' },
  { url: '/crm', name: 'CRM' },
  { url: '/production', name: '生产管理' },
  { url: '/bom', name: 'BOM' },
  { url: '/mrp', name: 'MRP' },
  { url: '/scheduling', name: '排产' },
  { url: '/fixed-assets', name: '固定资产' },
  { url: '/customer-credit', name: '客户信用' },
  { url: '/supplier-evaluation', name: '供应商评估' },
  { url: '/financial-analysis', name: '财务分析' },
  { url: '/currency', name: '币种管理' },
  { url: '/notification', name: '通知' },
  { url: '/data-permission', name: '数据权限' },
  { url: '/sales-returns', name: '销售退货' },
  { url: '/purchase-ext', name: '采购扩展' },
  { url: '/sales-ext', name: '销售扩展' },
  { url: '/inventory-count', name: '盘点' },
  { url: '/inventory-transfer', name: '调拨' },
  { url: '/inventory-adjustment', name: '调整' },
  { url: '/ar-reconciliation', name: '应收对账' },
  { url: '/finance-report', name: '财务报表' },
  { url: '/purchase-receipt', name: '采购入库' },
  { url: '/advanced', name: '高级功能' },
  { url: '/omni-audit', name: '审计' },
  { url: '/business-trace', name: '业务追溯' },
  { url: '/five-dimension', name: '五维追溯' },
  { url: '/assist-accounting', name: '辅助核算' },
  { url: '/inventory-batch', name: '批次管理' },
];

console.log(`\n=== 测试 ${pagesToTest.length} 个页面 ===\n`);

let passCount = 0;
let failCount = 0;

for (const p of pagesToTest) {
  try {
    const errorsBefore = apiErrors.length + consoleErrors.length + pageErrors.length;
    
    await page.goto(`http://111.230.99.236${p.url}`, { waitUntil: 'networkidle', timeout: 15000 });
    await page.waitForTimeout(1500);
    
    const errorsAfter = apiErrors.length + consoleErrors.length + pageErrors.length;
    const newErrors = errorsAfter - errorsBefore;
    
    if (newErrors === 0) {
      console.log(`✓ ${p.name} (${p.url})`);
      passCount++;
    } else {
      console.log(`✗ ${p.name} (${p.url}) - ${newErrors} 个错误`);
      failCount++;
    }
  } catch (e) {
    console.log(`✗ ${p.name} (${p.url}) - 访问失败: ${e.message.substring(0, 50)}`);
    failCount++;
  }
}

// 汇总报告
console.log('\n=========================================');
console.log('  测试汇总');
console.log('=========================================');
console.log(`页面测试: 通过 ${passCount}  失败 ${failCount}  总计 ${passCount + failCount}`);
console.log(`API 错误: ${apiErrors.length}`);
console.log(`控制台错误: ${consoleErrors.length}`);
console.log(`页面错误: ${pageErrors.length}`);

if (apiErrors.length > 0) {
  // 去重
  const uniqueApiErrors = {};
  apiErrors.forEach(e => {
    const key = `${e.status} ${e.url.replace(/\?.*/, '')}`;
    if (!uniqueApiErrors[key]) {
      uniqueApiErrors[key] = { ...e, count: 0 };
    }
    uniqueApiErrors[key].count++;
  });
  
  console.log('\n--- API 错误详情 (去重) ---');
  Object.values(uniqueApiErrors).forEach((e, i) => {
    console.log(`  ${i + 1}. [${e.status}] ${e.url.replace(/http:\/\/[^\/]+/, '')} (${e.count}次)`);
  });
}

if (pageErrors.length > 0) {
  const uniquePageErrors = {};
  pageErrors.forEach(e => {
    const key = e.message.substring(0, 80);
    if (!uniquePageErrors[key]) {
      uniquePageErrors[key] = { ...e, count: 0 };
    }
    uniquePageErrors[key].count++;
  });
  
  console.log('\n--- 页面错误详情 (去重) ---');
  Object.values(uniquePageErrors).forEach((e, i) => {
    console.log(`  ${i + 1}. ${e.message.substring(0, 80)} (${e.count}次)`);
  });
}

if (consoleErrors.length > 0) {
  const uniqueConsoleErrors = {};
  consoleErrors.forEach(e => {
    const key = e.text.substring(0, 80);
    if (!uniqueConsoleErrors[key]) {
      uniqueConsoleErrors[key] = { ...e, count: 0 };
    }
    uniqueConsoleErrors[key].count++;
  });
  
  console.log('\n--- 控制台错误详情 (去重) ---');
  Object.values(uniqueConsoleErrors).forEach((e, i) => {
    console.log(`  ${i + 1}. ${e.text.substring(0, 80)} (${e.count}次)`);
  });
}

await browser.close();
