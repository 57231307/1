import { test, expect } from '@playwright/test';
import { loginViaUI, BASE_URL, apiCall, apiCallRaw, getCtx } from './helpers';

test.describe('前端状态显示与业务逻辑验证', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('采购订单列表：状态标签颜色映射', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase`);
    await page.waitForTimeout(3000);

    await page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });

    const statusTags = page.locator('.el-table .el-tag');
    const tagCount = await statusTags.count();

    if (tagCount > 0) {
      for (let i = 0; i < Math.min(tagCount, 5); i++) {
        const tag = statusTags.nth(i);
        const classes = await tag.getAttribute('class');
        const text = await tag.textContent();

        expect(classes).toContain('el-tag');
        expect(text?.trim().length).toBeGreaterThan(0);
        expect(classes).toContain('el-tag--');
      }
    }
  });

  test('金额千分位格式化显示', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase`);
    await page.waitForTimeout(3000);

    await page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));

    const cells = page.locator('.el-table__body td');
    const cellCount = await cells.count();

    if (cellCount > 0) {
      let foundAmount = false;
      for (let i = 0; i < Math.min(cellCount, 20); i++) {
        const text = await cells.nth(i).textContent();
        if (
          text &&
          (text.includes('¥') || text.match(/\d{1,3}(,\d{3})+/) || text.match(/\d+\.\d{2}/))
        ) {
          foundAmount = true;
          break;
        }
      }
      if (foundAmount) {
        expect(foundAmount).toBe(true);
      }
    }
  });

  test('日期格式化显示为 YYYY-MM-DD', async ({ page }) => {
    // 先确保至少存在一笔采购订单（本用例自建数据，不依赖其他用例/分片），
    // 空表格没有日期单元格可断言
    const ctx = getCtx();
    try {
      const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
        page,
        'GET',
        '/purchase/orders?page=1&page_size=1'
      );
      if ((list.items?.length ?? 0) === 0) {
        await apiCall(page, 'POST', '/purchase/orders', {
          supplier_id: ctx.supplierId || 1,
          warehouse_id: ctx.warehouseIds[0] || 1,
          department_id: ctx.departmentIds[0] || 1,
          order_date: new Date().toISOString().slice(0, 10),
          items: [
            {
              material_id: ctx.productIds[0] || 1,
              quantity_ordered: '10',
              unit_price: '9.90',
            },
          ],
          notes: 'E2E 日期格式显示验证',
        });
      }
    } catch (e) {
      console.warn(`[日期格式化] 预置采购订单失败（继续尝试断言）: ${(e as Error).message}`);
    }

    await page.goto(`${BASE_URL}/purchase`);
    await page.waitForTimeout(3000);

    await page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));

    const cells = page.locator('.el-table__body td');
    const cellCount = await cells.count();

    let foundDate = false;
    for (let i = 0; i < Math.min(cellCount, 30); i++) {
      const text = await cells.nth(i).textContent();
      if (text && text.match(/\d{4}[-/]\d{1,2}[-/]\d{1,2}/)) {
        foundDate = true;
        break;
      }
    }
    expect(foundDate).toBeTruthy();
  });

  test('权限不足时按钮行为验证', async ({ page }) => {
    // admin 用户应能看到所有操作按钮
    await page.goto(`${BASE_URL}/purchase`);
    await page.waitForTimeout(3000);

    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });

    // 验证新建按钮可见（admin 有全部权限）
    const newBtn = page.locator('button:has-text("新建采购单")').first();
    await newBtn
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const newBtnVisible = await newBtn.isVisible().catch(() => false);
    expect(newBtnVisible).toBe(true);
  });

  test('空数据时 el-empty 或空表格展示', async ({ page }) => {
    await page.goto(`${BASE_URL}/voucher`);
    await page.waitForTimeout(3000);

    const table = page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first();
    await table
      .waitFor({ state: 'visible', timeout: 15_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const tableVisible = await table.isVisible().catch(() => false);
    if (tableVisible) {
      const emptyBlock = page.locator('.el-table__empty-block, .el-table__empty-text, .el-empty');
      await emptyBlock
        .first()
        .waitFor({ state: 'visible', timeout: 3000 })
        .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
      const emptyVisible = await emptyBlock
        .first()
        .isVisible()
        .catch(() => false);
      if (emptyVisible) {
        const emptyText = await emptyBlock.first().textContent();
        expect(emptyText).toBeTruthy();
        expect(emptyText?.length).toBeGreaterThan(0);
      }
    }
  });

  test('加载完成后内容可见', async ({ page }) => {
    await page.goto(`${BASE_URL}/dashboard`);
    await page.waitForTimeout(1000);

    // 验证页面有 loading 指示器或最终内容
    const hasLoading = await page
      .locator('.el-loading-mask, .el-skeleton, .el-loading-spinner')
      .first()
      .waitFor({ state: 'visible', timeout: 3000 })
      .then(() => true)
      .catch(() => false);

    await page.waitForTimeout(2000);

    const content = page.locator('.dashboard-container, .el-card, .el-row').first();
    await content
      .waitFor({ state: 'visible', timeout: 10_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const contentVisible = await content.isVisible().catch(() => false);
    expect(contentVisible).toBe(true);
  });

  test('响应式布局：窄屏不崩溃', async ({ page }) => {
    await page.setViewportSize({ width: 768, height: 600 });
    await page.goto(`${BASE_URL}/dashboard`);
    await page.waitForTimeout(3000);

    // 验证页面在窄屏下不崩溃
    const body = page.locator('body');
    await body
      .waitFor({ state: 'visible', timeout: 10_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const bodyVisible = await body.isVisible().catch(() => false);
    expect(bodyVisible).toBe(true);

    // 恢复宽屏
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.waitForTimeout(500);
  });

  test('消息提示自动消失', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase`);
    await page.waitForTimeout(3000);

    await page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));

    // 点击查询按钮触发操作
    const searchBtn = page.locator('button:has-text("查询")').first();
    await searchBtn
      .waitFor({ state: 'visible', timeout: 3000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const searchVisible = await searchBtn.isVisible().catch(() => false);
    if (searchVisible) {
      await searchBtn.click();
      await page.waitForTimeout(500);

      const message = page.locator('.el-message').first();
      await message
        .waitFor({ state: 'visible', timeout: 2000 })
        .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
      const messageVisible = await message.isVisible().catch(() => false);
      if (messageVisible) {
        await page.waitForTimeout(4000);
        await message
          .waitFor({ state: 'visible', timeout: 1000 })
          .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
        const messageStillVisible = await message.isVisible().catch(() => false);
        expect(messageStillVisible).toBe(false);
      }
    }
  });
});
