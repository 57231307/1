import { test, Page } from '@playwright/test';
import path from 'path';
import fs from 'fs';

const BASE = 'http://127.0.0.1:3000';
const SCREEN = path.resolve(__dirname, 'test-results/module-screenshots');
fs.mkdirSync(SCREEN, { recursive: true });

async function shot(page: Page, label: string) {
  try {
    const s = label.replace(/[\/\\\s:：（）()]/g, '_').substring(0, 50);
    await page.screenshot({ path: path.join(SCREEN, `${s}_${Date.now()}.png`), fullPage: true, timeout: 5000 });
  } catch { /* ignore */ }
}

async function quickLogin(page: Page) {
  try {
    await page.goto(`${BASE}/#/login`, { waitUntil: 'domcontentloaded', timeout: 10000 });
    await page.waitForTimeout(1000);
    const u = page.locator('input:visible').first();
    const pw = page.locator('input[type="password"]:visible').first();
    if (await u.isVisible({ timeout: 1000 }).catch(() => false) &&
        await pw.isVisible({ timeout: 1000 }).catch(() => false)) {
      await u.fill('admin');
      await pw.fill('admin123456');
      const btn = page.locator('button:has-text("登录"), button[type="submit"]').first();
      if (await btn.isVisible({ timeout: 1000 }).catch(() => false)) {
        await btn.click();
        await page.waitForTimeout(2500);
      }
    }
  } catch { /* skip login issues */ }
}

async function findAndClickButtons(page: Page, texts: string[]): Promise<string[]> {
  const clicks: string[] = [];
  for (const t of texts) {
    try {
      const btns = page.locator(`button:has-text("${t}")`);
      const cnt = await btns.count();
      for (let i = 0; i < Math.min(cnt, 2); i++) {
        const b = btns.nth(i);
        if (await b.isVisible({ timeout: 800 }).catch(() => false)) {
          await b.click({ timeout: 2000 });
          clicks.push(t);
          await page.waitForTimeout(800);
          
          // Handle confirm dialogs
          page.once('dialog', async d => { try { await d.dismiss(); } catch { /* */ } });
          
          // Close any modal that appeared
          await closeModal(page);
          break;
        }
      }
    } catch { /* skip */ }
  }
  return clicks;
}

async function closeModal(page: Page) {
  try {
    const modalVisible = page.locator('.modal:visible, .ant-modal:visible, [role="dialog"]:visible');
    if (await modalVisible.isVisible({ timeout: 250 }).catch(() => false)) {
      const cancel = page.locator('button:has-text("取消"):visible, button:has-text("关闭"):visible, .modal-close:visible');
      if (await cancel.first().isVisible({ timeout: 250 }).catch(() => false)) {
        await cancel.first().click({ timeout: 1000 });
        await page.waitForTimeout(300);
      }
    }
  } catch { /* */ }
}

async function countTableRows(page: Page): Promise<number> {
  try {
    return await page.locator('table tbody tr').count();
  } catch { return 0; }
}

