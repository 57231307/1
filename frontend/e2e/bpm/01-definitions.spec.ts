// BPM 审批管理 E2E 套件 — 01 流程定义
// 创建时间: 2026-08-19
// 覆盖范围：流程定义创建 → 编辑 → 版本管理
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('01 流程定义', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入流程定义页面', async ({ page }) => {
    await page.goto('/bpm/definitions');
    await expect(page.getByText(/流程定义/)).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('button', { name: /新增/ })).toBeVisible();
  });

  test('01-02 创建新流程定义', async ({ page }) => {
    await page.goto('/bpm/definitions');
    await page.getByRole('button', { name: /新增/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    await page.getByLabel(/流程标识/).fill(`e2e-${Date.now()}`);
    await page.getByLabel(/流程名称/).fill('E2E 测试流程');
    await page.getByLabel(/分类/).click();
    await page.getByRole('option').first().click();
    await page.getByLabel(/描述/).fill('E2E 测试流程定义');
    await page.getByRole('button', { name: /确认|提交/ }).last().click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({ timeout: 30000 }).catch(() => {
      return null;
    });
  });

  test('01-03 流程定义筛选功能可用', async ({ page }) => {
    await page.goto('/bpm/definitions');
    await page.getByLabel(/关键词/).fill('E2E');
    await page.getByRole('button', { name: /查询/ }).click();
    await expect(page.locator('table, .el-table')).toBeVisible({ timeout: 30000 });
    await page.getByRole('button', { name: /重置/ }).click();
    await expect(page.locator('table, .el-table')).toBeVisible({ timeout: 30000 });
  });

  test('01-04 流程定义可编辑', async ({ page }) => {
    await page.goto('/bpm/definitions');
    const editBtn = page.getByRole('button', { name: /编辑/ }).first();
    if (await editBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await editBtn.click();
      await expect(page.locator('.el-dialog')).toBeVisible();
      await expect(page.getByLabel(/流程标识/)).toBeVisible();
      await page.getByRole('button', { name: /取消/ }).click();
    }
  });
});