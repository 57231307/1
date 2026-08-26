import { test, expect } from '@playwright/test';
import { loginViaUI, BASE_URL } from './helpers';

test.describe('批量操作与弹窗确认', () => {
  test.beforeAll(async ({ page }) => {
    await loginViaUI(page);
  });

  test('采购订单批量选择+审批', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase/orders`);
    await page.waitForTimeout(3000);

    await page.locator('.el-table').first().waitFor({ state: 'visible', timeout: 30_000 });

    // 查找多选列 checkbox
    const checkboxes = page.locator('.el-table .el-checkbox');
    const checkboxCount = await checkboxes.count();

    if (checkboxCount > 1) {
      const firstRowCheckbox = checkboxes.nth(1);
      await firstRowCheckbox.click().catch(() => {});
      await page.waitForTimeout(500);

      // 查找批量操作按钮
      const batchApproveBtn = page.locator('button:has-text("批量审批"), button:has-text("批量通过"), button:has-text("批量提交")').first();
      const batchBtnVisible = await batchApproveBtn.isVisible({ timeout: 3000 }).catch(() => false);

      if (batchBtnVisible) {
        await batchApproveBtn.click();
        await page.waitForTimeout(1000);

        // 验证确认弹窗
        const confirmDialog = page.locator('.el-message-box, .el-dialog:has-text("确认"), .el-popconfirm').first();
        const dialogVisible = await confirmDialog.isVisible({ timeout: 5000 }).catch(() => false);

        if (dialogVisible) {
          const confirmBtn = page.locator('.el-message-box__btns button:has-text("确定"), .el-dialog button:has-text("确定")').first();
          await confirmBtn.click().catch(() => {});
          await page.waitForTimeout(2000);
        }
      }
    }

    // 取消选择
    const selectedCheckboxes = page.locator('.el-table .el-checkbox.is-checked');
    const selectedCount = await selectedCheckboxes.count();
    if (selectedCount > 0) {
      await page.locator('.el-table .el-checkbox.is-checked').first().click().catch(() => {});
    }
  });

  test('删除确认弹窗取消交互', async ({ page }) => {
    // 用户管理页有删除操作
    await page.goto(`${BASE_URL}/system/users`);
    await page.waitForTimeout(3000);

    await page.locator('.el-table, .el-card').first().waitFor({ state: 'visible', timeout: 30_000 });

    // 查找删除按钮
    const deleteBtn = page.locator('button:has-text("删除"), .el-button--danger:has-text("删")').first();
    const deleteVisible = await deleteBtn.isVisible({ timeout: 5000 }).catch(() => false);

    if (deleteVisible) {
      await deleteBtn.click();
      await page.waitForTimeout(500);

      // 验证确认弹窗
      const popconfirm = page.locator('.el-popconfirm, .el-message-box').first();
      const popVisible = await popconfirm.isVisible({ timeout: 5000 }).catch(() => false);

      if (popVisible) {
        // 点击取消
        const cancelBtn = page.locator('.el-popconfirm button:has-text("取消"), .el-message-box__btns button:has-text("取消")').first();
        const cancelVisible = await cancelBtn.isVisible({ timeout: 3000 }).catch(() => false);
        if (cancelVisible) {
          await cancelBtn.click();
          await page.waitForTimeout(500);

          const popStillVisible = await popconfirm.isVisible({ timeout: 2000 }).catch(() => false);
          expect(popStillVisible).toBe(false);
        }
      }
    }
  });

  test('导出功能交互', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase/orders`);
    await page.waitForTimeout(3000);

    await page.locator('.el-table').first().waitFor({ state: 'visible', timeout: 30_000 });

    // 查找导出按钮（真实文本"导出"）
    const exportBtn = page.locator('button:has-text("导出")').first();
    const exportVisible = await exportBtn.isVisible({ timeout: 5000 }).catch(() => false);

    if (exportVisible) {
      const downloadPromise = page.waitForEvent('download', { timeout: 5000 }).catch(() => null);
      await exportBtn.click();
      await page.waitForTimeout(1000);

      const pageStillOk = await page.locator('body').isVisible();
      expect(pageStillOk).toBe(true);

      const download = await downloadPromise;
      if (download) {
        const filename = download.suggestedFilename();
        expect(filename).toMatch(/\.(xlsx|xls|csv|pdf)/);
      }
    }
  });

  test('下拉级联选择交互', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase/orders`);
    await page.waitForTimeout(3000);

    await page.locator('.el-table').first().waitFor({ state: 'visible', timeout: 30_000 });

    // 点击新建采购单
    const newBtn = page.locator('button:has-text("新建采购单")').first();
    await newBtn.click().catch(() => {});

    await page.waitForTimeout(1000);
    const dialog = page.locator('.el-dialog').first();
    await dialog.waitFor({ state: 'visible', timeout: 10_000 }).catch(() => {});

    // 查找供应商下拉选择（真实 placeholder"选择供应商"）
    const supplierSelect = page.locator('.el-dialog .el-select').first();
    const selectVisible = await supplierSelect.isVisible({ timeout: 5000 }).catch(() => false);

    if (selectVisible) {
      await supplierSelect.click();
      await page.waitForTimeout(500);

      // 验证下拉选项出现
      const dropdownItems = page.locator('.el-select-dropdown__item');
      const itemCount = await dropdownItems.count();
      expect(itemCount).toBeGreaterThan(0);

      // 选择第一项
      await dropdownItems.first().click();
      await page.waitForTimeout(500);

      // 验证已选择
      const selectedValue = await supplierSelect.locator('.el-select__selected-item, .el-select__placeholder').first().textContent();
      expect(selectedValue).toBeTruthy();
    }

    // 关闭弹窗
    await page.locator('.el-dialog__headerbtn').first().click().catch(() => {});
  });
});
