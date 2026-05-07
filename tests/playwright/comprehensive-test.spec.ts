import { test, Page, expect } from '@playwright/test';
import path from 'path';
import fs from 'fs';

const BASE = 'http://127.0.0.1:3000';
const API = 'http://127.0.0.1:8082/api/v1/erp';
const SCREEN = path.resolve(__dirname, 'test-results/comprehensive-screenshots');
fs.mkdirSync(SCREEN, { recursive: true });

async function shot(page: Page, name: string) {
  const safe = name.replace(/[\/\\\s:：（）()]/g, '_');
  const p = path.join(SCREEN, `${safe}_${Date.now()}.png`);
  try { await page.screenshot({ path: p, fullPage: true }); } catch { /* ignore */ }
  return p;
}

interface ModuleResult {
  name: string;
  route: string;
  rendered: boolean;
  interactions: string[];
  consoleErrors: string[];
}

const results: ModuleResult[] = [];

let globalConsoleErrors: string[] = [];

test.describe('全面浏览器用户交互测试', () => {
  test.setTimeout(7200000);

  test('登录并逐模块深度交互测试', async ({ page, browserName }) => {
    page.on('console', msg => {
      if (msg.type() === 'error') {
        globalConsoleErrors.push(`[${msg.type()}] ${msg.text().substring(0, 200)}`);
      }
    });
    page.on('pageerror', err => {
      globalConsoleErrors.push(`[PAGE_ERROR] ${err.message.substring(0, 200)}`);
    });

    // ===== 登录 =====
    console.log('\n🔐 开始登录流程...');
    await page.goto(`${BASE}/#/login`, { waitUntil: 'domcontentloaded', timeout: 20000 });
    await page.waitForTimeout(2000);
    await shot(page, '01_登录页面');

    const usernameInput = page.locator('input[placeholder*="用户名"], input[name="username"], input[type="text"]').first();
    const passwordInput = page.locator('input[placeholder*="密码"], input[name="password"], input[type="password"]').first();
    const loginBtn = page.locator('button:has-text("登录"), button:has-text("登 录"), button[type="submit"]').first();

    if (await usernameInput.isVisible({ timeout: 3000 }).catch(() => false)) {
      await usernameInput.fill('admin');
      await passwordInput.fill('admin123456');
      console.log('   ✏️ 填写登录表单');
      await shot(page, '02_登录表单已填写');
      if (await loginBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
        await loginBtn.click();
        console.log('   🖱 点击登录按钮');
        await page.waitForTimeout(3000);
      }
    }

    await shot(page, '03_登录后页面');
    console.log('✅ 登录完成\n');

    // ===== 模块定义（全部模块及其测试策略）=====
    const modules = [
      // 基础数据
      { name: '仪表板', route: '/dashboard', category: '基础' },
      { name: '用户管理', route: '/users', category: '系统' },
      { name: '角色管理', route: '/roles', category: '系统' },
      { name: '系统设置', route: '/system-settings', category: '系统' },
      // 组织架构
      { name: '部门管理', route: '/departments', category: '基础' },
      { name: '仓库管理', route: '/warehouses', category: '基础' },
      // 产品与客户
      { name: '产品管理', route: '/products', category: '基础' },
      { name: '产品类别', route: '/product-categories', category: '基础' },
      { name: '客户管理', route: '/customers', category: '销售' },
      { name: '客户信用', route: '/customer-credits', category: '销售' },
      // 供应商
      { name: '供应商管理', route: '/suppliers', category: '采购' },
      // 采购
      { name: '采购订单', route: '/purchase-orders', category: '采购' },
      { name: '采购收货', route: '/purchase-receipts', category: '采购' },
      { name: '采购退货', route: '/purchase-returns', category: '采购' },
      { name: '采购合同', route: '/purchase-contracts', category: '采购' },
      { name: '采购价格', route: '/purchase-prices', category: '采购' },
      { name: '采购检验', route: '/purchase-inspections', category: '采购' },
      // 销售
      { name: '销售订单', route: '/sales', category: '销售' },
      { name: '销售合同', route: '/sales-contracts', category: '销售' },
      { name: '销售退货', route: '/sales-returns', category: '销售' },
      { name: '销售价格', route: '/sales-prices', category: '销售' },
      { name: '销售分析', route: '/sales-analysis', category: '销售' },
      // 库存
      { name: '库存查询', route: '/inventory', category: '库存' },
      { name: '库存调拨', route: '/inventory-transfers', category: '库存' },
      { name: '库存盘点', route: '/inventory-counts', category: '库存' },
      { name: '库存调整', route: '/inventory-adjustments', category: '库存' },
      // 质量
      { name: '质量检验', route: '/quality-inspection', category: '质量' },
      { name: '供应商评估', route: '/supplier-evaluation', category: '质量' },
      // 财务
      { name: '应收发票', route: '/ar-invoices', category: '财务' },
      { name: '应付发票', route: '/ap-invoices', category: '财务' },
      { name: '应付付款申请', route: '/ap-payment-requests', category: '财务' },
      { name: '应付付款', route: '/ap-payments', category: '财务' },
      { name: '资金管理', route: '/fund-management', category: '财务' },
      { name: '固定资产', route: '/fixed-assets', category: '财务' },
      { name: '会计科目', route: '/account-subjects', category: '财务' },
      { name: '记账凭证', route: '/vouchers', category: '财务' },
      { name: '成本归集', route: '/cost-collections', category: '财务' },
      { name: '财务分析', route: '/financial-analysis', category: '财务' },
      // 其他
      { name: '辅助核算', route: '/assist-accounting', category: '财务' },
      { name: '面料订单', route: '/sales/fabric', category: '销售' },
      { name: '批次管理', route: '/batches', category: '库存' },
      { name: '双单位转换', route: '/dual-unit-converter', category: '工具' },
      { name: '五维管理', route: '/five-dimensions', category: '报表' },
      { name: '业务追踪', route: '/business-trace', category: '报表' },
      { name: '应收应付报表', route: '/ap-reports', category: '报表' },
      { name: '应付对账', route: '/ap-reconciliations', category: '财务' },
      { name: '应付核销', route: '/ap-verifications', category: '财务' },
      { name: '染色批次', route: '/dye-batches', category: '生产' },
      { name: '染色配方', route: '/dye-recipes', category: '生产' },
      { name: '原布管理', route: '/greige-fabrics', category: '生产' },
      { name: 'CRM线索', route: '/crm/leads', category: 'CRM' },
      { name: 'CRM商机', route: '/crm/opportunities', category: 'CRM' },
      { name: '我的待办', route: '/my-tasks', category: '工作' },
    ];

    console.log(`📋 开始测试 ${modules.length} 个模块\n`);

    for (let i = 0; i < modules.length; i++) {
      const m = modules[i];
      const preErrorCount = globalConsoleErrors.length;
      const interactions: string[] = [];

      console.log(`\n━━━ ${i + 1}/${modules.length} [${m.category}] ${m.name} ━━━`);
      console.log(`   🔗 ${m.route}`);

      // 导航到页面
      let navOk = false;
      try {
        await page.goto(`${BASE}/#${m.route}`, { waitUntil: 'domcontentloaded', timeout: 20000 });
        await page.waitForTimeout(2000);
        navOk = true;
      } catch (e: any) {
        console.log(`   ❌ 导航失败: ${e.message}`);
      }

      if (!navOk) {
        results.push({ name: m.name, route: m.route, rendered: false, interactions, consoleErrors: [] });
        continue;
      }

      // 检查渲染状态
      const bodyText = (await page.textContent('body')) || '';
      const rendered = bodyText.length > 80;
      console.log(`   ${rendered ? '✅' : '❌'} 页面${rendered ? '已' : '未'}渲染 (body=${bodyText.length}字符)`);
      await shot(page, `${i + 1}_${m.name}`);

      if (!rendered) {
        const newErrs = globalConsoleErrors.slice(preErrorCount);
        results.push({ name: m.name, route: m.route, rendered: false, interactions, consoleErrors: newErrs });
        continue;
      }

      // === 1. 搜索/过滤测试 ===
      const searchInputs = page.locator('input[placeholder*="搜索"], input[placeholder*="关键字"], input[placeholder*="关键词"], input[placeholder*="search"], input[placeholder*="filter"], input[placeholder*="批次"], input[placeholder*="颜色"], input[placeholder*="编码"]');
      const searchCount = await searchInputs.count();
      if (searchCount > 0) {
        for (let si = 0; si < Math.min(searchCount, 2); si++) {
          const inp = searchInputs.nth(si);
          if (await inp.isVisible({ timeout: 500 }).catch(() => false)) {
            const ph = await inp.getAttribute('placeholder') || '';
            try {
              await inp.fill('');
              await inp.type('test', { delay: 50 });
              interactions.push(`搜索框"${ph}"输入"test"`);
              await page.waitForTimeout(500);
              await inp.fill('');
              await page.waitForTimeout(300);
              interactions.push('清空搜索');
            } catch { /* ignore */ }
          }
        }
      }

      // === 2. Select下拉筛选测试 ===
      const selects = page.locator('select');
      const selectCount = await selects.count();
      for (let si = 0; si < Math.min(selectCount, 3); si++) {
        const sel = selects.nth(si);
        if (await sel.isVisible({ timeout: 500 }).catch(() => false)) {
          const options = sel.locator('option');
          const optCount = await options.count();
          if (optCount > 1) {
            try {
              const vals = await options.evaluateAll((opts: HTMLOptionElement[]) => opts.map(o => o.value));
              const nonEmpty = vals.filter(v => v !== '');
              if (nonEmpty.length > 0) {
                await sel.selectOption(nonEmpty[0]);
                interactions.push(`下拉筛选选择"${nonEmpty[0]}"`);
                await page.waitForTimeout(500);
                await sel.selectOption('');
                interactions.push('重置下拉筛选');
                await page.waitForTimeout(300);
              }
            } catch { /* ignore */ }
          }
        }
      }

      // === 3. 查询/搜索按钮测试 ===
      const queryBtns = page.locator('button:has-text("查询"), button:has-text("搜索"), button:has-text("筛选")');
      const qCount = await queryBtns.count();
      for (let bi = 0; bi < Math.min(qCount, 2); bi++) {
        const btn = queryBtns.nth(bi);
        if (await btn.isVisible({ timeout: 500 }).catch(() => false)) {
          try {
            const txt = ((await btn.textContent()) || '').trim();
            await btn.click();
            interactions.push(`点击"${txt}"`);
            await page.waitForTimeout(500);
          } catch { /* ignore */ }
        }
      }

      // === 4. 分页测试 ===
      const pageBtns = page.locator('button:has-text("上一页"), button:has-text("下一页"), button:has-text("首页"), button:has-text("末页"), button.page-btn, button[class*="page"]');
      const pbCount = await pageBtns.count();
      for (let bi = 0; bi < Math.min(pbCount, 4); bi++) {
        const btn = pageBtns.nth(bi);
        if (await btn.isVisible({ timeout: 500 }).catch(() => false)) {
          try {
            const txt = ((await btn.textContent()) || '').trim();
            await btn.click();
            interactions.push(`分页: "${txt}"`);
            await page.waitForTimeout(500);
          } catch { /* ignore */ }
        }
      }

      // === 5. 表格数据验证 ===
      const tables = page.locator('table');
      const tableCount = await tables.count();
      if (tableCount > 0) {
        const rows = tables.first().locator('tbody tr');
        const rowCount = await rows.count();
        interactions.push(`表格有${rowCount}行数据`);
        if (rowCount > 0) {
          await shot(page, `${i + 1}_${m.name}_表格`);
        }
      }

      // === 6. 新建按钮测试 ===
      const createBtns = page.locator('button:has-text("新建"), button:has-text("新增"), button:has-text("添加"), button:has-text("创建"), button:has-text("发起")');
      const cCount = await createBtns.count();
      for (let bi = 0; bi < Math.min(cCount, 2); bi++) {
        const btn = createBtns.nth(bi);
        if (await btn.isVisible({ timeout: 1000 }).catch(() => false)) {
          try {
            const txt = ((await btn.textContent()) || '').trim();
            await btn.click();
            interactions.push(`点击"${txt}"`);
            await page.waitForTimeout(1000);
            await shot(page, `${i + 1}_${m.name}_新建弹窗`);

            // 在弹窗中填写表单
            const formInputs = page.locator('.modal input[type="text"], .modal textarea, .modal input:not([type="hidden"]), input.form-control, .form-group input');
            const fiCount = await formInputs.count();
            let filled = 0;
            for (let fi = 0; fi < Math.min(fiCount, 5); fi++) {
              const finp = formInputs.nth(fi);
              if (await finp.isVisible({ timeout: 500 }).catch(() => false)) {
                try {
                  const tagName = (await finp.evaluate(el => el.tagName)).toLowerCase();
                  const ph = await finp.getAttribute('placeholder') || '';
                  const testVal = `测试${fi + 1}`;
                  if (tagName === 'textarea' || tagName === 'input') {
                    await finp.fill(testVal);
                    filled++;
                  }
                } catch { /* ignore */ }
              }
            }
            if (filled > 0) interactions.push(`弹窗中填写${filled}个字段`);

            await shot(page, `${i + 1}_${m.name}_表单已填`);

            // 关闭弹窗（取消）
            const cancelBtns = page.locator('.modal button:has-text("取消"), .modal button:has-text("关闭"), button.close-btn, .modal-overlay button:has-text("×"), button:has-text("取消")');
            const caCount = await cancelBtns.count();
            if (caCount > 0) {
              for (let ci = 0; ci < Math.min(caCount, 2); ci++) {
                const cb = cancelBtns.nth(ci);
                if (await cb.isVisible({ timeout: 500 }).catch(() => false)) {
                  await cb.click();
                  interactions.push('取消弹窗');
                  await page.waitForTimeout(500);
                  break;
                }
              }
            } else {
              // Try clicking backdrop
              try { await page.locator('.modal-overlay').click({ position: { x: 5, y: 5 } }); } catch { /* ignore */ }
            }
          } catch { /* ignore */ }
        }
      }

      // === 7. 编辑按钮测试 ===
      if (tableCount > 0) {
        const editBtns = tables.first().locator('button:has-text("编辑"), button:has-text("修改"), a:has-text("编辑")');
        const eCount = await editBtns.count();
        if (eCount > 0) {
          const eb = editBtns.first();
          if (await eb.isVisible({ timeout: 500 }).catch(() => false)) {
            try {
              await eb.click();
              interactions.push('点击编辑');
              await page.waitForTimeout(1000);
              await shot(page, `${i + 1}_${m.name}_编辑`);
              
              // 关闭
              const cancelBtns = page.locator('button:has-text("取消"), button.close-btn, .modal-overlay button:has-text("×")');
              if (await cancelBtns.first().isVisible({ timeout: 500 }).catch(() => false)) {
                await cancelBtns.first().click();
                await page.waitForTimeout(500);
              }
            } catch { /* ignore */ }
          }
        }
      }

      // === 8. 删除按钮测试 ===
      if (tableCount > 0) {
        const delBtns = tables.first().locator('button:has-text("删除"), button.btn-danger');
        const dCount = await delBtns.count();
        if (dCount > 0) {
          const db = delBtns.last();
          if (await db.isVisible({ timeout: 500 }).catch(() => false)) {
            try {
              await db.click();
              interactions.push('点击删除');
              await page.waitForTimeout(500);
              // 处理确认对话框
              page.once('dialog', async dialog => {
                interactions.push('删除确认对话框出现');
                await dialog.dismiss();
              });
              await page.waitForTimeout(500);
            } catch { /* ignore */ }
          }
        }
      }

      // === 9. 刷新按钮 ===
      const refreshBtns = page.locator('button:has-text("刷新"), button:has-text("重新加载")');
      const rCount = await refreshBtns.count();
      if (rCount > 0) {
        const rb = refreshBtns.first();
        if (await rb.isVisible({ timeout: 500 }).catch(() => false)) {
          try {
            await rb.click();
            interactions.push('点击刷新');
            await page.waitForTimeout(500);
          } catch { /* ignore */ }
        }
      }

      // === 10. 详情/查看按钮 ===
      if (tableCount > 0) {
        const detailBtns = tables.first().locator('button:has-text("详情"), button:has-text("查看"), button:has-text("明细"), a:has-text("查看")');
        const detCount = await detailBtns.count();
        if (detCount > 0) {
          const detb = detailBtns.first();
          if (await detb.isVisible({ timeout: 500 }).catch(() => false)) {
            try {
              await detb.click();
              interactions.push('点击查看详情');
              await page.waitForTimeout(1000);
              await shot(page, `${i + 1}_${m.name}_详情`);
              
              // 关闭详情
              const closeBtns = page.locator('button:has-text("关闭"), button:has-text("返回"), button.close-btn');
              if (await closeBtns.first().isVisible({ timeout: 500 }).catch(() => false)) {
                await closeBtns.first().click();
                await page.waitForTimeout(300);
              }
            } catch { /* ignore */ }
          }
        }
      }

      // 收集结果
      const newErrs = globalConsoleErrors.slice(preErrorCount);
      if (newErrs.length > 0) {
        console.log(`   ⚠️ 新产生${newErrs.length}个控制台错误:`);
        for (const e of newErrs) console.log(`      • ${e}`);
      }

      results.push({ name: m.name, route: m.route, rendered, interactions, consoleErrors: newErrs });
      
      const status = rendered ? (newErrs.length > 0 ? '⚠️' : '✅') : '❌';
      console.log(`   ${status} 交互: [${interactions.join(', ')}]`);
    }

    // ===== 最终报告 =====
    console.log('\n\n' + '='.repeat(80));
    console.log('                    最 终 测 试 报 告');
    console.log('='.repeat(80));

    const allPass = results.filter(r => r.rendered && r.consoleErrors.length === 0);
    const withErrors = results.filter(r => r.rendered && r.consoleErrors.length > 0);
    const notRendered = results.filter(r => !r.rendered);

    console.log(`\n✅ 完全通过 (渲染+无错误): ${allPass.length}/${results.length}`);
    console.log(`⚠️ 渲染但有控制台错误: ${withErrors.length}/${results.length}`);
    console.log(`❌ 未渲染: ${notRendered.length}/${results.length}`);

    if (withErrors.length > 0) {
      console.log('\n⚠️ 有控制台错误的模块:');
      for (const r of withErrors) {
        console.log(`   ${r.name}: ${r.consoleErrors.length}个错误`);
        for (const e of r.consoleErrors) console.log(`      • ${e}`);
      }
    }

    if (notRendered.length > 0) {
      console.log('\n❌ 未渲染的模块:');
      for (const r of notRendered) console.log(`   ${r.name} (${r.route})`);
    }

    console.log('\n📊 总交互操作数:', results.reduce((s, r) => s + r.interactions.length, 0));
    console.log('📊 总控制台错误数:', globalConsoleErrors.length);
    console.log(`\n📸 截图保存在: ${SCREEN}\n`);

    // 断言
    expect(notRendered.length).toBe(0);
  });
});
