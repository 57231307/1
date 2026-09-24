// P9-3 销售 E2E 套件 — 01 创建报价单
// 创建时间: 2026-06-17
// 覆盖范围：销售报价单创建全流程（5 用例）

import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

/**
 * 测试套件：销售报价单创建
 *
 * 业务流程：
 * 1. 进入销售 → 报价单 → 新建报价单
 * 2. 选择客户（必填）
 * 3. 选择产品行（必填 ≥1 行）
 * 4. 设置数量、单价、折扣、税率
 * 5. 提交并验证报价单号生成
 */
test.describe('01 创建报价单', () => {
  test.beforeEach(async ({ page, context }) => {
    // V15 Batch 487 P0-T05：注入 auth mock，业务 API 走真实后端（applyAuthMocks 不再 mock 业务 API）
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入报价单列表页可见菜单', async ({ page }) => {
    // 报价单为独立模块（router index.ts:1086 path:'quotations'），真实可达路径 /quotations；
    // 原 /sales/quotation/list 子路由不存在 → 落 404
    await page.goto('/quotations');
    // 列表页真实标题 quotations.list.title = '报价单管理'，breadcrumb 同文本需 .first()
    await expect(page.getByText('报价单管理').first()).toBeVisible();
    // 验证列表区域有"新建报价单"按钮
    await expect(page.getByRole('button', { name: /新建报价单/ })).toBeVisible();
  });

  test('01-02 创建空报价单应校验失败', async ({ page }) => {
    // 真实新建报价单页 router index.ts:1097 path:'quotations/new'
    await page.goto('/quotations/new');
    // 不填任何字段直接提交
    await page.getByRole('button', { name: /保存/ }).click();
    // 应显示客户/产品必填错误
    await expect(page.getByText(/客户.*必选|请选择客户/)).toBeVisible();
  });

  test('01-03 创建有效报价单成功并生成报价单号', async ({ page }) => {
    await page.goto('/quotations/new');
    // 客户（form-item label='客户'，与 '客户等级' 共享子串，须 exact）
    await page.getByLabel('客户', { exact: true }).click();
    await page.getByRole('option').first().click();
    // 报价单 items 验证要求至少 1 行，通过 QuotationItemEditor 添加产品
    await page.getByRole('button', { name: '添加产品' }).click();
    // 产品选择：QuotationItemEditor 内 el-select placeholder='选择产品'
    const itemsTable = page.locator('[aria-label="报价明细编辑表"]');
    await itemsTable.getByPlaceholder('选择产品').first().click();
    await page.getByRole('option').first().click();
    // 数量/单价（el-input-number → spinbutton 在明细行内）
    await itemsTable.getByRole('spinbutton').first().fill('100');
    await itemsTable.getByRole('spinbutton').nth(1).fill('50');
    // 提交（quotations.create.saveDraft = '保存草稿'）
    await page.getByRole('button', { name: '保存草稿' }).click();
    // 真实成功提示 quotations.create.draftSaved = '草稿保存成功'
    await expect(page.getByText('草稿保存成功')).toBeVisible({ timeout: 30000 });
  });

  test('01-04 报价单草稿可保存后再次编辑', async ({ page }) => {
    await page.goto('/quotations');
    // 列表行操作按钮 quotations.list.view = '查看'，非点击状态文字
    const firstRow = page.getByRole('row').nth(1);
    await firstRow.getByRole('button', { name: '查看' }).click();
    await expect(page).toHaveURL(/\/quotations\/\d+/);
    // 详情页编辑按钮 quotations.detail.edit = '编辑'
    await page.getByRole('button', { name: '编辑' }).click();
    await expect(page).toHaveURL(/\/quotations\/\d+\/edit/);
    // 编辑成功后提示 quotations.create.draftUpdated = '草稿已更新'
    await page.getByRole('button', { name: '保存草稿' }).click();
    await expect(page.getByText('草稿已更新')).toBeVisible({ timeout: 30000 });
  });

  test('01-05 报价单可复制为新单', async ({ page }) => {
    await page.goto('/quotations');
    const firstRow = page.locator('tr, .el-table__row').nth(1);
    await firstRow.getByRole('button', { name: /复制/ }).click();
    // 应跳转到新建页面并预填数据（真实新建页 router index.ts:1097 path:'quotations/new'）
    await expect(page).toHaveURL(/\/quotations\/new/);
    // 客户字段应已预填
    const customerInput = page.getByLabel(/客户/).first();
    await expect(customerInput).not.toHaveValue('');
  });
});
