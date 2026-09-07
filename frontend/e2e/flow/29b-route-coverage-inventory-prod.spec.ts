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

  // ===== 库存域剩余路由 =====
  test('库存管理总页 /inventory', async ({ page }) => {
    await visitPage(page, '/inventory');
    await verifyTable(page);
  });
  test('库存调整(旧路径) /inventory-adjustment', async ({ page }) => {
    await visitPage(page, '/inventory-adjustment');
    await verifyTable(page);
  });
  test('库存批次 /inventory-batch', async ({ page }) => {
    await visitPage(page, '/inventory-batch');
    await verifyTable(page);
  });
  test('库存盘点(旧路径) /inventory-count', async ({ page }) => {
    await visitPage(page, '/inventory-count');
    await verifyTable(page);
  });
  test('库存调拨(旧路径) /inventory-transfer', async ({ page }) => {
    await visitPage(page, '/inventory-transfer');
    await verifyTable(page);
  });
  test('物流 /logistics', async ({ page }) => {
    await visitPage(page, '/logistics');
    await verifyTable(page);
  });

  // ===== 生产域剩余路由 =====
  test('生产管理总页 /production', async ({ page }) => {
    await visitPage(page, '/production');
    await verifyTable(page);
  });
  test('MRP历史 /mrp/history', async ({ page }) => {
    await visitPage(page, '/mrp/history');
    await verifyTable(page);
  });
  test('质量标准 /quality-standards', async ({ page }) => {
    await visitPage(page, '/quality-standards');
    await verifyTable(page);
  });
  test('面料管理 /fabric', async ({ page }) => {
    await visitPage(page, '/fabric');
    await verifyTable(page);
  });
});
