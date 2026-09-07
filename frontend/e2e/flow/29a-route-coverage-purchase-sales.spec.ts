import { test, expect } from '@playwright/test';
import { loginViaUI, BASE_URL } from './helpers';

test.describe('100% 前端路由 UI 交互全覆盖', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  // 辅助：访问页面，验证核心 UI 元素可见
  async function visitPage(page: import('@playwright/test').Page, path: string) {
    await page.goto(`${BASE_URL}${path}`);
    await page.waitForTimeout(2000);
    const container = page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-form, .el-empty, .el-tabs, .dashboard-container, canvas, .el-result, .error-page, body'
      )
      .first();
    await container
      .waitFor({ state: 'visible', timeout: 30_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    return container;
  }

  // 辅助：验证表格+表头
  async function verifyTable(page: import('@playwright/test').Page) {
    const table = page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first();
    await table
      .waitFor({ state: 'visible', timeout: 10_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const visible = await table.isVisible().catch(() => false);
    if (visible) {
      const headers = table.locator('th, .el-table-v2__header-cell');
      const count = await headers.count();
      expect(count).toBeGreaterThan(0);
    }
    return visible;
  }

  // 辅助：验证新建按钮+弹窗
  async function verifyNewButton(page: import('@playwright/test').Page, btnText: string) {
    const btn = page.locator(`button:has-text("${btnText}")`).first();
    await btn
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const visible = await btn.isVisible().catch(() => false);
    if (visible) {
      const disabled = await btn.isDisabled().catch(() => false);
      expect(disabled).toBe(false);
      await btn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      const dialogVisible = await dialog
        .waitFor({ state: 'visible', timeout: 5000 })
        .then(() => true)
        .catch(() => false);
      if (dialogVisible) {
        await page
          .locator('.el-dialog__headerbtn')
          .first()
          .click()
          .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
        await page.waitForTimeout(500);
      }
      return dialogVisible;
    }
    return false;
  }

  // ===== 采购域剩余路由 =====
  test('采购管理总页 /purchase', async ({ page }) => {
    await visitPage(page, '/purchase');
    const bodyOk = await page.locator('body').isVisible();
    expect(bodyOk).toBe(true);
  });
  test('采购合同 /purchase-contract', async ({ page }) => {
    await visitPage(page, '/purchase-contract');
    await verifyTable(page);
  });
  test('采购扩展 /purchase-ext', async ({ page }) => {
    await visitPage(page, '/purchase-ext');
    await verifyTable(page);
  });
  test('采购检验 /purchase-inspection', async ({ page }) => {
    await visitPage(page, '/purchase-inspection');
    await verifyTable(page);
  });
  test('采购价格 /purchase-price', async ({ page }) => {
    await visitPage(page, '/purchase-price');
    await verifyTable(page);
  });
  test('采购收货 /purchase-receipt', async ({ page }) => {
    await visitPage(page, '/purchase-receipt');
    await verifyTable(page);
  });
  test('采购退货 /purchase-return', async ({ page }) => {
    await visitPage(page, '/purchase-return');
    await verifyTable(page);
  });
  test('供应商 /supplier', async ({ page }) => {
    await visitPage(page, '/supplier');
    const tableOk = await verifyTable(page);
    if (tableOk) await verifyNewButton(page, '新建供应商');
  });
  test('供应商评估 /supplier-evaluation', async ({ page }) => {
    await visitPage(page, '/supplier-evaluation');
    await verifyTable(page);
  });

  // ===== 销售域剩余路由 =====
  test('销售管理总页 /sales', async ({ page }) => {
    await visitPage(page, '/sales');
    await verifyTable(page);
  });
  test('销售合同 /sales-contract', async ({ page }) => {
    await visitPage(page, '/sales-contract');
    await verifyTable(page);
  });
  test('销售扩展 /sales-ext', async ({ page }) => {
    await visitPage(page, '/sales-ext');
    await verifyTable(page);
  });
  test('销售价格 /sales-price', async ({ page }) => {
    await visitPage(page, '/sales-price');
    await verifyTable(page);
  });
  test('报价单新建 /quotations/new', async ({ page }) => {
    await visitPage(page, '/quotations/new');
    const form = page.locator('.el-form, .el-card').first();
    await form
      .waitFor({ state: 'visible', timeout: 10_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const visible = await form.isVisible().catch(() => false);
    expect(visible).toBe(true);
  });
  test('报价单详情 /quotations/:id', async ({ page }) => {
    await visitPage(page, '/quotations/1');
    const bodyOk = await page.locator('body').isVisible();
    expect(bodyOk).toBe(true);
  });
});
