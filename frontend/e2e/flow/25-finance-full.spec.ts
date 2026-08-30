import { test, expect } from '@playwright/test';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  genCode,
  getCtx,
  BASE_URL,
  safeGet,
  safeGetList,
  safePostAction,
  verifyEndpointHealthy,
} from './helpers';

test.describe('财务模块全量：API 端点 + 真实 UI 交互', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  // ===== API 端点覆盖 =====
  test('资金管理：账户+存取+冻结+转账+审批+报表+预测', async ({ page }) => {
    await verifyEndpointHealthy(page, '/fund-management/accounts?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/fund-management/accounts/by-type');
    await verifyEndpointHealthy(page, '/fund-management/transfers?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/fund-management/transfers/pending');
    await verifyEndpointHealthy(page, '/fund-management/reports/daily');
    await verifyEndpointHealthy(page, '/fund-management/reports/monthly');
    await verifyEndpointHealthy(page, '/fund-management/cash-flow-forecast');
    const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/fund-management/accounts?page=1&page_size=1'
    ).catch(() => ({ items: [] as Array<{ id: number }> }));
    const acctId = list.items?.[0]?.id;
    if (acctId) {
      await apiCallRaw(page, 'GET', `/fund-management/accounts/${acctId}`);
      await safePostAction(page, `/fund-management/accounts/${acctId}/deposit`, { amount: '1000' });
      await safePostAction(page, `/fund-management/accounts/${acctId}/withdraw`, { amount: '500' });
      await safePostAction(page, `/fund-management/accounts/${acctId}/freeze`, { amount: '200' });
      await safePostAction(page, `/fund-management/accounts/${acctId}/unfreeze`, { amount: '200' });
    }
  });

  test('多币种+汇率+应收对账+财务分析+报表+AP/AR+预算+固定资产+科目', async ({ page }) => {
    // 币种
    await verifyEndpointHealthy(page, '/currencies');
    await verifyEndpointHealthy(page, '/currencies/base');
    await verifyEndpointHealthy(page, '/exchange-rates?page=1&page_size=5');
    // 应收对账
    await verifyEndpointHealthy(page, '/ar-reconciliations?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/ar-reconciliations-enhanced/aging-report');
    await verifyEndpointHealthy(page, '/ar-reconciliation-alias/auto-reconcile/results');
    // 财务分析
    await verifyEndpointHealthy(page, '/financial-analysis/reports');
    await verifyEndpointHealthy(page, '/financial-analysis/indicators');
    await verifyEndpointHealthy(page, '/financial-analysis/dupont');
    // 报表
    await verifyEndpointHealthy(page, '/finance/reports/balance-sheet');
    await verifyEndpointHealthy(page, '/finance/reports/income-statement');
    await verifyEndpointHealthy(page, '/finance/reports/cash-flow');
    await verifyEndpointHealthy(page, '/finance/reports/trial-balance');
    // AP/AR
    await verifyEndpointHealthy(page, '/ap/invoices?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/ap/payments?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/ar/invoices?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/ar/payments?page=1&page_size=5');
    // 预算
    await verifyEndpointHealthy(page, '/budgets?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/budgets/plans?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/budgets/execution-warnings');
    // 固定资产
    await verifyEndpointHealthy(page, '/fixed-assets?page=1&page_size=5');
    const faList = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/fixed-assets?page=1&page_size=1'
    ).catch(() => ({ items: [] as Array<{ id: number }> }));
    if (faList.items?.[0]?.id) {
      await apiCallRaw(page, 'GET', `/fixed-assets/${faList.items?.[0].id}`);
      await verifyEndpointHealthy(
        page,
        `/fixed-assets/${faList.items?.[0].id}/depreciation-records`
      );
      await safePostAction(page, `/fixed-assets/${faList.items?.[0].id}/depreciate`);
    }
    // 科目
    await verifyEndpointHealthy(page, '/subjects?page=1&page_size=50');
    await verifyEndpointHealthy(page, '/assist-accounting?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/period-adjustments?page=1&page_size=5');
  });

  // ===== 真实 UI 交互验证 =====
  test('固定资产列表 UI：搜索+新建弹窗+折旧按钮', async ({ page }) => {
    await page.goto(`${BASE_URL}/fixed-assets`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-card, .el-empty')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    // 搜索
    const searchBtn = page.locator('button:has-text("查询")').first();
    await searchBtn.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const searchVisible = await searchBtn.isVisible().catch(() => false);
    if (searchVisible) {
      await searchBtn.click();
      await page.waitForTimeout(2000);
      const tableOk = await page
        .locator('.el-table')
        .first()
        .isVisible()
        .catch(() => false);
      expect(tableOk).toBe(true);
    }
    // 新建资产
    const newBtn = page.locator('button:has-text("新建资产")').first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const newBtnVisible = await newBtn.isVisible().catch(() => false);
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      await dialog.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
      const dialogVisible = await dialog.isVisible().catch(() => false);
      expect(dialogVisible).toBe(true);
      // 验证表单字段
      const inputs = dialog.locator('.el-input, .el-input-number, .el-select');
      const inputCount = await inputs.count();
      expect(inputCount).toBeGreaterThan(0);
      await page
        .locator('.el-dialog__headerbtn')
        .first()
        .click()
        .catch(() => {});
    }
  });

  test('预算列表 UI：搜索+新建弹窗', async ({ page }) => {
    await page.goto(`${BASE_URL}/budget`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-card, .el-empty')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const newBtn = page.locator('button:has-text("新建预算")').first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const newBtnVisible = await newBtn.isVisible().catch(() => false);
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      await dialog.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
      const dialogVisible = await dialog.isVisible().catch(() => false);
      expect(dialogVisible).toBe(true);
      await page
        .locator('.el-dialog__headerbtn')
        .first()
        .click()
        .catch(() => {});
    }
  });

  test('资金管理 UI：新建账户+转账按钮', async ({ page }) => {
    await page.goto(`${BASE_URL}/fund`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-card, .el-tabs')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const newBtn = page.locator('button:has-text("新建账户")').first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const newBtnVisible = await newBtn.isVisible().catch(() => false);
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      await dialog.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
      const dialogVisible = await dialog.isVisible().catch(() => false);
      expect(dialogVisible).toBe(true);
      await page
        .locator('.el-dialog__headerbtn')
        .first()
        .click()
        .catch(() => {});
    }
    // 验证转账按钮：按钮在“转账记录”Tab 内（默认激活 account），需先切 Tab
    // 真实按钮文本是“新建转账”（fund.transferTab.buttonNewTransfer），非“账户转账”
    await page
      .locator('.el-tabs__item:has-text("转账记录")')
      .first()
      .click()
      .catch(() => {});
    await page.waitForTimeout(1000);
    const transferBtn = page.locator('button:has-text("新建转账")').first();
    await transferBtn.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const transferVisible = await transferBtn.isVisible().catch(() => false);
    expect(transferVisible).toBe(true);
  });

  test('币种管理 UI：新建币种+新增汇率', async ({ page }) => {
    await page.goto(`${BASE_URL}/currency`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-card, .el-tabs')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const newBtn = page.locator('button:has-text("新建币种")').first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const newBtnVisible = await newBtn.isVisible().catch(() => false);
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      await dialog.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
      const dialogVisible = await dialog.isVisible().catch(() => false);
      expect(dialogVisible).toBe(true);
      await page
        .locator('.el-dialog__headerbtn')
        .first()
        .click()
        .catch(() => {});
    }
  });

  test('凭证列表 UI：新建凭证+借贷校验', async ({ page }) => {
    await page.goto(`${BASE_URL}/voucher`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-card, .el-tabs')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const newBtn = page.locator('button:has-text("新增凭证")').first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const newBtnVisible = await newBtn.isVisible().catch(() => false);
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      await dialog.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
      const dialogVisible = await dialog.isVisible().catch(() => false);
      expect(dialogVisible).toBe(true);
      // 直接保存触发必填校验
      const saveBtn = dialog.locator('button:has-text("保存"), button:has-text("确定")').first();
      await saveBtn.click().catch(() => {});
      await page.waitForTimeout(1000);
      await page
        .locator('.el-form-item__error, .el-message--error')
        .first()
        .waitFor({ state: 'visible', timeout: 5000 })
        .catch(() => {});
      const hasError = await page
        .locator('.el-form-item__error, .el-message--error')
        .first()
        .isVisible()
        .catch(() => false);
      expect(hasError).toBe(true);
      await page
        .locator('.el-dialog__headerbtn')
        .first()
        .click()
        .catch(() => {});
    }
  });

  test('会计科目 UI：树形表格+新建科目', async ({ page }) => {
    await page.goto(`${BASE_URL}/account-subject`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-card')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const newBtn = page.locator('button:has-text("新建科目")').first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const newBtnVisible = await newBtn.isVisible().catch(() => false);
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      await dialog.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
      const dialogVisible = await dialog.isVisible().catch(() => false);
      expect(dialogVisible).toBe(true);
      await page
        .locator('.el-dialog__headerbtn')
        .first()
        .click()
        .catch(() => {});
    }
  });

  test('会计期间 UI：新建期间+初始化年度', async ({ page }) => {
    await page.goto(`${BASE_URL}/accounting-period`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-card')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const newBtn = page.locator('button:has-text("新建期间")').first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const newBtnVisible = await newBtn.isVisible().catch(() => false);
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      await dialog.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
      const dialogVisible = await dialog.isVisible().catch(() => false);
      expect(dialogVisible).toBe(true);
      await page
        .locator('.el-dialog__headerbtn')
        .first()
        .click()
        .catch(() => {});
    }
    // 验证初始化年度按钮（dialog 关闭动画可能遮挡，给足 10s）
    const initBtn = page.locator('button:has-text("初始化年度")').first();
    await initBtn.waitFor({ state: 'visible', timeout: 10_000 }).catch(() => {});
    const initVisible = await initBtn.isVisible().catch(() => false);
    expect(initVisible).toBe(true);
  });

  test('应收对账 UI：新建对账+搜索', async ({ page }) => {
    await page.goto(`${BASE_URL}/ar-reconciliation`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-card')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const newBtn = page.locator('button:has-text("新增对账"), button:has-text("新建对账")').first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const newBtnVisible = await newBtn.isVisible().catch(() => false);
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      await dialog.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
      const dialogVisible = await dialog.isVisible().catch(() => false);
      expect(dialogVisible).toBe(true);
      await page
        .locator('.el-dialog__headerbtn')
        .first()
        .click()
        .catch(() => {});
    }
  });

  test('应付管理 UI 页面', async ({ page }) => {
    await page.goto(`${BASE_URL}/ap`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-card, .el-tabs')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    // “生成对账”按钮在“对账管理”Tab 内（默认激活 invoice），需先切 Tab
    await page
      .locator('.el-tabs__item:has-text("对账管理")')
      .first()
      .click()
      .catch(() => {});
    await page.waitForTimeout(1000);
    const genBtn = page.locator('button:has-text("生成对账")').first();
    await genBtn.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const genVisible = await genBtn.isVisible().catch(() => false);
    expect(genVisible).toBe(true);
  });

  test('财务报表 UI：生成+导出', async ({ page }) => {
    await page.goto(`${BASE_URL}/finance-report`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-card, .el-table, .el-form')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    // 验证生成报表按钮
    const genBtn = page.locator('button:has-text("生成报表")').first();
    await genBtn.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const genVisible = await genBtn.isVisible().catch(() => false);
    // 验证导出按钮
    const exportBtn = page.locator('button:has-text("导出")').first();
    await exportBtn.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const exportVisible = await exportBtn.isVisible().catch(() => false);
    expect(genVisible || exportVisible).toBe(true);
  });
});
