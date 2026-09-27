// 库存管理 E2E 套件 — 02 库存调整（盘盈/盘亏）
// 覆盖范围：库存调整对话框（increase/decrease）、表单填写、提交
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
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

  test('库存调整 - 盘盈', async ({ page }) => {
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
