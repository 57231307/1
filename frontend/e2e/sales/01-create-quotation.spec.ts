// P9-3 销售 E2E 套件 — 01 创建报价单
// 创建时间: 2026-06-17
// 覆盖范围：销售报价单创建全流程（5 用例）

import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { pickSelect, pickSelectIn } from '../flow/ui-helpers';

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
    // 客户（form-item label='客户'，与 '客户等级' 共享子串，pickSelectIn 精确锚定）
    await pickSelectIn(page, page, '客户');
    // 报价单 items 验证要求至少 1 行，通过 QuotationItemEditor 添加产品
    await page.getByRole('button', { name: '添加产品' }).click();
    // 产品选择：QuotationItemEditor 内 el-select placeholder='选择产品'
    const itemsTable = page.locator('[aria-label="报价明细编辑表"]');
    await pickSelect(page, itemsTable.locator('.el-select').first());
    // 数量/单价（el-input-number → spinbutton 在明细行内）
    await itemsTable.getByRole('spinbutton').first().fill('100');
    await itemsTable.getByRole('spinbutton').nth(1).fill('50');
    // 提交（quotations.create.saveDraft = '保存草稿'）
    await page.getByRole('button', { name: '保存草稿' }).click();
    // 真实成功提示 quotations.create.draftSaved = '草稿保存成功'
    await expect(page.getByText('草稿保存成功')).toBeVisible({ timeout: 30000 });
  });

  test('01-04 报价单草稿可保存后再次编辑', async ({ page }) => {
    // 去共享对象耦合：本例自建一张专属草稿、用其自身 id 进详情编辑，杜绝依赖列表行序。
    // 旧实现 goto('/quotations') + getByRole('row').nth(1) 抓的是同片内 01-03、
    // ensureTestEntities 步骤10 刚建的共享草稿行；该草稿被同片其它用例（如 quotations/02-04
    // 的取消、flow 链的提交/转单）流转掉状态后，详情页 canEdit=['draft','rejected']（detail.vue）
    // 判假 → 「编辑」按钮不渲染 → 点编辑 30s 超时（#4663 A6 真实红签名）。
    // 改为经 UI 新建一张专属草稿：create.vue:479 保存成功后 router.push(`/quotations/${res.data.id}`)
    // 落到本例专属详情页，从 URL 捕获 id——编辑目标恒为本例这张、状态必为 draft。
    await page.goto('/quotations/new');
    // 客户 + 产品 + 数量/单价：与 01-03 一致的必填项，保证草稿能成功保存
    await pickSelectIn(page, page, '客户');
    await page.getByRole('button', { name: '添加产品' }).click();
    const itemsTable = page.locator('[aria-label="报价明细编辑表"]');
    await pickSelect(page, itemsTable.locator('.el-select').first());
    await itemsTable.getByRole('spinbutton').first().fill('100');
    await itemsTable.getByRole('spinbutton').nth(1).fill('50');
    // 保存草稿 → 专属草稿落库，真实提示 quotations.create.draftSaved = '草稿保存成功'
    await page.getByRole('button', { name: '保存草稿' }).click();
    await expect(page.getByText('草稿保存成功')).toBeVisible({ timeout: 30000 });
    // 保存后前端跳本例专属详情页（router.push `/quotations/{id}`），URL 尾部即专属 id
    await expect(page).toHaveURL(/\/quotations\/\d+$/, { timeout: 30000 });
    const id = Number(new URL(page.url()).pathname.split('/').pop());
    expect(
      Number.isInteger(id) && id > 0,
      `未能从详情页 URL 捕获本例专属草稿 id：${page.url()}`
    ).toBeTruthy();
    // 本例专属草稿状态为 draft → 详情页渲染「编辑」（quotations.detail.edit = '编辑'）
    const editBtn = page.getByRole('button', { name: '编辑' });
    await expect(editBtn, `本例专属草稿 ${id} 详情页应渲染「编辑」按钮`).toBeVisible({
      timeout: 30000,
    });
    await editBtn.click();
    await expect(page).toHaveURL(new RegExp(`/quotations/${id}/edit`));
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
    // 客户 el-select 为 filterable，EP 2.14 下选中值渲染在 .el-select__selected-item.el-select__placeholder，
    // 且无值时该节点带 is-transparent（显示占位文案）。首个 .el-select__selected-item 是只读输入框容器
    // （.el-select__input-wrapper），textContent 恒为 ""（见 EP select2.mjs 渲染顺序 / flow/17-batch 取证）。
    // 因此取"可见选中项"（placeholder 且非 transparent）这一唯一能证明"确有值选中"的节点，
    // 未预填时该节点带 is-transparent → :not(.is-transparent) 命中 0 → 断言自然超时失败（真实红），不放宽。
    const customerSelected = page
      .locator('.el-form-item')
      .filter({ has: page.locator('.el-form-item__label', { hasText: /^\s*\*?\s*客户\s*$/ }) })
      .first()
      .locator('.el-select__selected-item.el-select__placeholder:not(.is-transparent)');
    await expect(customerSelected.first()).toBeVisible({ timeout: 15_000 });
    await expect(customerSelected.first()).not.toHaveText('', { timeout: 15_000 });
  });
});
