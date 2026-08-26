import { test, expect } from '@playwright/test';
import { loginViaUI, BASE_URL } from './helpers';

test.describe('表单校验真实 UI 交互', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('采购订单表单：必填字段校验', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase/orders`);
    await page.waitForTimeout(3000);

    // 等待表格加载
    await page.locator('.el-table').first().waitFor({ state: 'visible', timeout: 30_000 });

    // 点击新建按钮（真实文本"新建采购单"）
    const newBtn = page.locator('button:has-text("新建采购单")').first();
    await newBtn.click().catch(() => {});

    await page.waitForTimeout(1000);

    // 等待弹窗（el-dialog）出现
    const dialog = page.locator('.el-dialog').first();
    await dialog.waitFor({ state: 'visible', timeout: 10_000 }).catch(() => {});

    // 直接点保存（不填任何字段），应触发必填校验
    const saveBtn = page.locator('.el-dialog button:has-text("保存"), .el-dialog button:has-text("确定"), .el-dialog button:has-text("提交")').first();
    await saveBtn.click().catch(() => {});

    await page.waitForTimeout(1000);

    // 验证有校验错误提示
    const hasError = await page.locator('.el-form-item__error, .el-message--error')
      .first()
      .isVisible({ timeout: 5000 })
      .catch(() => false);
    expect(hasError).toBe(true);

    // 关闭弹窗
    await page.locator('.el-dialog__headerbtn').first().click().catch(() => {});
  });

  test('销售订单表单：数量为负数校验', async ({ page }) => {
    await page.goto(`${BASE_URL}/sales/orders`);
    await page.waitForTimeout(3000);

    await page.locator('.el-table').first().waitFor({ state: 'visible', timeout: 30_000 });

    // 点击新建按钮（真实文本"新建订单"）
    const newBtn = page.locator('button:has-text("新建订单")').first();
    await newBtn.click().catch(() => {});

    await page.waitForTimeout(1000);
    const dialog = page.locator('.el-dialog').first();
    await dialog.waitFor({ state: 'visible', timeout: 10_000 }).catch(() => {});

    // 在数量输入框中输入负数
    const qtyInput = page.locator('.el-dialog .el-input-number input, .el-dialog input[placeholder*="数量"]').first();
    await qtyInput.fill('-5').catch(() => {});

    // 触发表单校验
    await page.locator('.el-dialog button:has-text("保存"), .el-dialog button:has-text("确定")').first().click().catch(() => {});

    await page.waitForTimeout(1000);

    // 验证校验提示
    const hasError = await page.locator('.el-form-item__error, .el-message--error')
      .first()
      .isVisible({ timeout: 5000 })
      .catch(() => false);
    expect(hasError).toBe(true);

    await page.locator('.el-dialog__headerbtn').first().click().catch(() => {});
  });

  test('凭证表单：借贷不平衡校验', async ({ page }) => {
    await page.goto(`${BASE_URL}/finance/voucher`);
    await page.waitForTimeout(3000);

    await page.locator('.el-table, .el-card, .el-form').first().waitFor({ state: 'visible', timeout: 30_000 });

    // 点击新建按钮（真实文本"新增凭证"）
    const newBtn = page.locator('button:has-text("新增凭证")').first();
    await newBtn.click().catch(() => {});

    await page.waitForTimeout(1000);
    const dialog = page.locator('.el-dialog').first();
    await dialog.waitFor({ state: 'visible', timeout: 10_000 }).catch(() => {});

    // 输入借方金额（不输入贷方，制造借贷不平衡）
    const debitInput = page.locator('.el-dialog .el-input-number input').first();
    await debitInput.fill('1000').catch(() => {});

    // 提交
    await page.locator('.el-dialog button:has-text("保存"), .el-dialog button:has-text("确定")').first().click().catch(() => {});

    await page.waitForTimeout(1000);

    // 验证校验提示
    const hasError = await page.locator('.el-form-item__error, .el-message--error')
      .first()
      .isVisible({ timeout: 5000 })
      .catch(() => false);
    expect(hasError).toBe(true);

    await page.locator('.el-dialog__headerbtn').first().click().catch(() => {});
  });
});
