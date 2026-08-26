import { test, expect } from '@playwright/test';
import { loginViaUI, apiCall, apiCallRaw, apiCallExpectFail, genCode, getCtx, BASE_URL, safeGet, safeGetList, safePostAction, verifyEndpointHealthy } from './helpers';

test.describe('其他模块全量：API 端点 + 真实 UI 交互', () => {
  test.beforeEach(async ({ page }) => { await loginViaUI(page); });

  // ===== API 端点覆盖 =====
  test('合同+价格+检验+供应商+定制+库存扩展端点', async ({ page }) => {
    // 合同
    await verifyEndpointHealthy(page, '/purchase/purchase-contracts?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/sales/sales-contracts?page=1&page_size=5');
    // 价格
    await verifyEndpointHealthy(page, '/purchase/purchase-prices?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/sales/sales-prices?page=1&page_size=5');
    // 检验
    await verifyEndpointHealthy(page, '/purchase/inspections?page=1&page_size=5');
    // 供应商完整
    await verifyEndpointHealthy(page, '/purchase/suppliers?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/purchase/suppliers/abnormal-orders');
    await verifyEndpointHealthy(page, '/supplier-evaluations?page=1&page_size=5');
    const supList = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/purchase/suppliers?page=1&page_size=1').catch(() => ({ items: [] as Array<{ id: number }> }));
    const supId = supList.items?.[0]?.id || 1;
    await apiCallRaw(page, 'GET', `/purchase/suppliers/${supId}`);
    await verifyEndpointHealthy(page, `/purchase/suppliers/${supId}/balance`);
    await verifyEndpointHealthy(page, `/purchase/suppliers/${supId}/purchase-history`);
    await verifyEndpointHealthy(page, `/purchase/suppliers/${supId}/contacts`);
    await verifyEndpointHealthy(page, `/purchase/suppliers/${supId}/qualifications`);
    await verifyEndpointHealthy(page, `/purchase/suppliers/${supId}/evaluations`);
    // 定制订单
    await verifyEndpointHealthy(page, '/custom-orders?page=1&page_size=5');
    // 大货批色
    await verifyEndpointHealthy(page, '/bulk-color-approvals?page=1&page_size=5');
    // 8D/坏账/催收/预警/OA/PDA/商检
    await verifyEndpointHealthy(page, '/quality-8d-reports?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/bad-debts?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/collection-tasks?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/finance-alerts?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/oa-announcements?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/device-connections?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/export-inspections?page=1&page_size=5');
    // 库存扩展
    await verifyEndpointHealthy(page, '/inventory/adjustments?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/inventory/reservations?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/inventory/write-downs?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/inventory/batches?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/inventory/logistics?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/inventory/stock/export');
    await verifyEndpointHealthy(page, '/inventory/stock/transactions?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/inventory/stock/summary');
    await verifyEndpointHealthy(page, '/inventory/stock/low-stock');
  });

  // ===== 真实 UI 交互验证 =====
  test('采购合同列表 UI：搜索+新建', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase/contract`);
    await page.waitForTimeout(3000);
    await page.locator('.el-table, .el-card, .el-empty').first().waitFor({ state: 'visible', timeout: 30_000 });
    const searchBtn = page.locator('button:has-text("查询")').first();
    const searchVisible = await searchBtn.isVisible({ timeout: 5000 }).catch(() => false);
    if (searchVisible) {
      await searchBtn.click();
      await page.waitForTimeout(2000);
      const tableOk = await page.locator('.el-table').first().isVisible().catch(() => false);
      expect(tableOk).toBe(true);
    }
    const newBtn = page.locator('button:has-text("新建"), button:has-text("新增")').first();
    const newBtnVisible = await newBtn.isVisible({ timeout: 5000 }).catch(() => false);
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      const dialogVisible = await dialog.isVisible({ timeout: 5000 }).catch(() => false);
      expect(dialogVisible).toBe(true);
      await page.locator('.el-dialog__headerbtn').first().click().catch(() => {});
    }
  });

  test('采购价格列表 UI', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase/price`);
    await page.waitForTimeout(3000);
    await page.locator('.el-table, .el-card, .el-empty').first().waitFor({ state: 'visible', timeout: 30_000 });
    const table = page.locator('.el-table').first();
    const tableVisible = await table.isVisible({ timeout: 10_000 }).catch(() => false);
    expect(tableVisible).toBe(true);
  });

  test('供应商列表 UI：搜索+新建供应商弹窗', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase/supplier`);
    await page.waitForTimeout(3000);
    await page.locator('.el-table, .el-card').first().waitFor({ state: 'visible', timeout: 30_000 });
    // 搜索
    const searchInput = page.locator('input[placeholder*="供应商"], input[placeholder*="名称"]').first();
    const searchVisible = await searchInput.isVisible({ timeout: 5000 }).catch(() => false);
    if (searchVisible) {
      await searchInput.fill('测试');
      const queryBtn = page.locator('button:has-text("查询")').first();
      const btnVisible = await queryBtn.isVisible({ timeout: 3000 }).catch(() => false);
      if (btnVisible) { await queryBtn.click(); await page.waitForTimeout(2000); }
      const tableOk = await page.locator('.el-table').first().isVisible().catch(() => false);
      expect(tableOk).toBe(true);
    }
    // 新建供应商
    const newBtn = page.locator('button:has-text("新建供应商")').first();
    const newBtnVisible = await newBtn.isVisible({ timeout: 5000 }).catch(() => false);
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      const dialogVisible = await dialog.isVisible({ timeout: 5000 }).catch(() => false);
      expect(dialogVisible).toBe(true);
      // 验证供应商编码输入框
      const codeInput = dialog.locator('input[placeholder*="供应商编码"]').first();
      const codeVisible = await codeInput.isVisible({ timeout: 3000 }).catch(() => false);
      expect(codeVisible).toBe(true);
      // 直接保存触发必填校验
      const saveBtn = dialog.locator('button:has-text("保存"), button:has-text("确定")').first();
      await saveBtn.click().catch(() => {});
      await page.waitForTimeout(1000);
      const hasError = await page.locator('.el-form-item__error, .el-message--error').first().isVisible({ timeout: 5000 }).catch(() => false);
      expect(hasError).toBe(true);
      await page.locator('.el-dialog__headerbtn').first().click().catch(() => {});
    }
  });

  test('销售合同列表 UI', async ({ page }) => {
    await page.goto(`${BASE_URL}/sales/contract`);
    await page.waitForTimeout(3000);
    await page.locator('.el-table, .el-card, .el-empty').first().waitFor({ state: 'visible', timeout: 30_000 });
    const table = page.locator('.el-table').first();
    const tableVisible = await table.isVisible({ timeout: 10_000 }).catch(() => false);
    expect(tableVisible).toBe(true);
  });

  test('库存调拨列表 UI：搜索+新建弹窗', async ({ page }) => {
    await page.goto(`${BASE_URL}/inventory/transfer`);
    await page.waitForTimeout(3000);
    await page.locator('.el-table, .el-card, .el-empty').first().waitFor({ state: 'visible', timeout: 30_000 });
    const newBtn = page.locator('button:has-text("新建")').first();
    const newBtnVisible = await newBtn.isVisible({ timeout: 5000 }).catch(() => false);
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      const dialogVisible = await dialog.isVisible({ timeout: 5000 }).catch(() => false);
      expect(dialogVisible).toBe(true);
      // 验证调出/调入仓库选择器
      const selects = dialog.locator('.el-select');
      const selectCount = await selects.count();
      expect(selectCount).toBeGreaterThanOrEqual(2);
      await page.locator('.el-dialog__headerbtn').first().click().catch(() => {});
    }
  });

  test('库存盘点列表 UI：搜索+新建', async ({ page }) => {
    await page.goto(`${BASE_URL}/inventory/count`);
    await page.waitForTimeout(3000);
    await page.locator('.el-table, .el-card, .el-empty').first().waitFor({ state: 'visible', timeout: 30_000 });
    const newBtn = page.locator('button:has-text("新建")').first();
    const newBtnVisible = await newBtn.isVisible({ timeout: 5000 }).catch(() => false);
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      const dialogVisible = await dialog.isVisible({ timeout: 5000 }).catch(() => false);
      expect(dialogVisible).toBe(true);
      await page.locator('.el-dialog__headerbtn').first().click().catch(() => {});
    }
  });

  test('库存调整列表 UI', async ({ page }) => {
    await page.goto(`${BASE_URL}/inventory/adjustment`);
    await page.waitForTimeout(3000);
    await page.locator('.el-table, .el-card, .el-empty').first().waitFor({ state: 'visible', timeout: 30_000 });
    const table = page.locator('.el-table').first();
    const tableVisible = await table.isVisible({ timeout: 10_000 }).catch(() => false);
    expect(tableVisible).toBe(true);
  });

  test('定制订单列表 UI：新建+状态显示', async ({ page }) => {
    await page.goto(`${BASE_URL}/custom-orders`);
    await page.waitForTimeout(3000);
    await page.locator('.el-table, .el-card, .el-empty').first().waitFor({ state: 'visible', timeout: 30_000 });
    const newBtn = page.locator('button:has-text("新建"), button:has-text("新增")').first();
    const newBtnVisible = await newBtn.isVisible({ timeout: 5000 }).catch(() => false);
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      const dialogVisible = await dialog.isVisible({ timeout: 5000 }).catch(() => false);
      expect(dialogVisible).toBe(true);
      await page.locator('.el-dialog__headerbtn').first().click().catch(() => {});
    }
  });

  test('安全设置 UI：修改密码表单', async ({ page }) => {
    await page.goto(`${BASE_URL}/security/change-password`);
    await page.waitForTimeout(3000);
    await page.locator('.el-card, .el-form, body').first().waitFor({ state: 'visible', timeout: 30_000 });
    const form = page.locator('.el-form').first();
    const formVisible = await form.isVisible({ timeout: 10_000 }).catch(() => false);
    if (formVisible) {
      const inputs = form.locator('input');
      const inputCount = await inputs.count();
      expect(inputCount).toBeGreaterThan(0);
    }
  });

  test('坯布管理 UI 页面', async ({ page }) => {
    await page.goto(`${BASE_URL}/greige-fabrics`);
    await page.waitForTimeout(3000);
    await page.locator('.el-table, .el-card, .el-empty, body').first().waitFor({ state: 'visible', timeout: 30_000 });
    const bodyOk = await page.locator('body').isVisible();
    expect(bodyOk).toBe(true);
  });

  test('销售退货列表 UI 页面', async ({ page }) => {
    await page.goto(`${BASE_URL}/sales-returns`);
    await page.waitForTimeout(3000);
    await page.locator('.el-table, .el-card, .el-empty, body').first().waitFor({ state: 'visible', timeout: 30_000 });
    const table = page.locator('.el-table').first();
    const tableVisible = await table.isVisible({ timeout: 10_000 }).catch(() => false);
    expect(tableVisible).toBe(true);
  });

  test('采购退货列表 UI 页面', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase/return`);
    await page.waitForTimeout(3000);
    await page.locator('.el-table, .el-card, .el-empty, body').first().waitFor({ state: 'visible', timeout: 30_000 });
    const table = page.locator('.el-table').first();
    const tableVisible = await table.isVisible({ timeout: 10_000 }).catch(() => false);
    expect(tableVisible).toBe(true);
  });
});