const ALL_MODULES = [
  { name: '仪表板', route: '/dashboard', btns: ['刷新'], cat: '基础' },
  { name: '用户管理', route: '/users', btns: ['新建用户', '刷新'], cat: '系统' },
  { name: '角色管理', route: '/roles', btns: ['新建角色', '刷新'], cat: '系统' },
  { name: '系统设置', route: '/system-settings', btns: ['保存'], cat: '系统' },
  { name: '部门管理', route: '/departments', btns: ['新建部门', '编辑', '删除'], cat: '基础' },
  { name: '仓库管理', route: '/warehouses', btns: ['新建仓库', '编辑', '删除'], cat: '基础' },
  { name: '产品管理', route: '/products', btns: ['新建产品', '编辑', '删除'], cat: '基础' },
  { name: '产品类别', route: '/product-categories', btns: ['新建类别', '编辑', '删除'], cat: '基础' },
  { name: '客户管理', route: '/customers', btns: ['新建客户', '编辑', '删除'], cat: '销售' },
  { name: '客户信用', route: '/customer-credits', btns: ['新建', '刷新'], cat: '销售' },
  { name: '供应商管理', route: '/suppliers', btns: ['新建供应商', '编辑', '删除'], cat: '采购' },
  { name: '采购订单', route: '/purchase-orders', btns: ['新建采购订单', '编辑', '删除', '查询'], cat: '采购' },
  { name: '采购收货', route: '/purchase-receipts', btns: ['新建', '查询'], cat: '采购' },
  { name: '采购退货', route: '/purchase-returns', btns: ['新建', '查询'], cat: '采购' },
  { name: '采购合同', route: '/purchase-contracts', btns: ['新建', '查询'], cat: '采购' },
  { name: '采购价格', route: '/purchase-prices', btns: ['新建', '查询'], cat: '采购' },
  { name: '采购检验', route: '/purchase-inspections', btns: ['新建', '查询'], cat: '采购' },
  { name: '销售订单', route: '/sales', btns: ['新建销售订单', '编辑', '删除', '查询'], cat: '销售' },
  { name: '销售合同', route: '/sales-contracts', btns: ['新建', '查询'], cat: '销售' },
  { name: '销售退货', route: '/sales-returns', btns: ['新建', '查询'], cat: '销售' },
  { name: '销售价格', route: '/sales-prices', btns: ['新建', '查询'], cat: '销售' },
  { name: '销售分析', route: '/sales-analysis', btns: ['刷新'], cat: '销售' },
  { name: '库存查询', route: '/inventory', btns: ['查询', '刷新'], cat: '库存' },
  { name: '库存调拨', route: '/inventory-transfers', btns: ['新建', '查询'], cat: '库存' },
  { name: '库存盘点', route: '/inventory-counts', btns: ['新建', '查询'], cat: '库存' },
  { name: '库存调整', route: '/inventory-adjustments', btns: ['新建', '查询'], cat: '库存' },
  { name: '质量检验', route: '/quality-inspection', btns: ['新建', '查询'], cat: '质量' },
  { name: '供应商评估', route: '/supplier-evaluation', btns: ['新建', '刷新'], cat: '质量' },
  { name: '应收发票', route: '/ar-invoices', btns: ['新建', '查询'], cat: '财务' },
  { name: '应付发票', route: '/ap-invoices', btns: ['新建', '查询'], cat: '财务' },
  { name: '应付付款申请', route: '/ap-payment-requests', btns: ['新建', '查询'], cat: '财务' },
  { name: '应付付款', route: '/ap-payments', btns: ['新建', '查询'], cat: '财务' },
  { name: '资金管理', route: '/fund-management', btns: ['新建', '查询'], cat: '财务' },
  { name: '固定资产', route: '/fixed-assets', btns: ['新建', '查询'], cat: '财务' },
  { name: '会计科目', route: '/account-subjects', btns: ['新建科目', '刷新'], cat: '财务' },
  { name: '记账凭证', route: '/vouchers', btns: ['新建凭证', '审核'], cat: '财务' },
  { name: '成本归集', route: '/cost-collections', btns: ['新建', '查询'], cat: '财务' },
  { name: '财务分析', route: '/financial-analysis', btns: ['刷新'], cat: '财务' },
  { name: '辅助核算', route: '/assist-accounting', btns: ['新建', '刷新'], cat: '财务' },
  { name: '面料订单', route: '/sales/fabric', btns: ['新建', '查询'], cat: '销售' },
  { name: '批次管理', route: '/batches', btns: ['新建', '查询'], cat: '库存' },
  { name: '双单位转换', route: '/dual-unit-converter', btns: ['转换'], cat: '工具' },
  { name: '五维管理', route: '/five-dimensions', btns: ['刷新'], cat: '报表' },
  { name: '业务追踪', route: '/business-trace', btns: ['查询'], cat: '报表' },
  { name: '应收应付报表', route: '/ap-reports', btns: ['查询'], cat: '报表' },
  { name: '应付对账', route: '/ap-reconciliations', btns: ['新建', '查询'], cat: '财务' },
  { name: '应付核销', route: '/ap-verifications', btns: ['新建', '查询'], cat: '财务' },
  { name: '染色批次', route: '/dye-batches', btns: ['新建', '查询'], cat: '生产' },
  { name: '染色配方', route: '/dye-recipes', btns: ['新建', '查询'], cat: '生产' },
  { name: '原布管理', route: '/greige-fabrics', btns: ['新建', '查询'], cat: '生产' },
  { name: 'CRM线索', route: '/crm/leads', btns: ['新建', '转换'], cat: 'CRM' },
  { name: 'CRM商机', route: '/crm/opportunities', btns: ['新建', '转换'], cat: 'CRM' },
  { name: '我的待办', route: '/my-tasks', btns: ['刷新', '处理'], cat: '工作' },
];

