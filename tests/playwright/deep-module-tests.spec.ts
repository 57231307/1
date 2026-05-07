import { test, Page, expect } from '@playwright/test';
import path from 'path';
import fs from 'fs';

const BASE = 'http://127.0.0.1:3000';
const SCREEN = path.resolve(__dirname, 'test-results/deep-screenshots');
fs.mkdirSync(SCREEN, { recursive: true });

async function shot(page: Page, name: string) {
  const safe = name.replace(/[\/\\\s:：（）()]/g, '_').substring(0, 60);
  const p = path.join(SCREEN, `${safe}_${Date.now()}.png`);
  try { await page.screenshot({ path: p, fullPage: true, timeout: 5000 }); } catch { /* ignore */ }
  return p;
}

async function loginSetup(page: Page) {
  await page.goto(`${BASE}/#/login`, { waitUntil: 'domcontentloaded', timeout: 15000 });
  await page.waitForTimeout(1500);
  const u = page.locator('input[placeholder*="用户名"], input[name="username"], input[type="text"]').first();
  const p = page.locator('input[placeholder*="密码"], input[name="password"], input[type="password"]').first();
  if (await u.isVisible({ timeout: 2000 }).catch(() => false)) {
    await u.fill('admin');
    await p.fill('admin123456');
    const btn = page.locator('button:has-text("登录"), button[type="submit"]').first();
    if (await btn.isVisible({ timeout: 2000 }).catch(() => false)) {
      await btn.click();
      await page.waitForTimeout(3000);
    }
  }
}

async function testButtons(page: Page, name: string): Promise<string[]> {
  const actions: string[] = [];
  const allBtns = page.locator('button:visible');
  const count = await allBtns.count();
  const max = Math.min(count, 8);
  
  for (let i = 0; i < max; i++) {
    try {
      const btn = allBtns.nth(i);
      const txt = ((await btn.textContent()) || '').trim();
      if (!txt || txt.length > 30) continue;
      
      await btn.click({ timeout: 2000 });
      actions.push(`点击"${txt}"`);
      await page.waitForTimeout(400);
      
      // Handle any dialog
      page.once('dialog', async d => {
        await d.dismiss().catch(() => {});
      });
      
      // 如果出现弹窗/模态框，截图并关闭
      const modal = page.locator('.modal, .ant-modal, .el-dialog, [role="dialog"]');
      if (await modal.isVisible({ timeout: 300 }).catch(() => false)) {
        await shot(page, `${name}_弹窗_${txt}`);
        const cancel = page.locator('button:has-text("取消"), button:has-text("关闭"), .modal-close');
        if (await cancel.first().isVisible({ timeout: 300 }).catch(() => false)) {
          await cancel.first().click({ timeout: 1000 });
          await page.waitForTimeout(300);
          actions.push('关闭弹窗');
        }
      }
    } catch { /* skip broken buttons */ }
  }
  return actions;
}

async function testInputs(page: Page, name: string): Promise<string[]> {
  const actions: string[] = [];
  const inputs = page.locator('input:visible:not([type="hidden"]):not([type="submit"]):not([type="button"])');
  const count = await inputs.count();
  const max = Math.min(count, 4);
  
  for (let i = 0; i < max; i++) {
    try {
      const inp = inputs.nth(i);
      const ph = (await inp.getAttribute('placeholder')) || '';
      await inp.fill(`测试${i + 1}`);
      actions.push(`输入"${ph}"=测试${i + 1}`);
      await page.waitForTimeout(200);
    } catch { /* skip */ }
  }
  return actions;
}

async function testSelects(page: Page): Promise<string[]> {
  const actions: string[] = [];
  const selects = page.locator('select:visible');
  const count = await selects.count();
  
  for (let i = 0; i < Math.min(count, 3); i++) {
    try {
      const sel = selects.nth(i);
      const opts = sel.locator('option');
      const optCount = await opts.count();
      if (optCount > 1) {
        const vals = await opts.evaluateAll((els: HTMLOptionElement[]) => els.map(e => e.value));
        const nonEmpty = vals.filter(v => v && v !== '');
        if (nonEmpty.length > 0) {
          await sel.selectOption(nonEmpty[0]);
          actions.push(`选择${nonEmpty[0]}`);
          await page.waitForTimeout(300);
        }
      }
    } catch { /* skip */ }
  }
  return actions;
}

async function testPagination(page: Page): Promise<string[]> {
  const actions: string[] = [];
  const pager = page.locator('button:has-text("下一页"), button:has-text("上一页"), button:has-text("首页"), button:has-text("末页"), button[class*="pagination"]');
  const count = await pager.count();
  for (let i = 0; i < Math.min(count, 2); i++) {
    try {
      const btn = pager.nth(i);
      if (await btn.isEnabled({ timeout: 500 }).catch(() => false)) {
        await btn.click({ timeout: 2000 });
        actions.push(`分页: ${(await btn.textContent()) || ''}`);
        await page.waitForTimeout(500);
      }
    } catch { /* skip */ }
  }
  return actions;
}

