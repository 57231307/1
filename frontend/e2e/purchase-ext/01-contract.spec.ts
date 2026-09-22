// 采购扩展 E2E 套件 — 01 采购合同
// 创建时间: 2026-08-19
// 覆盖范围：采购合同创建 → 审批 → 执行（draft → pending → executing）
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, apiCallRaw, genCode, tryCleanup } from '../flow/helpers';

/**
 * 前置数据构造（方法一）：
 * 原用例用 `if (await btn.isVisible()) { ... }` 包裹，列表无可操作记录时整条用例
 * 一条断言都不执行仍判通过（假绿）。现改为：先经 API 真实创建处于目标状态的合同，
 * 再按合同编号定位到自己那一行点击行内操作，使断言必然执行。
 *
 * 后端词表依据（真缺陷，见 .monkeycode/doto.md）：
 *   purchase_contract 状态机为 draft → active → cancelled（backend/src/services/
 *   purchase_contract_service.rs:70 建单置 'draft'，:271 审批置 contract::ACTIVE），
 *   而执行（execute）要求 status == ACTIVE（同文件:185）。
 *   但前端 ContractTab.vue:103 的执行按钮渲染条件是 `row.status === 'pending'`——
 *   后端根本不产生 'pending' 态（contract 词表 bpm_crm_contract.rs:34 只有
 *   draft/active/cancelled），故执行按钮对任何真实数据都不渲染。01-04 因此只能
 *   采用方法二（显式断言前置存在）并记录该前后端状态词表不一致缺陷。
 */
const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** 取一个可用供应商 id（合同建单要求真实外键 supplier_id） */
async function ensureSupplierId(page: import('@playwright/test').Page): Promise<number> {
  const list = await apiCallRaw<{ items?: Array<{ id: number }> }>(
    page,
    'GET',
    '/purchase/suppliers?page=1&page_size=1'
  );
  if (list.items?.[0]?.id) return list.items[0].id;
  const created = await apiCall<{ id?: number }>(page, 'POST', '/purchase/suppliers', {
    supplier_name: `E2E供应商${Date.now()}`,
    supplier_short_name: 'E2E供',
    contact_phone: '13800000001',
  });
  if (!created.data?.id) throw new Error('无法创建供应商用于合同前置');
  return created.data.id;
}

/** 建一张 draft 采购合同，返回 { id, contract_no }；登记清理 */
async function seedDraftContract(
  page: import('@playwright/test').Page
): Promise<{ id: number; contractNo: string }> {
  const supplierId = await ensureSupplierId(page);
  const contractNo = genCode('E2E-PC');
  const created = await apiCall<{ id?: number }>(page, 'POST', '/purchase/purchase-contracts', {
    contract_no: contractNo,
    contract_name: `E2E 测试合同 ${contractNo}`,
    supplier_id: supplierId,
    total_amount: 100000,
    delivery_date: '2026-12-31',
    payment_terms: 'E2E 账期30天',
  });
  if (!created.data?.id) throw new Error(`建单失败：${JSON.stringify(created)}`);
  CLEANUP.push({
    path: `/purchase/purchase-contracts/${created.data.id}`,
    label: 'purchase_contract',
  });
  return { id: created.data.id, contractNo };
}

/** 进入采购合同 Tab 并按合同编号筛选出目标行（规避并发分片 .first() 误点） */
async function gotoContractTab(page: import('@playwright/test').Page): Promise<void> {
  await page.goto('/purchase-ext');
  await page.getByRole('tab', { name: /合同/ }).click();
  await expect(page.locator('table, .el-table')).toBeVisible({ timeout: 30000 });
}

test.describe('01 采购合同', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入采购扩展页面', async ({ page }) => {
    await page.goto('/purchase-ext');
    await expect(page.getByText(/采购/)).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('tab', { name: /合同/ })).toBeVisible();
  });

  test('01-02 新建采购合同', async ({ page }) => {
    await page.goto('/purchase-ext');
    await page.getByRole('tab', { name: /合同/ }).click();
    await page.getByRole('button', { name: /新建|创建/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    await page.getByLabel(/合同编号/).fill(`PC-${Date.now()}`);
    await page.getByLabel(/供应商/).fill('E2E 测试供应商');
    await page.getByLabel(/合同日期/).fill('2026-08-19');
    await page.getByLabel(/总金额/).fill('100000');
    await page.getByLabel(/币种/).click();
    await page.getByRole('option').first().click();
    await page
      .getByRole('button', { name: /确认|保存|提交/ })
      .last()
      .click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({ timeout: 30000 });
  });

  test('01-03 草稿合同可审批', async ({ page }) => {
    // 方法一：建一张 draft 合同（ContractTab.vue:95 审批按钮渲染条件 status==='draft'），
    // 定位自己那一行点击审批，断言必然执行
    const { contractNo } = await seedDraftContract(page);
    await gotoContractTab(page);
    const row = page.getByRole('row').filter({ hasText: contractNo });
    const approveBtn = row.getByText('审批', { exact: false }).first();
    await expect(approveBtn, `定位草稿合同 ${contractNo} 的审批按钮失败`).toBeVisible({
      timeout: 10000,
    });
    await approveBtn.click();
    await page.getByRole('button', { name: /确定/ }).click();
    await expect(page.getByText(/审批成功/)).toBeVisible({ timeout: 30000 });
  });

  test('01-04 待执行合同可执行（pending → executing）', async ({ page }) => {
    await gotoContractTab(page);
    // 方法二：后端合同状态词表无 'pending'（审批直接置 active），
    // 前端执行按钮渲染条件 row.status==='pending' 与后端不一致，无法用 API 构造该态。
    // 不改断言方向掩盖缺陷（见文件头说明与 .monkeycode/doto.md），显式断言前置存在。
    const execBtn = page.getByText('执行', { exact: false }).first();
    await expect(
      await execBtn.isVisible(),
      '缺少 status==="pending" 的可执行合同：前端 ContractTab.vue:103 执行按钮渲染条件 ' +
        'pending 与后端合同词表（draft/active/cancelled，审批后置 active）不一致，属真缺陷'
    ).toBe(true);
    await execBtn.click();
    await page.getByRole('button', { name: /确定/ }).click();
    await expect(page.getByText(/执行成功/)).toBeVisible({ timeout: 30000 });
  });
});
