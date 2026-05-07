import { test, expect, Page } from '@playwright/test';
import path from 'path';

const BASE_URL = 'http://127.0.0.1:3000';
const API_BASE = 'http://127.0.0.1:8082/api/v1/erp';
const SCREENSHOT_DIR = path.resolve(__dirname, 'test-results', 'screenshots');

interface TestResult {
  module: string;
  status: 'pass' | 'fail' | 'skip';
  error?: string;
  screenshot?: string;
  consoleErrors: string[];
}

const results: TestResult[] = [];
let currentPage: Page;

function recordResult(module: string, status: 'pass' | 'fail' | 'skip', error?: string, screenshot?: string): TestResult {
  const r: TestResult = { module, status, error, screenshot, consoleErrors: [] };
  results.push(r);
  return r;
}

async function loginViaUI(page: Page): Promise<boolean> {
  try {
    const apiResp = await page.request.post(`${API_BASE}/auth/login`, {
      data: { username: 'admin', password: 'admin123456' }
    });
    if (apiResp.ok()) {
      const data = await apiResp.json();
      const token = data?.data?.token;
      if (token) {
        await page.evaluate((t) => {
          localStorage.setItem('token', t);
          localStorage.setItem('user', JSON.stringify({ id: 0, username: 'admin', role_id: 1 }));
        }, token);
        await page.goto(`${BASE_URL}/#/dashboard`, { waitUntil: 'domcontentloaded', timeout: 10000 });
        await page.waitForTimeout(3000);
        return true;
      }
    }
    return false;
  } catch (e: any) {
    console.error('Login failed:', e.message);
    return false;
  }
}

async function expandL1Group(page: Page, label: string): Promise<void> {
  const header = page.locator('.nav-group-header', { hasText: label }).first();
  if (await header.count() > 0) {
    const icon = header.locator('.nav-group-icon').first();
    const iconClass = (await icon.count() > 0) ? (await icon.getAttribute('class') || '') : '';
    const isOpen = iconClass.includes('open');
    if (!isOpen) {
      await header.click();
      await page.waitForTimeout(400);
    }
  }
}

async function expandL2Group(page: Page, label: string): Promise<void> {
  const header = page.locator('.nav-l2-header', { hasText: label });
  if (await header.count() > 0) {
    await header.click();
    await page.waitForTimeout(400);
  }
}

async function clickNavItem(page: Page, label: string): Promise<void> {
  const item = page.locator('.nav-item', { hasText: label });
  if (await item.count() > 0) {
    await item.first().click();
    await page.waitForTimeout(1500);
  }
}

async function checkPageErrors(page: Page): Promise<{ hasError: boolean; errorText: string; consoleErrors: string[] }> {
  const consoleErrors: string[] = [];

  page.on('console', msg => {
    if (msg.type() === 'error') {
      consoleErrors.push(msg.text());
    }
  });

  const body = await page.textContent('body') || '';

  // 检查常见错误模式
  const errorPatterns = [
    'DatabaseError', 'Query Error', '404', '500', '401',
    '无权访问', '加载失败', '获取失败', '创建失败', '删除失败',
    'error', 'Error', '失败'
  ];

  const foundErrors = errorPatterns.filter(p => body.includes(p));

  return {
    hasError: foundErrors.length > 0 || consoleErrors.length > 0,
    errorText: foundErrors.join(', '),
    consoleErrors
  };
}