async function checkTableData(page: Page): Promise<string> {
  try {
    const rows = page.locator('table tbody tr');
    const count = await rows.count();
    return `${count}行`;
  } catch { return '无表格'; }
}

// ===== 模块清单 =====
const modules = [
  '仪表板,/dashboard', '用户管理,/users', '角色管理,/roles', '系统设置,/system-settings',
  '部门管理,/departments', '仓库管理,/warehouses',
  '产品管理,/products', '产品类别,/product-categories',
  '客户管理,/customers', '客户信用,/customer-credits',
  '供应商管理,/suppliers',
  '采购订单,/purchase-orders', '采购收货,/purchase-receipts', '采购退货,/purchase-returns',
  '采购合同,/purchase-contracts', '采购价格,/purchase-prices', '采购检验,/purchase-inspections',
  '销售订单,/sales', '销售合同,/sales-contracts', '销售退货,/sales-returns',
  '销售价格,/sales-prices', '销售分析,/sales-analysis',
  '库存查询,/inventory', '库存调拨,/inventory-transfers', '库存盘点,/inventory-counts',
  '库存调整,/inventory-adjustments',
  '质量检验,/quality-inspection', '供应商评估,/supplier-evaluation',
  '应收发票,/ar-invoices', '应付发票,/ap-invoices',
  '应付付款申请,/ap-payment-requests', '应付付款,/ap-payments',
  '资金管理,/fund-management', '固定资产,/fixed-assets',
  '会计科目,/account-subjects', '记账凭证,/vouchers',
  '成本归集,/cost-collections', '财务分析,/financial-analysis',
  '辅助核算,/assist-accounting', '面料订单,/sales/fabric',
  '批次管理,/batches', '双单位转换,/dual-unit-converter',
  '五维管理,/five-dimensions', '业务追踪,/business-trace',
  '应收应付报表,/ap-reports', '应付对账,/ap-reconciliations',
  '应付核销,/ap-verifications',
  '染色批次,/dye-batches', '染色配方,/dye-recipes', '原布管理,/greige-fabrics',
  'CRM线索,/crm/leads', 'CRM商机,/crm/opportunities',
  '我的待办,/my-tasks',
];

// 分组并行：每10个模块一组
const groupSize = 10;
const groups: string[][] = [];
for (let i = 0; i < modules.length; i += groupSize) {
  groups.push(modules.slice(i, i + groupSize));
}

groups.forEach((group, gi) => {
  test.describe(`第${gi + 1}组模块 (${group.length}个)`, () => {
    test.setTimeout(group.length * 60000);
    
    for (const entry of group) {
      const [name, route] = entry.split(',');
      
      test(`${name} - ${route}`, async ({ page }) => {
        const errors: string[] = [];
        page.on('console', m => { if (m.type() === 'error') errors.push(m.text().substring(0, 150)); });
        page.on('pageerror', e => errors.push(e.message.substring(0, 150)));
        
        console.log(`\n🔍 [${name}] ${route}`);
        
        // 登录
        await loginSetup(page);
        
        // 导航
        try {
          await page.goto(`${BASE}/#${route}`, { waitUntil: 'domcontentloaded', timeout: 15000 });
          await page.waitForTimeout(2000);
        } catch (e: any) {
          console.log(`  ❌ 导航超时: ${e.message}`);
          return;
        }
        
        // 渲染检查
        const body = (await page.textContent('body')) || '';
        const ok = body.length > 80;
        console.log(`  ${ok ? '✅' : '❌'} 渲染 body=${body.length}字符`);
        
        if (!ok) return;
        
        await shot(page, name);
        
        // 交互测试
        const allActions: string[] = [];
        
        const inpActions = await testInputs(page, name);
        allActions.push(...inpActions);
        
        const selActions = await testSelects(page);
        allActions.push(...selActions);
        
        const pageActions = await testPagination(page);
        allActions.push(...pageActions);
        
        const tableInfo = await checkTableData(page);
        allActions.push(`表格:${tableInfo}`);
        
        const btnActions = await testButtons(page, name);
        allActions.push(...btnActions);
        
        console.log(`  🖱 [${allActions.join(' | ')}]`);
        
        if (errors.length > 0) {
          console.log(`  ⚠️ ${errors.length}个控制台错误:`);
          errors.slice(0, 3).forEach(e => console.log(`     • ${e}`));
          if (errors.length > 3) console.log(`     ... 共${errors.length}个`);
        }
      });
    }
  });
});
