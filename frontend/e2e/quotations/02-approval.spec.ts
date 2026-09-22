// 报价单 E2E 套件 — 02 报价单审批与转订单
// 创建时间: 2026-08-19（本轮去假绿改造 2026-09-23）
// 覆盖范围：提交审批（draft → pending_approval/approved） → 批准（→ approved） → 转订单（→ converted） → 取消（→ cancelled）
//
// 去假绿说明：原 4 个用例在报价单「列表页」用 `if (await btn.isVisible())` 找提交/批准/转订单按钮。
// 但这些按钮渲染在报价单「详情页」(src/views/quotations/detail.vue，按 status 计算 canSubmit/canApprove/
// canConvert/canCancel)，列表页根本没有提交/批准 → 条件恒为假 → 一个断言都不执行 → 假绿。
// 现改为：真实造数据到对应状态 → 进入详情页 → 硬断言按钮可见（Tier A：该状态必须渲染此按钮）→
// 点击 → 断言 toast + 后端真实状态字面量。
// 状态词表（小写，backend/src/models/status/sales.rs::quotation / quotation_ext）：
//   draft →（submit，金额 < 10 万自批）→ approved；
//   draft →（submit，金额 ≥ 10 万走 BPM）→ pending_approval →（approve）→ approved →（convert）→ converted；
//   draft →（cancel）→ cancelled。
import { test, expect, type Page } from '@playwright/test';
import {
  loginViaUI,
  ensureTestEntities,
  getCtx,
  apiCall,
  apiCallRaw,
  BASE_URL,
} from '../flow/helpers';

/** 创建一张草稿报价单；amount 作为明细数量、单价固定 1，控制 submit 落点：<10 万自批→approved，≥10 万→pending_approval */
async function createDraftQuotation(page: Page, amount: number): Promise<number> {
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
        quantity: String(amount),
        unit_price: '1',
        unit_price_with_tax: '1',
      },
    ],
    notes: 'E2E 报价单审批用例',
  });
  const id = res.data?.id;
  expect(id, '前置失败：报价单创建未返回 id').toBeTruthy();
  const created = await apiCallRaw<{ status: string; total_amount: number | string }>(
    page,
    'GET',
    `/quotations/${id}`
  );
  expect(created.status, '新建报价单应为 draft').toBe('draft');
  expect(
    Number(created.total_amount),
    `报价单金额应可控为 ${amount}（决定 submit 自批/审批落点）`
  ).toBe(amount);
  return id as number;
}

async function quotationStatus(page: Page, id: number): Promise<string> {
  const q = await apiCallRaw<{ status: string }>(page, 'GET', `/quotations/${id}`);
  return q.status;
}

test.describe('02 报价单审批与转订单', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('02-01 草稿报价单可提交审批', async ({ page }) => {
    const id = await createDraftQuotation(page, 1000); // < 10 万 → submit 自批
    await page.goto(`${BASE_URL}/quotations/${id}`);
    const submitBtn = page.getByRole('button', { name: '提交审批', exact: true });
    await expect(submitBtn, 'draft 报价单详情页应渲染「提交审批」按钮').toBeVisible({
      timeout: 30000,
    });
    await submitBtn.click();
    await expect(page.getByText('已提交审批')).toBeVisible({ timeout: 30000 });
    const status = await quotationStatus(page, id);
    expect(['approved', 'pending_approval'], `提交后状态应离开 draft（实际 ${status}）`).toContain(
      status
    );
  });

  test('02-02 待审批报价单可批准', async ({ page }) => {
    const id = await createDraftQuotation(page, 200000); // ≥ 10 万 → submit 走 BPM → pending_approval
    await apiCall(page, 'POST', `/quotations/${id}/submit`);
    expect(await quotationStatus(page, id), '金额 ≥ 10 万提交后应为 pending_approval').toBe(
      'pending_approval'
    );

    await page.goto(`${BASE_URL}/quotations/${id}`);
    const approveBtn = page.getByRole('button', { name: '批准', exact: true });
    await expect(approveBtn, 'pending_approval 详情页应渲染「批准」按钮').toBeVisible({
      timeout: 30000,
    });
    await approveBtn.click();
    await page.locator('.el-message-box__btns .el-button--primary').click();
    await expect(page.getByText('已批准')).toBeVisible({ timeout: 30000 });
    expect(await quotationStatus(page, id), '批准后状态应为 approved').toBe('approved');
  });

  test('02-03 已批准报价单可转为销售订单', async ({ page }) => {
    const id = await createDraftQuotation(page, 200000);
    await apiCall(page, 'POST', `/quotations/${id}/submit`);
    const afterSubmit = await quotationStatus(page, id);
    if (afterSubmit !== 'approved') {
      await apiCall(page, 'POST', `/quotations/${id}/approve`);
    }
    expect(await quotationStatus(page, id), '转订单前置：报价单应为 approved').toBe('approved');

    await page.goto(`${BASE_URL}/quotations/${id}`);
    const convertBtn = page.getByRole('button', { name: '转销售订单' });
    await expect(convertBtn, 'approved 详情页应渲染「转销售订单」按钮').toBeVisible({
      timeout: 30000,
    });
    await convertBtn.click();
    await page.locator('.el-message-box__btns .el-button--primary').click();
    await expect(page.getByText('转订单成功')).toBeVisible({ timeout: 30000 });

    const conv = await apiCallRaw<{ status: string; converted_sales_order_id?: number }>(
      page,
      'GET',
      `/quotations/${id}`
    );
    expect(conv.status, '转订单后状态应为 converted').toBe('converted');
    expect(conv.converted_sales_order_id, '转订单应回填 converted_sales_order_id').toBeTruthy();
  });

  test('02-04 草稿报价单可取消', async ({ page }) => {
    const id = await createDraftQuotation(page, 1000);
    await page.goto(`${BASE_URL}/quotations/${id}`);
    const cancelBtn = page.getByRole('button', { name: '取消', exact: true });
    await expect(cancelBtn, 'draft 详情页应渲染「取消」按钮').toBeVisible({ timeout: 30000 });
    await cancelBtn.click();
    await page.locator('.el-message-box__btns .el-button--primary').click();
    await expect(page.getByText('已取消')).toBeVisible({ timeout: 30000 });
    expect(await quotationStatus(page, id), '取消后状态应为 cancelled').toBe('cancelled');
  });
});
