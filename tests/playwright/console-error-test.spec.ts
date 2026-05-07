import { test, Page } from '@playwright/test';
import path from 'path';
import fs from 'fs';
import http from 'http';

const BASE = 'http://127.0.0.1:3000';
const API = 'http://127.0.0.1:8082/api/v1/erp';
const SCREEN = path.resolve(__dirname, 'test-results/console-screenshots');
fs.mkdirSync(SCREEN, { recursive: true });

const testResults: { module: string; rendered: boolean; actions: string[]; errors: string[]; consoleErrors: string[] }[] = [];

function getLoginToken(): Promise<string> {
  return new Promise((resolve, reject) => {
    const data = JSON.stringify({ username: 'admin', password: 'admin123456' });
    const req = http.request({
      hostname: '127.0.0.1', port: 8082, path: '/api/v1/erp/auth/login',
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(data) }
    }, res => {
      let body = '';
      res.on('data', c => body += c);
      res.on('end', () => {
        try {
          const d = JSON.parse(body);
          if (d.success && d.data?.token) resolve(d.data.token);
          else reject(new Error('登录失败: ' + body.substring(0, 200)));
        } catch (e) { reject(new Error('解析失败: ' + body.substring(0, 200))); }
      });
    });
    req.on('error', reject);
    req.write(data);
    req.end();
  });
}

async function shot(page: Page, name: string) {
  const p = path.join(SCREEN, `${name.replace(/[\/\\\s:]/g, '_')}_${Date.now()}.png`);
  await page.screenshot({ path: p, fullPage: true });
  return p;
}

async function navigateAndWait(page: Page, route: string, timeoutMs = 25000): Promise<string> {
  const t0 = Date.now();
  await page.goto(`${BASE}/#${route}`, { waitUntil: 'domcontentloaded', timeout: 15000 }).catch(() => {});

  while (Date.now() - t0 < timeoutMs) {
    try {
      const info = await page.evaluate(() => {
        const body = document.body;
        const html = body.innerHTML;
        const bodyText = body.textContent || '';
        
        // 检测WASM渲染错误
        if (html.includes('DatabaseError') || bodyText.includes('column "') || bodyText.includes('relation "')) {
          return { status: 'db_error', detail: bodyText.substring(0, 200) };
        }
        
        // 检测内容是否渲染
        const main = document.querySelector('.main-content');
        const mainText = main ? (main.textContent || '') : '';
        
        if (mainText.includes('加载失败') || mainText.includes('解析响应')) {
          return { status: 'parse_error', detail: mainText.substring(0, 200) };
        }
        
        if (/秉羲/.test(bodyText) && bodyText.length > 80) {
          if (mainText.length > 5 || /[\u4e00-\u9fff]{3,}/.test(mainText)) {
            return { status: 'rendered', detail: '' };
          }
        }
        
        return { status: 'waiting', detail: '' };
      });
      
      if (info.status === 'rendered' || info.status === 'db_error' || info.status === 'parse_error') {
        return info.status === 'parse_error' ? 'parse_error:' + info.detail : info.status;
      }
    } catch (e) {
      // page may be navigating
    }
    await page.waitForTimeout(500);
  }
  return 'timeout';
}

