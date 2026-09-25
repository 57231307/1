// 生产计划 E2E 套件 — 03 工单管理（查看、删除、按状态筛选）
// 覆盖范围：查看详情、删除草稿、按状态筛选
import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, ensureTestEntities, getCtx } from '../flow/helpers';

async function filterByStatus(page: Page, statusLabel: string): Promise<void> {
  await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
  await page.getByLabel(/状态/).click();
  await page.getByRole('option', { name: statusLabel, exact: true }).click();
  await page.getByRole('button', { name: /查询/ }).click();
}

async function createDraftOrder(page: Page): Promise<number> {
  const ctx = getCtx();
  expect(ctx.productIds.length, '前置：需要至少一个产品').toBeGreaterThanOrEqual(1);
  const created = await apiCall<{ id?: number }>(
    page,
    'POST',
    '/production/production-orders/orders',
    {
      product_id: ctx.productIds[0],
      planned_quantity: 100,
      priority: 5,
    }
  );
  const id = created.data?.id;
  if (!id) {
    throw new Error(`生产工单造数失败：${JSON.stringify(created).slice(0, 200)}`);
  }
  return id;
}

test.describe('生产计划 - 03 工单管理', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('工单详情可查看', async ({ page }) => {
    // 原实现 `if (await viewBtn.isVisible())` + getByRole('link')：查看按钮为
    // el-button(link) → 角色是 button，link 定位恒不命中 → 零断言假绿。
    // 改为先造一张工单，再硬断言"查看"按钮存在并打开详情。
    await ensureTestEntities(page);
    await createDraftOrder(page);

    await page.goto('/production');
    const viewBtn = page.getByRole('button', { name: '查看', exact: true }).first();
    await expect(viewBtn, '工单列表应渲染"查看"按钮').toBeVisible({ timeout: 30000 });
    await viewBtn.click();
    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible({ timeout: 10000 });
    // el-descriptions 详情面板里「订单编号/产品名称/计划数量」三个标签 + 其值均可命中正则 →
    // getByText strict 多命中（>=1 元素即可证详情面板已渲染这些字段，.first() 收敛多命中，
    // 非放宽为恒真：若详情未渲染这些标签则 .first() 不可见、断言失败）。
    await expect(dialog.getByText(/订单编号|产品名称|计划数量/).first()).toBeVisible();
    await dialog.getByRole('button', { name: /关闭/ }).click();
  });

  test('草稿工单可删除', async ({ page }) => {
    await ensureTestEntities(page);
    await createDraftOrder(page);

    await page.goto('/production');
    await filterByStatus(page, '草稿');

    const deleteBtn = page.getByRole('button', { name: '删除', exact: true }).first();
    await expect(deleteBtn, '草稿工单应渲染"删除"按钮').toBeVisible({ timeout: 30000 });
    await deleteBtn.click();
    await page.getByRole('button', { name: /确定/ }).click();
    await expect(page.getByText(/删除成功/)).toBeVisible({ timeout: 30000 });
  });

  test('按状态筛选工单', async ({ page }) => {
    await ensureTestEntities(page);
    await createDraftOrder(page);
    await page.goto('/production');
    // 操作列按钮均为 el-button（角色 button），且查询按钮文案是"查询"（原写"搜索"点不到）
    await filterByStatus(page, '草稿');
    await expect(page.getByRole('button', { name: '编辑', exact: true }).first()).toBeVisible({
      timeout: 30000,
    });
  });
});
