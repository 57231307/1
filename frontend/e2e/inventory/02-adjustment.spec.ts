// 库存管理 E2E 套件 — 02 库存调整（盘盈/盘亏）
// 覆盖范围：库存调整对话框（increase/decrease）、表单填写、提交
import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { ensureTestEntities, apiCallRaw, seedFourDimStockIn } from '../flow/helpers';
import { fillFieldByLabel, formItemByExactLabel, pickSelectIn } from '../flow/ui-helpers';

test.describe('库存管理 - 02 库存调整', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  // 调整类型 radio 的可点区是外层 <label class="el-radio">：EP 把真 <input type=radio>
  // 隐藏（.el-radio__original），getByRole('radio') 命中的是该隐藏 input，被 .el-radio__inner
  // 拦 pointer events → click 超时（既往假红根因）。改点其可见文本（label 内 span），
  // 事件冒泡到 label 触发选中。
  const pickAdjustType = async (dialog: import('@playwright/test').Locator, label: string) => {
    await dialog.getByText(label, { exact: true }).click();
  };

  // 后端库存调整以「既有库存行 stock_id」为调整对象（inventory_adjustment_service
  // ::create_adjustment_items 按 stock_id 读取在库量作为调整前量），工具栏入口只选「产品+仓库」，
  // 提交前须保证该组合确有库存行，否则调整对象不存在、后端拒绝。
  // 这里以对话框两个下拉实际渲染的口径（GET /warehouses、/products，page_size=1000）读取首项
  // ——与对话框 fetchWarehouses/fetchProducts 完全相同的请求，index 0 命中同一行——再为其造
  // 一行带全四维的库存，确保「选中的首行」确有库存可调整。属补齐真实前置，不弱化成功断言。
  const seedAdjustmentTarget = async (page: Page): Promise<void> => {
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
    const warehouseId = wh.items?.[0]?.id;
    const productId = pr.items?.[0]?.id;
    expect(warehouseId, '前置：库存调整需至少一个仓库（对话框首项）').toBeTruthy();
    expect(productId, '前置：库存调整需至少一个产品（对话框首项）').toBeTruthy();
    const tag = Date.now().toString().slice(-6);
    await seedFourDimStockIn(page, {
      productId: productId!,
      warehouseId: warehouseId!,
      colorNo: `E2E-ADJ-C${tag}`,
      dyeLotNo: `E2E-ADJ-D${tag}`,
      batchNo: `E2E-ADJ-B${tag}`,
      quantityMeters: '5000',
    });
  };

  test('库存调整 - 盘盈', async ({ page }) => {
    await seedAdjustmentTarget(page);
    await page.goto('/inventory');
    await page.getByRole('button', { name: '库存调整' }).click();
    const dialog = page.locator('.el-dialog:visible').last();
    await expect(dialog).toBeVisible({ timeout: 30000 });
    // 缺陷A 修复后：工具栏入口渲染可选 el-select（仓库/产品）
    await expect(formItemByExactLabel(dialog, '仓库')).toBeVisible({ timeout: 10000 });
    await pickSelectIn(dialog, page, '仓库', { index: 0 });
    await pickSelectIn(dialog, page, '产品', { index: 0 });
    // 调整类型是 radio（AdjustmentDialog.vue），标签为「增加」/「减少」（i18n typeIncrease/typeDecrease）
    await expect(dialog.getByText('增加', { exact: true })).toBeVisible();
    await pickAdjustType(dialog, '增加');
    // el-input-number fill 后按 Tab 失焦提交 v-model，否则 modelValue 不更新 → 提交被本地校验拦下
    await fillFieldByLabel(dialog, page, '调整数量', '50');
    await page.keyboard.press('Tab');
    // 调整原因为 el-input type=textarea（真 <textarea>，非 <input>），fillFieldByLabel 只命中
    // input，故按精确 label 锚定其 textarea 元素填写。
    await formItemByExactLabel(dialog, '调整原因')
      .locator('textarea')
      .first()
      .fill('E2E 测试盘盈调整');
    await dialog.getByRole('button', { name: '确定' }).click();
    await expect(page.getByText('库存调整成功')).toBeVisible({ timeout: 30000 });
  });

  test('库存调整 - 盘亏', async ({ page }) => {
    await seedAdjustmentTarget(page);
    await page.goto('/inventory');
    await page.getByRole('button', { name: '库存调整' }).click();
    const dialog = page.locator('.el-dialog:visible').last();
    await expect(dialog).toBeVisible({ timeout: 30000 });
    // 缺陷A 修复后：工具栏入口渲染可选 el-select（仓库/产品）
    await expect(formItemByExactLabel(dialog, '仓库')).toBeVisible({ timeout: 10000 });
    await pickSelectIn(dialog, page, '仓库', { index: 0 });
    await pickSelectIn(dialog, page, '产品', { index: 0 });
    await expect(dialog.getByText('减少', { exact: true })).toBeVisible();
    await pickAdjustType(dialog, '减少');
    await fillFieldByLabel(dialog, page, '调整数量', '30');
    await page.keyboard.press('Tab');
    // 调整原因同为 el-input type=textarea
    await formItemByExactLabel(dialog, '调整原因')
      .locator('textarea')
      .first()
      .fill('E2E 测试盘亏调整');
    await dialog.getByRole('button', { name: '确定' }).click();
    await expect(page.getByText('库存调整成功')).toBeVisible({ timeout: 30000 });
  });
});
