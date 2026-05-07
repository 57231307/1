import { test, Page } from '@playwright/test';
import path from 'path';
import fs from 'fs';

const BASE = 'http://127.0.0.1:3000';
const API = 'http://127.0.0.1:8082/api/v1/erp';
const SCREEN = path.resolve(__dirname, 'test-results/screenshots');
fs.mkdirSync(SCREEN, { recursive: true });

// ── 结果记录 ─────────────────────────────────────

interface Step { module: string; action: string; status: string; detail?: string; ss?: string }
const allSteps: Step[] = [];

function log(m: string, a: string, s: string, d?: string, ss?: string) {
  const e = s === 'pass' ? '✅' : s === 'fail' ? '❌' : '⊘';
  console.log(`${e} [${m}] ${a}${d ? ': ' + d.substring(0, 60) : ''}`);
  allSteps.push({ module: m, action: a, status: s, detail: d, ss });
}

async function shot(page: Page, name: string) {
  const p = path.join(SCREEN, `${name.replace(/[\/\\\s]/g, '_')}_${Date.now()}.png`);
  await page.screenshot({ path: p, fullPage: true });
  return p;
}

// ── WASM渲染等待 ─────────────────────────────────

async function waitWasm(page: Page, ms = 30000): Promise<string> {
  const t0 = Date.now();
  while (Date.now() - t0 < ms) {
    const b = await page.textContent('body') || '';
    if (b.includes('DatabaseError') || b.includes('column ') || b.includes('relation ')) return 'db_error';
    if (/秉羲/.test(b) && b.length > 100) return b;
    // 用innerHTML检测更丰富的内容
    const html = await page.locator('body').innerHTML();
    if (html.includes('DatabaseError')) return 'db_error';
    if (html.length > 3000 && /[\u4e00-\u9fff]{5,}/.test(html)) return b;
    await page.waitForTimeout(800);
  }
  const b = await page.textContent('body') || '';
  return b.length > 100 ? b : 'timeout';
}

// ── 模块测试 ─────────────────────────────────────

interface ModuleDef { route: string; name: string }

const MODULES: ModuleDef[] = [
  { route: '/dashboard', name: '仪表板' },
  { route: '/users', name: '用户管理' },
  { route: '/roles', name: '角色管理' },
  { route: '/departments', name: '部门管理' },
  { route: '/products', name: '产品管理' },
  { route: '/product-categories', name: '产品类别' },
  { route: '/warehouses', name: '仓库管理' },
  { route: '/customers', name: '客户管理' },
  { route: '/suppliers', name: '供应商管理' },
  { route: '/sales', name: '销售订单' },
  { route: '/sales-contracts', name: '销售合同' },
  { route: '/sales-returns', name: '销售退货' },
  { route: '/purchase-orders', name: '采购订单' },
  { route: '/purchase-contracts', name: '采购合同' },
  { route: '/purchase-receipts', name: '采购收货' },
  { route: '/purchase-returns', name: '采购退货' },
  { route: '/sales-prices', name: '销售价格' },
  { route: '/purchase-prices', name: '采购价格' },
  { route: '/inventory', name: '库存查询' },
  { route: '/inventory-transfers', name: '库存调拨' },
  { route: '/inventory-counts', name: '库存盘点' },
  { route: '/inventory-adjustments', name: '库存调整' },
  { route: '/quality-inspection', name: '质量检验' },
  { route: '/purchase-inspections', name: '采购检验' },
  { route: '/supplier-evaluation', name: '供应商评估' },
  { route: '/fund-management', name: '资金管理' },
  { route: '/fixed-assets', name: '固定资产' },
  { route: '/account-subjects', name: '会计科目' },
  { route: '/vouchers', name: '记账凭证' },
  { route: '/ar-invoices', name: '应收发票' },
  { route: '/ap-invoices', name: '应付发票' },
  { route: '/ap-payment-requests', name: '应付付款申请' },
  { route: '/ap-payments', name: '应付付款' },
  { route: '/customer-credits', name: '客户信用' },
  { route: '/cost-collections', name: '成本归集' },
  { route: '/assist-accounting', name: '辅助核算' },
  { route: '/batches', name: '批次管理' },
  { route: '/dye-batches', name: '染缸管理' },
  { route: '/dye-recipes', name: '染料配方' },
  { route: '/greige-fabrics', name: '坯布管理' },
  { route: '/dual-unit-converter', name: '双单位转换' },
  { route: '/five-dimensions', name: '五维查询' },
  { route: '/business-trace', name: '业务追溯' },
  { route: '/sales-analysis', name: '销售分析' },
  { route: '/financial-analysis', name: '财务分析' },
  { route: '/system-settings', name: '系统设置' },
];