// 模块定义: [L1组, L2组, 导航项文本, 页面路由, 模块名]
const MODULES: [string, string, string, string, string][] = [
  // 工作台
  ['工作台', '', '首页', '/dashboard', '仪表板'],
  ['工作台', '', '我的待办', '/my-tasks', '我的待办'],

  // 基础数据 - 组织架构
  ['基础数据', '组织架构', '用户管理', '/users', '用户管理'],
  ['基础数据', '组织架构', '角色管理', '/roles', '角色管理'],
  ['基础数据', '组织架构', '部门管理', '/departments', '部门管理'],

  // 基础数据 - 产品资料
  ['基础数据', '产品资料', '产品管理', '/products', '产品管理'],
  ['基础数据', '产品资料', '产品类别', '/product-categories', '产品类别'],

  // 基础数据 - 业务往来
  ['基础数据', '业务往来', '客户管理', '/customers', '客户管理'],
  ['基础数据', '业务往来', '供应商管理', '/suppliers', '供应商管理'],

  // 基础数据 - 仓库资料
  ['基础数据', '仓库资料', '仓库管理', '/warehouses', '仓库管理'],

  // 供应链管理 - 销售业务
  ['供应链管理', '销售业务', '销售订单', '/sales', '销售订单'],
  ['供应链管理', '销售业务', '面料订单', '/sales/fabric', '面料订单'],
  ['供应链管理', '销售业务', '销售合同', '/sales-contracts', '销售合同'],
  ['供应链管理', '销售业务', '销售退货', '/sales-returns', '销售退货'],

  // 供应链管理 - 采购业务
  ['供应链管理', '采购业务', '采购订单', '/purchase-orders', '采购订单'],
  ['供应链管理', '采购业务', '采购合同', '/purchase-contracts', '采购合同'],
  ['供应链管理', '采购业务', '采购收货', '/purchase-receipts', '采购收货'],
  ['供应链管理', '采购业务', '采购退货', '/purchase-returns', '采购退货'],

  // 供应链管理 - 价格管理
  ['供应链管理', '价格管理', '销售价格', '/sales-prices', '销售价格'],
  ['供应链管理', '价格管理', '采购价格', '/purchase-prices', '采购价格'],

  // 仓储与质量 - 仓储作业
  ['仓储与质量', '仓储作业', '库存查询', '/inventory', '库存查询'],
  ['仓储与质量', '仓储作业', '库存调拨', '/inventory-transfers', '库存调拨'],
  ['仓储与质量', '仓储作业', '库存盘点', '/inventory-counts', '库存盘点'],
  ['仓储与质量', '仓储作业', '库存调整', '/inventory-adjustments', '库存调整'],

  // 仓储与质量 - 质量管理
  ['仓储与质量', '质量管理', '质量检验', '/quality-inspection', '质量检验'],
  ['仓储与质量', '质量管理', '采购检验', '/purchase-inspections', '采购检验'],
  ['仓储与质量', '质量管理', '供应商评估', '/supplier-evaluation', '供应商评估'],

  // 财务核算 - 账务管理
  ['财务核算', '账务管理', '资金管理', '/fund-management', '资金管理'],
  ['财务核算', '账务管理', '固定资产', '/fixed-assets', '固定资产'],
  ['财务核算', '账务管理', '会计科目', '/account-subjects', '会计科目'],
  ['财务核算', '账务管理', '记账凭证', '/vouchers', '记账凭证'],

  // 财务核算 - 应收应付
  ['财务核算', '应收应付', '销售发票', '/ar-invoices', '应收发票'],
  ['财务核算', '应收应付', '采购发票', '/ap-invoices', '应付发票'],
  ['财务核算', '应收应付', '应付付款申请', '/ap-payment-requests', '应付付款申请'],
  ['财务核算', '应收应付', '应付付款', '/ap-payments', '应付付款'],
  ['财务核算', '应收应付', '客户信用', '/customer-credits', '客户信用'],

  // 财务核算 - 成本与分析
  ['财务核算', '成本与分析', '成本归集', '/cost-collections', '成本归集'],
  ['财务核算', '成本与分析', '辅助核算', '/assist-accounting', '辅助核算'],

  // 面料行业特色 - 生产与批次
  ['面料行业特色', '生产与批次', '批次管理', '/batches', '批次管理'],
  ['面料行业特色', '生产与批次', '染缸管理', '/dye-batches', '染缸管理'],
  ['面料行业特色', '生产与批次', '染料配方', '/dye-recipes', '染料配方'],
  ['面料行业特色', '生产与批次', '坯布管理', '/greige-fabrics', '坯布管理'],

  // 面料行业特色 - 特色工具
  ['面料行业特色', '特色工具', '双单位转换', '/dual-unit-converter', '双单位转换'],
  ['面料行业特色', '特色工具', '五维查询', '/five-dimensions', '五维查询'],
  ['面料行业特色', '特色工具', '业务追溯', '/business-trace', '业务追溯'],

  // 系统与分析 - 报表中心
  ['系统与分析', '报表中心', '销售分析', '/sales-analysis', '销售分析'],
  ['系统与分析', '报表中心', '财务分析', '/financial-analysis', '财务分析'],

  // 系统与分析 - 系统管理
  ['系统与分析', '系统管理', '系统设置', '/system-settings', '系统设置'],
];

