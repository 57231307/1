// 库存管理 E2E 套件 — 03 库存调拨（创建 → 审批）
// 覆盖范围：调拨单创建（调出→调入仓库、明细行）、调拨审批
import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  apiCall,
  apiCallRaw,
  ensureTestEntities,
  getCtx,
  seedFourDimStockIn,
} from '../flow/helpers';
import { pickSelectIn, formItemByExactLabel, fillFieldByLabel } from '../flow/ui-helpers';

// 调拨出库要求「调出仓库对该产品有足量库存」（inventory_move::check_from_warehouse_inventory）
// 且每条明细批次非空（fabric_class::validate_fabric_trace）。工具栏入口按对话框渲染口径
// （GET /warehouses、/products page_size=1000）取首行作为调出仓库/产品——与对话框两个下拉的
// index 0 命中同一行——为其造一行足量四维库存，使「选中的调出仓库/产品」确有货可调。
async function seedTransferSource(page: Page): Promise<void> {
  await ensureTestEntities(page);
  const wh = await apiCallRaw<{ items: { id: number }[] }>(
    page,
    'GET',
    '/warehouses?page=1&page_size=1000'
  );
  const pr = await apiCallRaw<{ items: { id: number }[] }>(
    page,
    'GET',
    '/products?page=1&page_size=1000'
  );
  const fromWarehouseId = wh.items?.[0]?.id;
  const productId = pr.items?.[0]?.id;
  expect(fromWarehouseId, '前置：调拨需至少一个调出仓库（对话框首项）').toBeTruthy();
  expect(productId, '前置：调拨需至少一个产品（对话框首项）').toBeTruthy();
  const tag = Date.now().toString().slice(-6);
  await seedFourDimStockIn(page, {
    productId: productId!,
    warehouseId: fromWarehouseId!,
    colorNo: `E2E-TRF-C${tag}`,
    dyeLotNo: `E2E-TRF-D${tag}`,
    batchNo: `E2E-TRF-B${tag}`,
    quantityMeters: '5000',
  });
}

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
    // 缺陷B 修复后：调拨对话框明细行有产品 el-select，绑定 item.product_id。
    // 提交前校验至少一行选了产品，否则弹出真实提示。
    // 真实后端另要求：调出仓库对该产品有货、明细批次非空（四维追溯）。见 seedTransferSource。
    await seedTransferSource(page);
    await page.goto('/inventory');
    await page.getByRole('button', { name: /调拨/ }).click();
    const dialog = page.locator('.el-dialog:visible').last();
    await expect(dialog).toBeVisible({ timeout: 30000 });
    // 选择调出/调入仓库
    await expect(formItemByExactLabel(dialog, '调出仓库')).toBeVisible({ timeout: 10000 });
    await pickSelectIn(dialog, page, '调出仓库', { index: 0 });
    await pickSelectIn(dialog, page, '调入仓库', { index: 1 });
    // 选择第一行产品（修复后新增的 el-select，form-item label="产品"）
    await pickSelectIn(dialog, page, '产品', { index: 0 });
    // 填写数量：el-input-number 内层 input，定位到同一 form-item 下的数字输入框
    const qtyInput = formItemByExactLabel(dialog, '产品').locator('.el-input-number input').first();
    await qtyInput.waitFor({ state: 'visible', timeout: 10000 });
    await qtyInput.click({ clickCount: 3 });
    await qtyInput.fill('10');
    await page.keyboard.press('Tab');
    // 批次为后端出入库四维必填（白坯布色号留空即免缸号）：缺陷B 补齐的产品选择器之外必须录入批次
    await fillFieldByLabel(dialog, page, '批次号', `E2E-TRF-${Date.now().toString().slice(-6)}`);
    // 提交按钮真实文案「确定」（inventory.transferDialog.confirm）
    await dialog.getByRole('button', { name: '确定' }).click();
    await expect(page.getByText('调拨单创建成功')).toBeVisible({ timeout: 30000 });
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