async function userInteract(page: Page, moduleName: string): Promise<string[]> {
  const actions: string[] = [];

  // 1) 点击新建/创建按钮
  for (const t of ['新建', '创建', '添加', '新增']) {
    try {
      const btn = page.getByRole('button', { name: new RegExp(t) }).first();
      if (await btn.count() > 0 && await btn.isVisible({ timeout: 1500 }).catch(() => false)) {
        await btn.click();
        await page.waitForTimeout(1500);
        actions.push(`点击"${t}"按钮-弹窗打开`);

        const inp = page.locator('input:not([type="hidden"]):not([type="number"]):not([readonly]):enabled').first();
        if (await inp.count() > 0 && await inp.isVisible({ timeout: 500 }).catch(() => false)) {
          await inp.fill('测试用户输入123');
          actions.push('弹窗中输入数据');
        }

        const cancel = page.getByRole('button', { name: /取消|关闭/ }).first();
        if (await cancel.count() > 0 && await cancel.isVisible({ timeout: 500 }).catch(() => false)) {
          await cancel.click();
          actions.push('点击取消关闭弹窗');
        } else {
          await page.keyboard.press('Escape');
          actions.push('ESC关闭弹窗');
        }
        await page.waitForTimeout(1000);
        break;
      }
    } catch { /* ignore */ }
  }

  // 2) 搜索
  for (const t of ['搜索', '查询']) {
    try {
      const btn = page.getByRole('button', { name: new RegExp(t) }).first();
      if (await btn.count() > 0 && await btn.isVisible({ timeout: 1000 }).catch(() => false)) {
        const sb = page.locator('input[placeholder*="搜"],input[placeholder*="search"],input[placeholder*="查"]').first();
        if (await sb.count() > 0) { await sb.fill('test'); }
        await btn.click();
        await page.waitForTimeout(1000);
        actions.push(`点击"${t}"按钮`);
        break;
      }
    } catch { /* ignore */ }
  }

  // 3) 刷新
  try {
    const btn = page.getByRole('button', { name: /刷新/ }).first();
    if (await btn.count() > 0 && await btn.isVisible({ timeout: 1000 }).catch(() => false)) {
      await btn.click();
      actions.push('点击刷新');
    }
  } catch { /* ignore */ }

  return actions;
}

const modules: { name: string; route: string }[] = [
  { name: '仪表板', route: '/dashboard' },
  { name: '我的待办', route: '/my-tasks' },
  { name: '用户管理', route: '/users' },
  { name: '角色管理', route: '/roles' },
  { name: '部门管理', route: '/departments' },
  { name: '产品管理', route: '/products' },
  { name: '产品类别', route: '/product-categories' },
  { name: '客户管理', route: '/customers' },
  { name: '供应商管理', route: '/suppliers' },
  { name: '仓库管理', route: '/warehouses' },
  { name: '采购订单', route: '/purchase-orders' },
  { name: '采购收货', route: '/purchase-receipts' },
  { name: '采购退货', route: '/purchase-returns' },
  { name: '采购合同', route: '/purchase-contracts' },
  { name: '销售订单', route: '/sales' },
  { name: '销售合同', route: '/sales-contracts' },
  { name: '销售退货', route: '/sales-returns' },
  { name: '销售价格', route: '/sales-prices' },
  { name: '采购价格', route: '/purchase-prices' },
  { name: '库存查询', route: '/inventory' },
  { name: '库存调拨', route: '/inventory-transfers' },
  { name: '库存盘点', route: '/inventory-counts' },
  { name: '库存调整', route: '/inventory-adjustments' },
  { name: '质量检验', route: '/quality-inspection' },
  { name: '供应商评估', route: '/supplier-evaluation' },
  { name: '应收发票', route: '/ar-invoices' },
  { name: '应付发票', route: '/ap-invoices' },
  { name: '应付付款申请', route: '/ap-payment-requests' },
  { name: '应付付款', route: '/ap-payments' },
  { name: '成本归集', route: '/cost-collections' },
  { name: '资金管理', route: '/fund-management' },
  { name: '固定资产', route: '/fixed-assets' },
  { name: '会计科目', route: '/account-subjects' },
  { name: '记账凭证', route: '/vouchers' },
  { name: '客户信用', route: '/customer-credits' },
];