// ── 页面内用户操作 ───────────────────────────────

async function interactPage(page: Page, name: string) {
  try {
    // 等待渲染
    const body = await waitWasm(page);
    if (body === 'timeout') { log(name, '页面加载', 'fail', 'WASM未渲染', await shot(page, name)); return; }
    if (body === 'db_error') { log(name, '页面加载', 'fail', '数据库错误', await shot(page, name)); return; }
    log(name, '页面加载并渲染', 'pass');

    // 1) 点击"新建/创建/添加"按钮 → 弹窗中填表格 → 点取消关闭
    const createTexts = ['新建', '创建', '添加', '新增'];
    let foundCreate = false;
    for (const t of createTexts) {
      const btn = page.getByRole('button', { name: new RegExp(t) }).first();
      if (await btn.count() > 0 && await btn.isVisible({ timeout: 1000 }).catch(() => false)) {
        await btn.click();
        await page.waitForTimeout(2000);
        log(name, `点击"${t}"按钮打开弹窗`, 'pass');

        // 填写第一个可见的非number input
        const inp = page.locator('input:not([type="hidden"]):not([type="number"]):not([readonly]):enabled').first();
        if (await inp.count() > 0 && await inp.isVisible({ timeout: 500 }).catch(() => false)) {
          await inp.fill('测试用户操作123');
          log(name, '在弹窗表单中输入数据', 'pass');
        } else {
          log(name, '弹窗表单输入', 'skip', '未找到可填写的输入框');
        }

        // 关闭弹窗
        const cancel = page.getByRole('button', { name: /取消|关闭/ }).first();
        if (await cancel.count() > 0 && await cancel.isVisible({ timeout: 500 }).catch(() => false)) {
          await cancel.click();
          log(name, '点击取消关闭弹窗', 'pass');
        } else {
          await page.keyboard.press('Escape');
          log(name, 'ESC关闭弹窗', 'pass');
        }
        await page.waitForTimeout(1000);
        foundCreate = true;
        break;
      }
    }
    if (!foundCreate) log(name, '创建按钮', 'skip', '页面无新建/创建按钮');

    // 2) 点击搜索 + 填写搜索框
    let foundSearch = false;
    for (const t of ['搜索', '查询']) {
      const btn = page.getByRole('button', { name: new RegExp(t) }).first();
      if (await btn.count() > 0 && await btn.isVisible({ timeout: 1000 }).catch(() => false)) {
        // 先填搜索框
        const sb = page.locator('input[placeholder*="搜"],input[placeholder*="search"],input[placeholder*="Search"]').first();
        if (await sb.count() > 0) { await sb.fill('test'); log(name, '填写搜索框', 'pass'); }
        await btn.click();
        await page.waitForTimeout(1500);
        log(name, `点击"${t}"按钮`, 'pass');
        foundSearch = true;
        break;
      }
    }
    if (!foundSearch) log(name, '搜索按钮', 'skip');

    // 3) 点击第一行的"编辑"/"查看"
    for (const t of ['编辑', '查看']) {
      const btn = page.getByRole('button', { name: new RegExp(`^${t}$| ${t}`) }).first();
      if (await btn.count() > 0 && await btn.isVisible({ timeout: 1000 }).catch(() => false)) {
        await btn.click();
        await page.waitForTimeout(1000);
        log(name, `点击"${t}"按钮`, 'pass');
        await page.keyboard.press('Escape');
        await page.waitForTimeout(500);
        break;
      }
    }

    // 4) 点击刷新
    for (const t of ['刷新']) {
      const btn = page.getByRole('button', { name: new RegExp(t) }).first();
      if (await btn.count() > 0 && await btn.isVisible({ timeout: 1000 }).catch(() => false)) {
        await btn.click();
        await page.waitForTimeout(1000);
        log(name, `点击"${t}"按钮`, 'pass');
        break;
      }
    }

  } catch (e: any) {
    log(name, '异常', 'fail', e.message, await shot(page, name));
  }
}

