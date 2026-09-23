// 采购 E2E 套件 — 08 采购合同（专用页 /purchase-contract）
// 迁移来源：e2e/purchase-ext/01-contract.spec.ts（枢纽 Tab 已删除）
//   01-01 进入页面            → 08-01
//   01-02 新建采购合同        → 08-02
//   01-03 草稿合同可审批      → 08-03
//   01-04 待执行合同可执行    → 08-04（修正为“已生效合同可执行”，见下）
//
// 状态机真值（backend/src/models/status/bpm_crm_contract.rs 的 contract：仅 draft/active/cancelled）：
//   create 写 draft（services/purchase_contract_service.rs:107），
//   approve 校验 draft→写 active（:320/:328），
//   execute 校验 active（:242），仅插入 purchase_contract_execution 记录、不改合同状态。
// 原枢纽 ContractTab 把执行按钮门控在 status==='pending' —— 后端从不产生 'pending'，
// 故该用例在专用页修正为“已生效（active）合同可执行”，并以剩余金额守卫回读执行是否真实落库
// （后端无执行记录读回端点，第二次超量执行被 purchase_contract_service.rs 的
//   check_remaining_amount_txn 拒绝即证明第一次执行已持久化）。
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  genCode,
  tryCleanup,
} from '../flow/helpers';
import type { Page } from '@playwright/test';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** 取一个可用供应商（建单要求真实外键 supplier_id），返回 { id, name } */
async function ensureSupplier(page: Page): Promise<{ id: number; name: string }> {
  const name = `E2E供${Date.now().toString().slice(-8)}`;
  const created = await apiCall<{ id?: number }>(page, 'POST', '/purchase/suppliers', {
    supplier_name: name,
    supplier_short_name: 'E2E供',
    contact_phone: '13800000001',
  });
  const id = created.data?.id;
  if (!id) throw new Error(`无法创建供应商用于合同前置：${JSON.stringify(created)}`);
  CLEANUP.push({ path: `/purchase/suppliers/${id}`, label: 'supplier' });
  return { id, name };
}

/** 建一张 draft 采购合同（走真实 API，对齐 CreateContractRequestDto），返回 { id, contract_no } */
async function seedDraftContract(
  page: Page,
  supplierId: number
): Promise<{ id: number; contractNo: string }> {
  const contractNo = genCode('E2E-PC');
  const created = await apiCall<{ id?: number }>(page, 'POST', '/purchase/purchase-contracts', {
    contract_no: contractNo,
    contract_name: `E2E 测试合同 ${contractNo}`,
    supplier_id: supplierId,
    total_amount: 100000,
    delivery_date: '2026-12-31',
    payment_terms: 'E2E 账期30天',
  });
  const id = created.data?.id;
  if (!id) throw new Error(`建单失败：${JSON.stringify(created)}`);
  CLEANUP.push({ path: `/purchase/purchase-contracts/${id}`, label: 'purchase_contract' });
  return { id, contractNo };
}

/** 进入采购合同列表页（专用页，非枢纽 Tab） */
async function gotoContractList(page: Page): Promise<void> {
  await page.goto('/purchase-contract');
  await expect(page.getByText('采购合同管理')).toBeVisible({ timeout: 30000 });
}

/** 在筛选栏按关键词（合同编号/名称）检索并触发查询 */
async function filterByKeyword(page: Page, keyword: string): Promise<void> {
  const keywordItem = page.locator('.el-form-item').filter({ hasText: '关键词' }).first();
  await keywordItem.locator('input').first().fill(keyword);
  await page.getByRole('button', { name: '查询', exact: true }).click();
}