test.describe('浏览器控制台错误捕获+用户操作测试', () => {
  test.setTimeout(3600000);

  test('逐模块交互测试+捕获控制台错误', async ({ page }) => {
    // 收集控制台错误和警告
    const capturedErrors: string[] = [];
    const capturedWarnings: string[] = [];
    
    page.on('console', msg => {
      const t = msg.type();
      const txt = msg.text();
      if (t === 'error') capturedErrors.push(`[CONSOLE_${t}] ${txt.substring(0, 200)}`);
      else if (txt.includes('Error') || txt.includes('错误') || txt.includes('失败')) 
        capturedErrors.push(`[CONSOLE_${t}] ${txt.substring(0, 200)}`);
      else if (t === 'warning') capturedWarnings.push(`[WARN] ${txt.substring(0, 200)}`);
    });
    
    page.on('pageerror', err => {
      capturedErrors.push(`[PAGE_ERROR] ${err.message.substring(0, 200)}`);
    });

    // 获取登录token
    console.log('\n🔑 获取登录token...');
    let token: string;
    try {
      token = await getLoginToken();
      console.log('✅ 获取token成功:', token.substring(0, 20) + '...');
    } catch (e: any) {
      console.log('❌ 登录失败:', e.message);
      return;
    }

    // 先导航到首页并设置token
    await page.goto(BASE, { waitUntil: 'domcontentloaded', timeout: 15000 });
    await page.evaluate((t) => {
      sessionStorage.setItem('auth_token', t);
      sessionStorage.setItem('is_authenticated', 'true');
    }, token);

    console.log(`\n📋 开始测试 ${modules.length} 个模块\n`);

    for (let i = 0; i < modules.length; i++) {
      const m = modules[i];
      const prevErrCount = capturedErrors.length;
      const modErrors: string[] = [];

      console.log(`\n${i+1}. [${m.name}] ${m.route}`);
      
      const status = await navigateAndWait(page, m.route);
      
      // 新产生的控制台错误
      const newConsoleErrs = capturedErrors.slice(prevErrCount);
      
      let rendered = false;
      let actions: string[] = [];

      if (status === 'rendered') {
        rendered = true;
        console.log(`   ✅ 页面渲染成功`);
        actions = await userInteract(page, m.name);
        for (const a of actions) console.log(`      🖱 ${a}`);
      } else if (status.startsWith('parse_error:')) {
        const detail = status.replace('parse_error:', '');
        console.log(`   ❌ 前端解析错误: ${detail}`);
        modErrors.push(detail);
      } else if (status === 'db_error') {
        console.log(`   ❌ 数据库错误`);
        modErrors.push('数据库错误');
      } else {
        console.log(`   ⏰ 超时(未渲染)`);
        modErrors.push('超时');
      }

      if (newConsoleErrs.length > 0) {
        console.log(`   🔴 浏览器控制台错误(${newConsoleErrs.length}条):`);
        for (const e of newConsoleErrs.slice(0, 5)) {
          console.log(`      ${e}`);
        }
        modErrors.push(...newConsoleErrs);
      }

      const ss = await shot(page, m.name);
      console.log(`   📸 ${ss}`);

      testResults.push({
        module: m.name, rendered, actions, errors: modErrors,
        consoleErrors: newConsoleErrs
      });
    }

    // 输出报告
    console.log('\n\n═══════════════════════════════════════');
    console.log('📊 浏览器UI测试报告');
    console.log('═══════════════════════════════════════');
    const pass = testResults.filter(r => r.rendered && r.errors.length === 0);
    const fail = testResults.filter(r => !r.rendered || r.errors.length > 0);

    console.log(`\n✅ 通过 (${pass.length}/${modules.length}):`);
    for (const r of pass) console.log(`   ${r.module} - ${r.actions.join(', ')}`);

    console.log(`\n❌ 失败 (${fail.length}/${modules.length}):`);
    for (const r of fail) {
      console.log(`   [${r.module}] 渲染:${r.rendered} 错误:${r.errors.slice(0,3).join(' | ')}`);
    }

    const reportPath = path.resolve(__dirname, 'test-results/console-error-report.json');
    fs.writeFileSync(reportPath, JSON.stringify({
      summary: { total: modules.length, pass: pass.length, fail: fail.length },
      pass: pass.map(r => ({ module: r.module, actions: r.actions })),
      fail: fail.map(r => ({ module: r.module, rendered: r.rendered, errors: r.errors }))
    }, null, 2));
    console.log(`\n📄 报告: ${reportPath}`);
  });
});