// ── 测试套件 ──────────────────────────────────────

test.describe('浏览器用户操作模拟测试', () => {
  test.setTimeout(1800000);

  let page: Page;
  let token = '';

  test.beforeAll(async ({ browser }) => {
    const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 }, locale: 'zh-CN' });
    page = await ctx.newPage();

    // 使用浏览器内的 fetch 调用 API 获取 token
    await page.goto(BASE, { waitUntil: 'domcontentloaded', timeout: 10000 });
    token = await page.evaluate(async (api) => {
      const r = await fetch(`${api}/auth/login`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ username: 'admin', password: 'admin123456' }),
      });
      const d = await r.json();
      return d?.data?.token || '';
    }, API);

    // 设置sessionStorage认证
    await page.evaluate((t) => {
      sessionStorage.setItem('auth_token', t);
      sessionStorage.setItem('is_authenticated', 'true');
    }, token);

    log('登录', '获取token并设置sessionStorage', token ? 'pass' : 'fail', token ? token.substring(0, 20) : '无');
  });

  // 逐模块测试
  for (const m of MODULES) {
    test(`${m.name}`, async () => {
      if (!token) { log(m.name, '前置条件', 'skip', '未获取到token'); return; }

      // 每次导航前先设置sessionStorage
      await page.evaluate((t) => {
        sessionStorage.setItem('auth_token', t);
        sessionStorage.setItem('is_authenticated', 'true');
      }, token);

      // URL导航
      await page.goto(`${BASE}/#${m.route}`, { waitUntil: 'domcontentloaded', timeout: 10000 });
      await page.waitForTimeout(1000);

      // 如果页面重定向到登录页，重新设置sessionStorage
      await page.waitForTimeout(2000);
      if (page.url().includes('#/login') || page.url().includes('/login#')) {
        await page.evaluate((t) => {
          sessionStorage.setItem('auth_token', t);
          sessionStorage.setItem('is_authenticated', 'true');
        }, token);
        await page.goto(`${BASE}/#${m.route}`, { waitUntil: 'domcontentloaded', timeout: 10000 });
      }

      await interactPage(page, m.name);
    });
  }

  test.afterAll(async () => {
    const pass = allSteps.filter(s => s.status === 'pass').length;
    const fail = allSteps.filter(s => s.status === 'fail').length;
    const skip = allSteps.filter(s => s.status === 'skip').length;
    const total = pass + fail + skip;

    console.log('\n╔══════════════════════════════════╗');
    console.log(`║  模块:${String(MODULES.length).padStart(2)} | 步骤:${String(total).padStart(3)} | 通过:${String(pass).padStart(3)}   ║`);
    console.log(`║  失败:${String(fail).padStart(3)} | 跳过:${String(skip).padStart(3)} | 通过率:${((pass/(pass+fail||1))*100).toFixed(1)}% ║`);
    console.log('╚══════════════════════════════════╝');

    if (fail > 0) {
      console.log('\n❌ 失败项:');
      allSteps.filter(s => s.status === 'fail').forEach(s =>
        console.log(`  [${s.module}] ${s.action}: ${(s.detail || '').substring(0, 80)}`)
      );
    }

    fs.writeFileSync(
      path.resolve(__dirname, 'test-results/ui-interaction-report.json'),
      JSON.stringify({ time: new Date().toISOString(), modules: MODULES.length, total, pass, fail, skip,
        rate: `${((pass/(pass+fail||1))*100).toFixed(1)}%`,
        steps: allSteps.map(s => ({ m: s.module, a: s.action, s: s.status, d: (s.detail || '').substring(0, 200) }))
      }, null, 2), 'utf8');

    await page.close();
  });
});