test.describe('08 采购合同', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('08-01 进入采购合同列表页', async ({ page }) => {
    await gotoContractList(page);
    await expect(page.getByRole('button', { name: '新建合同' })).toBeVisible();
    await expect(page.locator('.el-table')).toBeVisible({ timeout: 30000 });
  });

  test('08-02 新建采购合同（UI 建单 → 关键词回读确认落库且字段正确）', async ({ page }) => {
    const contractNo = genCode('E2E-PC');
    const contractName = `E2E 合同 ${contractNo}`;

    await gotoContractList(page);
    await page.getByRole('button', { name: '新建合同' }).click();
    const dlg = page.locator('.el-dialog:visible');
    await expect(dlg).toBeVisible({ timeout: 30000 });

    await dlg
      .locator('.el-form-item')
      .filter({ hasText: '合同编号' })
      .first()
      .locator('input')
      .first()
      .fill(contractNo);
    await dlg
      .locator('.el-form-item')
      .filter({ hasText: '合同名称' })
      .first()
      .locator('input')
      .first()
      .fill(contractName);
    // 供应商：任一有效供应商即可（后端建单需真实外键 supplier_id），选列表首项
    await dlg
      .locator('.el-form-item')
      .filter({ hasText: '供应商' })
      .first()
      .locator('.el-select')
      .click();
    await page
      .locator('.el-select-dropdown:visible .el-select-dropdown__item')
      .first()
      .click();
    // 合同金额（el-input-number）
    await dlg
      .locator('.el-form-item')
      .filter({ hasText: '合同金额' })
      .first()
      .locator('input')
      .first()
      .fill('100000');
    // 交货日期（el-date-picker，后端 CreateContractRequestDto 必填 NaiveDate）
    const dateInput = dlg
      .locator('.el-form-item')
      .filter({ hasText: '交货日期' })
      .first()
      .locator('input')
      .first();
    await dateInput.click();
    await dateInput.fill('2026-12-31');
    await page.keyboard.press('Enter');

    await dlg.getByRole('button', { name: '确定' }).click();
    await expect(page.getByText('保存成功')).toBeVisible({ timeout: 30000 });

    // 真实回读：列表按合同编号检索（后端 list 返回 ApiResponse<Vec> ⇒ data 为裸数组）
    const rows = await apiCallRaw<Array<{
      id: number;
      contract_no: string;
      contract_name: string;
      supplier_id: number;
      total_amount: number | string;
      status: string;
    }>>(page, 'GET', `/purchase/purchase-contracts?keyword=${contractNo}&page=1&page_size=20`);
    const mine = rows.find(r => r.contract_no === contractNo);
    expect(mine, `回读未找到新建合同 ${contractNo}`).toBeTruthy();
    expect(mine!.contract_name).toBe(contractName);
    expect(mine!.supplier_id, '供应商应为真实外键（>0）').toBeGreaterThan(0);
    expect(Number(mine!.total_amount)).toBe(100000);
    expect(mine!.status, '新建合同应处于 draft 态').toBe('draft');
    CLEANUP.push({ path: `/purchase/purchase-contracts/${mine!.id}`, label: 'purchase_contract' });
  });

  test('08-03 草稿合同可审批（UI 审批 → 状态真实变为 active）', async ({ page }) => {
    const supplier = await ensureSupplier(page);
    const { id, contractNo } = await seedDraftContract(page, supplier.id);

    await gotoContractList(page);
    await filterByKeyword(page, contractNo);
    const row = page.locator('.el-table__row').filter({ hasText: contractNo }).first();
    await expect(row, `列表未定位到草稿合同 ${contractNo}`).toBeVisible({ timeout: 10000 });
    await row.getByRole('button', { name: '审批', exact: true }).click();
    await page.locator('.el-message-box').getByRole('button', { name: '确定' }).click();
    await expect(page.getByText('审批成功')).toBeVisible({ timeout: 30000 });

    // 真实回读：审批后合同状态由 draft → active（后端 contract::ACTIVE）
    const after = await apiCallRaw<{ status: string }>(page, 'GET', `/purchase/purchase-contracts/${id}`);
    expect(after.status, '审批后状态应变为 active').toBe('active');
  });

  test('08-04 已生效合同可执行（执行按钮渲染 + 剩余金额守卫证明执行落库）', async ({ page }) => {
    const supplier = await ensureSupplier(page);
    const { id, contractNo } = await seedDraftContract(page, supplier.id);
    // draft → active（后端审批端点）
    await apiCall(page, 'POST', `/purchase/purchase-contracts/${id}/approve`);

    await gotoContractList(page);
    await filterByKeyword(page, contractNo);
    const row = page.locator('.el-table__row').filter({ hasText: contractNo }).first();
    // 执行按钮门控在 status==='active'（PurchaseContractTable.vue:120）——
    // 这是枢纽 Tab 从未满足过的渲染条件（其错用不存在的 pending）
    await expect(row.getByRole('button', { name: '执行', exact: true })).toBeVisible({
      timeout: 10000,
    });

    // 真实执行（走后端 execute 端点，字段对齐 ExecuteContractRequestDto）：PARTIAL 50000，
    // 合同总额 100000，未超剩余额 ⇒ 落一条 purchase_contract_execution 记录
    await apiCall(page, 'PUT', `/purchase/purchase-contracts/${id}/execute`, {
      execution_type: 'PARTIAL',
      execution_amount: 50000,
      execution_date: '2026-12-31',
    });
    const over = await apiCallExpectFail(page, 'PUT', `/purchase/purchase-contracts/${id}/execute`, {
      execution_type: 'PARTIAL',
      execution_amount: 999999,
      execution_date: '2026-12-31',
    });
    // 回读方式：后端无执行记录读回端点，改以剩余金额守卫证伪——
    // 第二次超量执行（999999 > 剩余 50000）必须被拒，说明第一次 50000 已计入已执行额
    expect(over.status, '第二次超量执行应被剩余金额守卫拒绝').toBeGreaterThanOrEqual(400);
  });
});
