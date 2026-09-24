// 库存管理 E2E 套件 — 03 库存调拨（创建 → 审批）
// 覆盖范围：调拨单创建（调出→调入仓库、明细行）、调拨审批
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, ensureTestEntities, getCtx } from '../flow/helpers';

test.describe('库存管理 - 03 库存调拨', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('库存调拨 Tab 数据加载', async ({ page }) => {
    await page.goto('/inventory');
    await page.getByRole('tab', { name: /库存调拨/ }).click();
    await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
  });

  test('创建库存调拨单', async ({ page }) => {
    await page.goto('/inventory');
    await page.getByRole('button', { name: /调拨/ }).click();
    const dialog = page.locator('.el-dialog:visible').last();
    await expect(dialog).toBeVisible({ timeout: 30000 });
    await expect(dialog.getByText('调出仓库')).toBeVisible({ timeout: 10000 });
    await dialog.getByLabel(/调出仓库/).click();
    await page.getByRole('option').first().click();
    await dialog.getByLabel(/调入仓库/).click();
    await page.getByRole('option').nth(1).click();
    await dialog.getByRole('button', { name: /添加产品|添加|新增/ }).click();
    await expect(dialog.getByText(/数量/)).toBeVisible();
    await dialog.getByRole('button', { name: /确认/ }).click();
    await expect(page.getByText(/成功|已提交|已创建/)).toBeVisible({ timeout: 30000 });
  });

  test('审批待审批调拨单', async ({ page }) => {
    // 真实造数：先补前置实体（≥2 仓库 + 产品），再用 API 落一张"待审批"调拨单。
    // 原实现用 `if (await approveBtn.isVisible())` 把整段交互包进可见性分支：
    // 空库时列表无 pending 单 → 审批按钮不渲染 → 一条断言都不执行却记为通过（假绿）。
    await ensureTestEntities(page);
    const ctx = getCtx();
    expect(ctx.warehouseIds.length, '前置：需要至少两个仓库（调出/调入）').toBeGreaterThanOrEqual(
      2
    );
    expect(ctx.productIds.length, '前置：需要至少一个产品').toBeGreaterThanOrEqual(1);

    // 白坯布口径（色号为空、免缸号、批次必填）满足后端建单校验，落库初始态 PENDING
    const batchNo = `E2E-AP-${Date.now().toString().slice(-6)}`;
    const created = await apiCall<{ id?: number }>(page, 'POST', '/inventory/transfers', {
      from_warehouse_id: ctx.warehouseIds[0],
      to_warehouse_id: ctx.warehouseIds[1],
      transfer_date: new Date().toISOString(),
      notes: 'E2E 审批用例造数',
      items: [{ product_id: ctx.productIds[0], quantity: '5', color_no: '', batch_no: batchNo }],
    });
    expect(
      created.data?.id,
      `调拨建单应返回 data.id，实际响应：${JSON.stringify(created).slice(0, 200)}`
    ).toBeTruthy();

    // 切到库存调拨 Tab，让刚造的 pending 单进入列表
    await page.goto('/inventory');
    await page.getByRole('tab', { name: /库存调拨/ }).click();

    // 硬断言（Tier A）：待审批单必然渲染"审批"按钮，缺失即缺陷
    const approveBtn = page.getByRole('button', { name: '审批', exact: true }).first();
    await expect(approveBtn, '待审批调拨单应渲染"审批"按钮').toBeVisible({ timeout: 30000 });
    await approveBtn.click();
    await page.getByRole('button', { name: /确定/ }).click();
    await expect(page.getByText(/审批成功/)).toBeVisible({ timeout: 30000 });
  });
});
