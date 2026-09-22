// 报价单 E2E 套件 — 03 报价单详情与编辑
// 创建时间: 2026-08-19（本轮去假绿改造 2026-09-23）
// 覆盖范围：报价单详情查看 → 草稿报价单进入编辑页
//
// 去假绿说明：原 2 个用例在报价单「列表页」用 `if (await btn.isVisible())` 判断查看/编辑按钮，
// 且断言的文案与产品实际渲染不一致（详情页无「基本信息」标题；编辑页标签是「客户」而非可被
// `getByLabel(/客户/)` 命中的形式）。真实数据下这些条件可能不成立即 0 断言 → 假绿。
// 现改为：真实造一张草稿报价单 → 进详情页硬断言渲染内容（标题/报价单号/状态标签）→
// 从详情页点「编辑」→ 硬断言进入编辑页且表单渲染。
import { test, expect, type Page } from '@playwright/test';
import {
  loginViaUI,
  ensureTestEntities,
  getCtx,
  apiCall,
  apiCallRaw,
  BASE_URL,
} from '../flow/helpers';

/** 创建一张最小草稿报价单，返回 id 与后端报价单号 */
async function createDraftQuotation(page: Page): Promise<{ id: number; quotationNo: string }> {
  const ctx = getCtx();
  const res = await apiCall<{ id?: number }>(page, 'POST', '/quotations', {
    customer_id: ctx.customerId,
    sales_user_id: ctx.userIds[0],
    quotation_date: new Date().toISOString().slice(0, 10),
    valid_until: new Date(Date.now() + 30 * 86400000).toISOString().slice(0, 10),
    currency: 'CNY',
    exchange_rate: '1',
    base_currency: 'CNY',
    price_terms: 'FOB',
    tax_inclusive: true,
    tax_rate: '0',
    items: [
      {
        product_id: ctx.productIds[0],
        unit: '米',
        quantity: '5',
        unit_price: '100',
        unit_price_with_tax: '100',
      },
    ],
    notes: 'E2E 报价单详情用例',
  });
  const id = res.data?.id;
  expect(id, '前置失败：报价单创建未返回 id').toBeTruthy();
  const created = await apiCallRaw<{ status: string; quotation_no: string }>(
    page,
    'GET',
    `/quotations/${id}`
  );
  expect(created.status, '新建报价单应为 draft').toBe('draft');
  return { id: id as number, quotationNo: created.quotation_no };
}

test.describe('03 报价单详情与编辑', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('03-01 报价单详情可查看', async ({ page }) => {
    const { id, quotationNo } = await createDraftQuotation(page);
    await page.goto(`${BASE_URL}/quotations/${id}`);
    // detail.vue 头部标题为「报价单详情 - <quotation_no>」，quotation_no 独立 span 渲染
    await expect(page.getByText('报价单详情')).toBeVisible({ timeout: 30000 });
    await expect(page.getByText(quotationNo)).toBeVisible();
    // 描述列表标签 + 报价明细区（详情页无「基本信息」标题，改为断言真实存在的标签）
    await expect(page.getByText('客户', { exact: true })).toBeVisible();
    await expect(page.getByText('报价明细')).toBeVisible();
    // draft 报价单状态标签渲染为「草稿」
    await expect(page.getByText('草稿')).toBeVisible();
  });

  test('03-02 草稿报价单可编辑', async ({ page }) => {
    const { id } = await createDraftQuotation(page);
    await page.goto(`${BASE_URL}/quotations/${id}`);
    const editBtn = page.getByRole('button', { name: '编辑', exact: true });
    await expect(editBtn, 'draft 详情页应渲染「编辑」按钮').toBeVisible({ timeout: 30000 });
    await editBtn.click();
    await page.waitForURL(`${BASE_URL}/quotations/${id}/edit`, { timeout: 30000 });
    // create.vue 编辑模式标题「编辑报价单」，客户为必填表单控件（标签「客户」）
    await expect(page.getByText('编辑报价单')).toBeVisible({ timeout: 30000 });
    await expect(page.getByLabel('客户', { exact: true })).toBeVisible();
  });
});
