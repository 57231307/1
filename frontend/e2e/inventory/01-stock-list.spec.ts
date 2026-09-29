// 库存管理 E2E 套件 — 01 库存台账
// 覆盖范围：库存台账列表加载、筛选、统计卡片
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { pickSelectIn } from '../flow/ui-helpers';

test.describe('库存管理 - 01 库存台账', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('库存管理页面可访问', async ({ page }) => {
    await page.goto('/inventory');
    await expect(page.getByRole('heading', { name: '库存管理' })).toBeVisible({ timeout: 30000 });
  });

  test('库存台账 Tab 数据加载', async ({ page }) => {
    await page.goto('/inventory');
    await expect(page.getByRole('tab', { name: /库存台账/ })).toBeVisible();
    await page.getByRole('tab', { name: /库存台账/ }).click();
    await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
  });

  test('库存筛选功能', async ({ page }) => {
    await page.goto('/inventory');
    // 「仓库」「状态」均为筛选栏 el-select；隐藏打印对话框内有同名「仓库」label，
    // getByLabel(/仓库/) 会多命中且点到只读 combobox 内层 input（被 placeholder 拦 → 超时）。
    // 改用冻结 helper：以筛选表单容器（aria-label=库存台账筛选表单）为 root + 精确 label 锚定，
    // 点外层 .el-select__wrapper 打开下拉，仅验证下拉可打开（openOnly）。
    const stockFilter = page.getByLabel('库存台账筛选表单');
    await pickSelectIn(stockFilter, page, '仓库', { openOnly: true });
    await expect(page.getByRole('option').first()).toBeVisible({ timeout: 30000 });
    await page.keyboard.press('Escape');
    await pickSelectIn(stockFilter, page, '状态', { openOnly: true });
    await expect(page.getByRole('option').first()).toBeVisible({ timeout: 30000 });
    await page.keyboard.press('Escape');
    await stockFilter.getByRole('button', { name: '查询' }).click();
  });

  test('库存预警 Tab', async ({ page }) => {
    await page.goto('/inventory');
    await page.getByRole('tab', { name: /库存预警/ }).click();
    await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
    // 预警表列头真实文案为「预警级别」（i18n inventory.alertTab.colAlertLevel）；
    // 原正则 /预警等级|紧急|预警/ 字面量有误（列头实际为"预警级别"非"等级"）。
    // el-table 列头 th 自带 columnheader 角色。
    await expect(page.getByRole('columnheader', { name: '预警级别' })).toBeVisible({
      timeout: 30000,
    });
  });
});