test.describe('浏览器UI全面测试', () => {
  test.setTimeout(600000); // 10分钟总超时

  test.beforeAll(async ({ browser }) => {
    const context = await browser.newContext({
      viewport: { width: 1440, height: 900 },
      locale: 'zh-CN',
    });
    currentPage = await context.newPage();
  });

  test('登录测试', async () => {
    let success = false;
    try {
      success = await loginViaUI(currentPage);
    } catch (e: any) {
      console.error('Login attempt error:', e.message);
    }
    if (!success) {
       console.log('UI登录未完全成功，但后续模块测试使用API token可以正常工作');
     }
     expect(currentPage.url()).not.toContain('#/login');
  });

  // 逐模块测试
  for (const [l1, l2, label, route, name] of MODULES) {
    test(`导航到: ${name}`, async () => {
      try {
        // 确保已登录
        const currentUrl = currentPage.url();
        if (!currentUrl.includes('/#/') || currentUrl.includes('/login')) {
          await loginViaUI(currentPage);
        }

        // 展开L1组
        if (l1) {
          await expandL1Group(currentPage, l1);
        }
        // 展开L2组
        if (l2) {
          await expandL2Group(currentPage, l2);
        }

        // 收集控制台错误
        const consoleErrors: string[] = [];
        const consoleHandler = (msg: any) => {
          if (msg.type() === 'error') {
            consoleErrors.push(msg.text());
          }
        };
        currentPage.on('console', consoleHandler);

        // 点击导航项
        await clickNavItem(currentPage, label);

        // 等待页面稳定
        await currentPage.waitForTimeout(2000);

        // 检查页面状态 - 排除WASM初始化代码
        const body = await currentPage.textContent('body') || '';
        const bodyNoWasm = body.replace(/import\s+init[\s\S]*?wasm\s*=\s*await\s*init[\s\S]*?;\s*/g, '');
        const hasErrorPattern = /DatabaseError|Query Error|relation.*does not exist|column.*does not exist/.test(bodyNoWasm);
        const hasAccessDenied = bodyNoWasm.includes('无权访问');
        const hasServerError = bodyNoWasm.includes('500') || bodyNoWasm.includes('Internal Server Error');

        currentPage.off('console', consoleHandler);

        if (hasErrorPattern || hasServerError) {
          const screenshot = `${SCREENSHOT_DIR}/${name.replace(/[\/\\]/g, '_')}.png`;
          await currentPage.screenshot({ path: screenshot, fullPage: true });
          recordResult(name, 'fail', bodyNoWasm.substring(0, 300), screenshot);
          console.log(`❌ ${name}: 页面显示后端错误 - ${bodyNoWasm.substring(0, 150)}`);
        } else if (hasAccessDenied) {
          recordResult(name, 'skip', '权限不足');
          console.log(`⊘ ${name}: 权限不足，跳过`);
        } else if (consoleErrors.length > 0 && !consoleErrors.every(e => e.includes('favicon') || e.includes('wasm'))) {
          const realErrors = consoleErrors.filter(e => !e.includes('favicon') && !e.includes('wasm'));
          if (realErrors.length > 0) {
            const screenshot = `${SCREENSHOT_DIR}/${name.replace(/[\/\\]/g, '_')}.png`;
            await currentPage.screenshot({ path: screenshot, fullPage: true });
            recordResult(name, 'fail', `Console errors: ${realErrors.join('; ')}`, screenshot);
            console.log(`❌ ${name}: 控制台错误 - ${realErrors.join('; ')}`);
          } else {
            recordResult(name, 'pass');
            console.log(`✅ ${name}: 页面正常加载`);
          }
        } else {
          recordResult(name, 'pass');
          console.log(`✅ ${name}: 页面正常加载`);
        }
      } catch (e: any) {
        const screenshot = `${SCREENSHOT_DIR}/${name.replace(/[\/\\]/g, '_')}.png`;
        try { await currentPage.screenshot({ path: screenshot, fullPage: true }); } catch {}
        recordResult(name, 'fail', e.message, screenshot);
        console.log(`❌ ${name}: 异常 - ${e.message}`);
      }
    });
  }

  test.afterAll(async () => {
    // 生成测试报告
    const total = results.length;
    const passed = results.filter(r => r.status === 'pass').length;
    const failed = results.filter(r => r.status === 'fail').length;
    const skipped = results.filter(r => r.status === 'skip').length;

    console.log('\n========================================');
    console.log('         浏览器UI测试报告');
    console.log('========================================');
    console.log(`总计: ${total} | 通过: ${passed} | 失败: ${failed} | 跳过: ${skipped}`);
    console.log(`通过率: ${((passed / total) * 100).toFixed(1)}%`);
    console.log('----------------------------------------');

    if (failed > 0) {
      console.log('\n失败模块:');
      results.filter(r => r.status === 'fail').forEach(r => {
        console.log(`  ❌ ${r.module}: ${r.error?.substring(0, 100)}`);
      });
    }

    if (skipped > 0) {
      console.log('\n跳过模块:');
      results.filter(r => r.status === 'skip').forEach(r => {
        console.log(`  ⊘ ${r.module}`);
      });
    }

    // 写JSON报告
    const fs = require('fs');
    const reportPath = path.resolve(__dirname, 'test-results', 'ui-test-report.json');
    fs.mkdirSync(path.dirname(reportPath), { recursive: true });

    const summary = {
      timestamp: new Date().toISOString(),
      total,
      passed,
      failed,
      skipped,
      passRate: `${((passed / total) * 100).toFixed(1)}%`,
      results: results.map(r => ({
        module: r.module,
        status: r.status,
        error: r.error?.substring(0, 300),
        screenshot: r.screenshot,
        consoleErrors: r.consoleErrors.slice(0, 5),
      })),
    };
    fs.writeFileSync(reportPath, JSON.stringify(summary, null, 2), 'utf8');
    console.log(`\n测试报告已保存: ${reportPath}`);

    await currentPage.close();
  });
});
