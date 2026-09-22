// 销售扩展 E2E 套件 — 01 销售合同
// 创建时间: 2026-08-19（本轮去假绿改造 2026-09-23）
// 覆盖范围：进入合同页 → 合同列表真实渲染 → 草稿合同 UI 审批
//
// 去假绿说明：01-03 原用 `getByRole('link', { name: /审批/ })` + `if (isVisible)` 判断审批按钮，
// 而审批是 `el-button`（role=button，非 link）且仅当 row.status==='draft' 渲染 → 选择器/条件双重
// 不命中 → 0 断言 → 假绿。改为：真实造一张 draft 合同（走后端权威合同，含 contract_name /
// delivery_date / customer_id）→ 硬断言列表出现该合同号 → 用 role=button 点「审批」→ 断言 toast
// 与后端状态。合同状态词表（backend/src/models/status/bpm_crm_contract.rs::contract，小写）：
// draft →（approve）→ active；另 active/completed/cancelled。
//
// 本轮另修复产品缺陷（见 doto）：ContractTab 列表读取 res.data.items，但后端 list_contracts
// 返回 ApiResponse<Vec>（data 为数组）→ 列表恒空。已改前端归一化数组形状。
import { test, expect, type Page } from '@playwright/test';
import {
  loginViaUI,
  ensureTestEntities,
  getCtx,
  apiCall,
  apiCallRaw,
  BASE_URL,
} from '../flow/helpers';

/** 通过后端权威合同创建一张 draft 销售合同，返回 id 与合同号 */
async function createDraftContract(
  page: Page,
  suffix: string
): Promise<{ id: number; no: string }> {
  const ctx = getCtx();
  const no = `E2E-SC-${suffix}`;
  const res = await apiCall<{ id?: number }>(page, 'POST', '/sales/sales-contracts', {
    contract_no: no,
    contract_name: `E2E 销售合同 ${suffix}`,
    customer_id: ctx.customerId,
    total_amount: '200000',
    contract_type: 'sale',
    delivery_date: new Date(Date.now() + 30 * 86400000).toISOString().slice(0, 10),
    items: [
      {
        product_name: 'E2E 坯布',
        unit: '米',
        quantity: '1000',
        unit_price: '200',
      },
    ],
  });
  const id = res.data?.id;
  expect(id, '前置失败：销售合同创建未返回 id').toBeTruthy();
  return { id: id as number, no };
}

async function contractStatus(page: Page, id: number): Promise<string> {
  const c = await apiCallRaw<{ status: string }>(page, 'GET', `/sales/sales-contracts/${id}`);
  return c.status;
}

test.describe('01 销售合同', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('01-01 进入销售扩展合同页', async ({ page }) => {
    await page.goto(`${BASE_URL}/sales-ext`);
    await expect(page.getByRole('tab', { name: '销售合同' })).toBeVisible({ timeout: 30000 });
    await page.getByRole('tab', { name: '销售合同' }).click();
    await expect(page.getByRole('button', { name: '新建合同' })).toBeVisible({ timeout: 30000 });
  });

  test('01-02 合同列表渲染真实数据', async ({ page }) => {
    const { no } = await createDraftContract(page, Date.now().toString().slice(-6));
    await page.goto(`${BASE_URL}/sales-ext`);
    await page.getByRole('tab', { name: '销售合同' }).click();
    // 硬断言：真实创建的合同号必须渲染到列表（后端返回数组，前端归一化后应可见）
    await expect(
      page.getByText(no),
      `合同 ${no} 应渲染到列表（列表数据形状错误/为空即判红，不再空过）`
    ).toBeVisible({ timeout: 30000 });
  });

  test('01-03 草稿合同可审批', async ({ page }) => {
    const suffix = Date.now().toString().slice(-6);
    const { id, no } = await createDraftContract(page, suffix);
    expect(await contractStatus(page, id), '新建合同初始状态应为 draft').toBe('draft');

    await page.goto(`${BASE_URL}/sales-ext`);
    await page.getByRole('tab', { name: '销售合同' }).click();
    const row = page.locator('tr.el-table__row', { hasText: no });
    await expect(row, `列表应出现 draft 合同 ${no}`).toBeVisible({ timeout: 30000 });
    // 审批按钮为 el-button（role=button），仅 draft 行渲染
    const approveBtn = row.getByRole('button', { name: '审批', exact: true });
    await expect(approveBtn, 'draft 合同行应渲染「审批」按钮').toBeVisible({ timeout: 30000 });
    await approveBtn.click();
    await page.locator('.el-message-box__btns .el-button--primary').click();
    await expect(page.getByText('合同审批成功')).toBeVisible({ timeout: 30000 });
    // 后端 sales_contract_service::approve：draft → active
    expect(await contractStatus(page, id), '审批后合同状态应为 active').toBe('active');
  });
});
