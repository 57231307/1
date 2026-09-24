// 报价单 E2E 套件 — 01 报价单创建
// 创建时间: 2026-08-19
// 覆盖范围：报价单创建（含明细行、币种、价格条款）→ 提交审批
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('01 报价单创建', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入报价单列表页', async ({ page }) => {
    await page.goto('/quotations');
    // 列表页真实标题 quotations.list.title = '报价单管理'，breadcrumb 同文本需 .first()
    await expect(page.getByText('报价单管理').first()).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('button', { name: /新建报价单/ })).toBeVisible();
  });

  test('01-02 新建报价单', async ({ page }) => {
    await page.goto('/quotations/new');
    await expect(page.locator('form')).toBeVisible({ timeout: 30000 });
    // labelCustomer='客户' 与 labelCustomerLevel='客户等级' 共享子串，exact 避免 strict 多命中
    await page.getByLabel('客户', { exact: true }).click();
    await page.getByRole('option').first().click();
    await page.getByLabel('报价日期').fill('2026-08-19');
    await page.keyboard.press('Enter');
    await page.getByLabel('有效期至').fill('2026-09-19');
    await page.keyboard.press('Enter');
    await page.getByLabel('价格条款').click();
    await page.getByRole('option').first().click();
    await page.getByLabel('币种').click();
    await page.getByRole('option').first().click();
    await page.getByLabel('汇率').fill('1');
    // 报价单 items 验证规则要求至少 1 行明细
    await page.getByRole('button', { name: '添加产品' }).click();
    // 明细表 aria-label 在 el-table 包装 div 上，用属性选择器定位
    const itemsTable = page.locator('[aria-label="报价明细编辑表"]');
    await itemsTable.getByPlaceholder('选择产品').first().click();
    await page.getByRole('option').first().click();
    await itemsTable.getByRole('spinbutton').first().fill('100');
    await itemsTable.getByRole('spinbutton').nth(1).fill('50');
    await page.getByLabel('备注').fill('E2E 测试报价单');
    await page.getByRole('button', { name: '保存草稿' }).click();
    // 真实成功提示 quotations.create.draftSaved = '草稿保存成功'
    await expect(page.getByText('草稿保存成功')).toBeVisible({ timeout: 30000 });
  });

  test('01-03 报价单列表可正常加载', async ({ page }) => {
    await page.goto('/quotations');
    await expect(page.locator('table, .el-table')).toBeVisible({ timeout: 30000 });
  });
});
