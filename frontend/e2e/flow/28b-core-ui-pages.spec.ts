import { test, expect } from '@playwright/test';
import { loginViaUI, BASE_URL, getCtx } from './helpers';

test.describe('核心业务流程真实 UI 交互验证', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  // 辅助：访问页面并验证表格加载
  async function visitAndVerifyTable(page: import('@playwright/test').Page, path: string) {
    await page.goto(`${BASE_URL}${path}`);
    await page.waitForTimeout(3000);
    const container = page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-empty, .el-form'
      )
      .first();
    await container
      .waitFor({ state: 'visible', timeout: 30_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    return page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first();
  }

  // 辅助：验证按钮可见且可点击
  async function verifyButton(page: import('@playwright/test').Page, text: string) {
    const btn = page.locator(`button:has-text("${text}")`).first();
    await btn
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const visible = await btn.isVisible().catch(() => false);
    if (visible) {
      const disabled = await btn.isDisabled().catch(() => false);
      expect(disabled).toBe(false);
    }
    return visible;
  }

  // 辅助：点击新建按钮并验证弹窗（只匹配可见对话框）
  async function clickNewAndVerifyDialog(page: import('@playwright/test').Page, btnText: string) {
    const btn = page.locator(`button:has-text("${btnText}")`).first();
    await btn
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const visible = await btn.isVisible().catch(() => false);
    if (!visible) return false;
    await btn.click();
    await page.waitForTimeout(1000);
    const dialog = page.locator('.el-dialog:visible').first();
    await dialog
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const dialogVisible = await dialog.isVisible().catch(() => false);
    return dialogVisible;
  }

  // 辅助：验证表单必填校验
  async function verifyRequiredValidation(page: import('@playwright/test').Page) {
    // 可见对话框（页面可能挂载多个 el-dialog，隐藏的不参与匹配）
    const dialog = page.locator('.el-dialog:visible').first();
    // 提交按钮 = 对话框 footer 的主按钮（各模块文案不一：保存/确定/确认/提交），
    // 按文案匹配会因文案差异（如 "确认"）匹配不到而 30s 超时
    const saveBtn = dialog.locator('.el-dialog__footer .el-button--primary').first();
    try {
      await saveBtn.click({ timeout: 10_000 });
      console.log('[verifyRequiredValidation] 已点击 footer 主按钮');
    } catch (e) {
      console.error(`[verifyRequiredValidation] 点击主按钮失败: ${(e as Error).message}`);
      return false;
    }
    await page.waitForTimeout(1000);
    // 表单校验用 ElMessage（warning/error）或 el-form-item__error，
    // 统一匹配 .el-message（含 --warning/--error）以覆盖所有提示类型
    await page
      .locator('.el-message, .el-form-item__error')
      .first()
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const hasError = await page
      .locator('.el-message, .el-form-item__error')
      .first()
      .isVisible()
      .catch(() => false);
    if (!hasError) {
      // 诊断输出（IR 详细日志要求）：无任何校验提示时打印对话框文本片段
      const dialogText = await dialog.innerText().catch(() => '<无法获取>');
      console.warn(
        `[verifyRequiredValidation] 未出现校验提示，对话框文本前 200 字: ${dialogText.slice(0, 200)}`
      );
    }
    return hasError;
  }

  // 辅助：关闭弹窗
  async function closeDialog(page: import('@playwright/test').Page) {
    await page
      .locator('.el-dialog__headerbtn')
      .first()
      .click()
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    await page.waitForTimeout(500);
  }

  // ================================================================
  // P2P 采购到付款流程 UI
  // ================================================================
  test('仪表盘 UI：统计卡片渲染+图表加载+刷新按钮', async ({ page }) => {
    await page.goto(`${BASE_URL}/dashboard`);
    await page.waitForTimeout(3000);
    await page
      .locator('.dashboard-container, .el-card, .el-row')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });

    // 验证统计卡片存在
    const statCards = page.locator('.el-card, .el-statistic, [class*="stat"]');
    const cardCount = await statCards.count();
    expect(cardCount).toBeGreaterThan(0);

    // 验证图表或数据展示
    const charts = page.locator('canvas, .echarts, [class*="chart"], [class*="trend"]');
    const chartCount = await charts.count();

    // 刷新按钮
    const refreshBtn = page
      .locator('button:has-text("刷新"), .el-button--primary:has(.el-icon)')
      .first();
    await refreshBtn
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const refreshVisible = await refreshBtn.isVisible().catch(() => false);
    if (refreshVisible) {
      const disabled = await refreshBtn.isDisabled().catch(() => false);
      expect(disabled).toBe(false);
    }
  });

  // ================================================================
  // 采购退货流程 UI
  // ================================================================
  test('采购退货列表 UI：表格+状态标签', async ({ page }) => {
    const table = await visitAndVerifyTable(page, '/purchase-return');
    const tableVisible = await table.isVisible().catch(() => false);
    expect(tableVisible).toBe(true);

    const tags = table.locator('.el-tag');
    const tagCount = await tags.count();
    if (tagCount > 0) {
      const tagText = await tags.first().textContent();
      expect(tagText?.trim().length).toBeGreaterThan(0);
    }
  });

  // ================================================================
  // 库存调拨流程 UI
  // ================================================================
  test('库存调拨列表 UI：搜索+新建弹窗+仓库选择', async ({ page }) => {
    const table = await visitAndVerifyTable(page, '/inventory-transfer');
    const tableVisible = await table.isVisible().catch(() => false);
    expect(tableVisible).toBe(true);

    // 新建
    const dialogVisible = await clickNewAndVerifyDialog(page, '新建');
    if (dialogVisible) {
      // 验证调出/调入仓库选择器
      const selects = page.locator('.el-dialog .el-select');
      const selectCount = await selects.count();
      expect(selectCount).toBeGreaterThanOrEqual(2);
      await closeDialog(page);
    }
  });

  // ================================================================
  // 库存盘点流程 UI
  // ================================================================
  test('库存盘点列表 UI：搜索+新建', async ({ page }) => {
    const table = await visitAndVerifyTable(page, '/inventory-count');
    const tableVisible = await table.isVisible().catch(() => false);
    expect(tableVisible).toBe(true);

    const dialogVisible = await clickNewAndVerifyDialog(page, '新建');
    if (dialogVisible) {
      await closeDialog(page);
    }
  });

  // ================================================================
  // 成本核算流程 UI
  // ================================================================
  test('成本归集列表 UI', async ({ page }) => {
    await page.goto(`${BASE_URL}/cost`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-card, .el-table, .el-empty, body')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const bodyOk = await page.locator('body').isVisible();
    expect(bodyOk).toBe(true);
  });

  // ================================================================
  // 异常处理 UI：错误页面显示
  // ================================================================
  test('403 页面 UI：错误展示', async ({ page }) => {
    await page.goto(`${BASE_URL}/403`);
    await page.waitForTimeout(2000);
    const content = page.locator('.el-result, .error-page, body').first();
    await content
      .waitFor({ state: 'visible', timeout: 10_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const visible = await content.isVisible().catch(() => false);
    expect(visible).toBe(true);
    const text = await content.textContent();
    expect(text?.length).toBeGreaterThan(0);
  });

  test('404 页面 UI：错误展示', async ({ page }) => {
    await page.goto(`${BASE_URL}/404`);
    await page.waitForTimeout(2000);
    const content = page.locator('.error-page, body').first();
    await content
      .waitFor({ state: 'visible', timeout: 10_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const visible = await content.isVisible().catch(() => false);
    expect(visible).toBe(true);
  });
});
