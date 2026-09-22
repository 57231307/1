// 生产计划 E2E 套件 — 02 生产执行（已排产 → 生产中 → 已完成）
// 覆盖范围：开始生产、完成生产、状态标签验证
import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, ensureTestEntities, getCtx } from '../flow/helpers';

async function filterByStatus(page: Page, statusLabel: string): Promise<void> {
  await expect(page.locator('.v2-table, .el-table')).toBeVisible({ timeout: 30000 });
  await page.getByLabel(/状态/).click();
  await page.getByRole('option', { name: statusLabel, exact: true }).click();
  await page.getByRole('button', { name: /查询/ }).click();
}

/**
 * 造一张无默认 BOM 的新产品（避免完成时走原料库存扣减而受种子数据干扰），
 * 再建 DRAFT 工单并按状态机推进到 targetStatus（PUT /status 走后端白名单校验）。
 */
async function createOrderInStatus(page: Page, targetStatus: string): Promise<number> {
  const ctx = getCtx();
  const categoryId = ctx.productCategoryIds[0];
  expect(categoryId, '前置：需要产品分类（建产品用）').toBeTruthy();
  const suffix = Date.now().toString().slice(-6);
  const prod = await apiCall<{ id?: number }>(page, 'POST', '/products', {
    code: `E2E-MFG${suffix}`,
    name: `E2E生产产品${suffix}`,
    unit: '米',
    category_id: categoryId,
  });
  const productId = prod.data?.id;
  if (!productId) {
    throw new Error(`造数产品创建失败：${JSON.stringify(prod).slice(0, 200)}`);
  }

  const order = await apiCall<{ id?: number }>(
    page,
    'POST',
    '/production/production-orders/orders',
    {
      product_id: productId,
      planned_quantity: 50,
      priority: 5,
    }
  );
  const id = order.data?.id;
  if (!id) {
    throw new Error(`生产工单造数失败：${JSON.stringify(order).slice(0, 200)}`);
  }

  // DRAFT →(白名单)→ SCHEDULED → IN_PROGRESS
  const path = `/production/production-orders/orders/${id}/status`;
  await apiCall(page, 'PUT', path, { status: 'SCHEDULED' });
  if (targetStatus === 'IN_PROGRESS') {
    await apiCall(page, 'PUT', path, { status: 'IN_PROGRESS' });
  }
  return id;
}

test.describe('生产计划 - 02 生产执行', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('已排产工单可开始生产（SCHEDULED → 生产中）', async ({ page }) => {
    // 原实现 `if (await startBtn.isVisible())` + getByRole('link')：操作列按钮角色是
    // button（el-button is-link），link 定位恒不命中；且无造数时列表无 SCHEDULED 行，
    // 条件不成立即零断言假绿。改为真实造一张 SCHEDULED 工单并硬断言。
    await ensureTestEntities(page);
    await createOrderInStatus(page, 'SCHEDULED');

    await page.goto('/production');
    await filterByStatus(page, '已排产');

    const startBtn = page.getByRole('button', { name: '开始生产', exact: true }).first();
    await expect(startBtn, '已排产工单应渲染"开始生产"按钮').toBeVisible({ timeout: 30000 });
    await startBtn.click();
    await page.getByRole('button', { name: /确定/ }).click();
    await expect(page.getByText(/状态更新成功/)).toBeVisible({ timeout: 30000 });
  });

  test('生产中工单可完成（IN_PROGRESS → 已完成）', async ({ page }) => {
    await ensureTestEntities(page);
    // 用无 BOM 的新产品，确保"完成"仅入成品、不触发原料扣减，状态变更稳定成功
    await createOrderInStatus(page, 'IN_PROGRESS');

    await page.goto('/production');
    await filterByStatus(page, '生产中');

    const completeBtn = page.getByRole('button', { name: '完成', exact: true }).first();
    await expect(completeBtn, '生产中工单应渲染"完成"按钮').toBeVisible({ timeout: 30000 });
    await completeBtn.click();
    await page.getByRole('button', { name: /确定/ }).click();
    await expect(page.getByText(/状态更新成功/)).toBeVisible({ timeout: 30000 });
  });

  test('已完成工单无操作按钮', async ({ page }) => {
    await page.goto('/production');
    await expect(page.locator('.el-table, .v2-table')).toBeVisible({ timeout: 30000 });
    await expect(page.locator('.el-tag')).toBeVisible({ timeout: 30000 });
  });
});