// ===== 测试 =====
test.describe('逐模块截图+交互测试', () => {
  test.setTimeout(600000);

  ALL_MODULES.forEach((m, i) => {
    test(`${String(i + 1).padStart(2, '0')} [${m.cat}] ${m.name}`, async ({ page }) => {
      const errors: string[] = [];
      page.on('console', msg => { if (msg.type() === 'error') errors.push(msg.text().substring(0, 100)); });
      page.on('pageerror', e => errors.push(e.message.substring(0, 100)));

      console.log(`\n${i + 1}/${ALL_MODULES.length} [${m.cat}] ${m.name}`);

      // 登录
      await quickLogin(page);

      // 导航
      try {
        await page.goto(`${BASE}/#${m.route}`, { waitUntil: 'domcontentloaded', timeout: 12000 });
        await page.waitForTimeout(1500);
      } catch (e: any) {
        console.log(`  ❌ 导航失败: ${e.message}`);
        return;
      }

      // 渲染
      const body = (await page.textContent('body')) || '';
      const ok = body.length > 80;
      const rows = await countTableRows(page);
      
      await shot(page, `${m.name}_base`);
      console.log(`  ${ok ? '✅' : '❌'} 渲染 body=${body.length} 表格=${rows}行`);

      if (!ok) return;

      // 按钮交互
      const clicks = await findAndClickButtons(page, m.btns);
      if (clicks.length > 0) console.log(`  🖱 交互: ${clicks.join(' → ')}`);

      // 搜索框交互
      const searchInput = page.locator('input[placeholder*="搜索"], input[placeholder*="关键字"]').first();
      if (await searchInput.isVisible({ timeout: 500 }).catch(() => false)) {
        try {
          await searchInput.fill('测试搜索');
          console.log('  ⌨ 搜索框输入');
          await page.waitForTimeout(400);
          await searchInput.fill('');
          // Click search btn if exists
          const sBtn = page.locator('button:has-text("搜索"), button:has-text("查询")').first();
          if (await sBtn.isVisible({ timeout: 400 }).catch(() => false)) {
            await sBtn.click();
            await page.waitForTimeout(500);
          }
        } catch { /* */ }
      }

      // 分页
      const nextBtn = page.locator('button:has-text("下一页")').first();
      if (await nextBtn.isVisible({ timeout: 300 }).catch(() => false)) {
        try { await nextBtn.click(); console.log('  📄 下一页'); await page.waitForTimeout(300); } catch { /* */ }
      }

      await shot(page, `${m.name}_after`);
      await closeModal(page);

      if (errors.length > 0) {
        console.log(`  ⚠️ 控制台错误: ${errors.length}个`);
        errors.slice(0, 2).forEach(e => console.log(`     • ${e}`));
      }
    });
  });
});
