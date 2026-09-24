// CRM 客户关系管理 E2E 套件 — 03 商机管理
// 创建时间: 2026-08-19
// 覆盖范围：商机创建 → 跟进 → 推进阶段 → 赢单/输单
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, apiCallRaw, genCode, tryCleanup } from '../flow/helpers';

/**
 * 前置数据构造（方法一）：
 * 原 `if (await btn.isVisible())` 在无对应阶段商机时零断言假绿。
 * 按前端 opportunities/index.vue 按钮渲染条件建商机，再按 opportunity_no 定位自身行操作：
 *   跟进按钮 stage!==OPPORTUNITY_STAGE.CLOSED_WON && !==CLOSED_LOST（index.vue:193）、
 *   成交(赢单)按钮 stage===OPPORTUNITY_STAGE.NEGOTIATION（index.vue:204，按钮文案「成交」）、
 *   流失(输单)按钮 stage!==CLOSED_WON && !==CLOSED_LOST（index.vue:213，按钮文案「流失」）。
 * 后端 crm_opportunity 阶段词表（models/status/bpm_crm_contract.rs::crm_opportunity::ALL_STAGES）
 *   为大写 QUALIFICATION/NEEDS_ANALYSIS/PROPOSAL/NEGOTIATION/CLOSED_WON/CLOSED_LOST，
 *   DB 侧有 chk_crm_opportunity_stage CHECK；状态词表 OPEN/CLOSED_WON/CLOSED_LOST（create 固定写 OPEN）。
 *   前端 OPPORTUNITY_STAGE（utils/crm-status.ts）与之逐字一致。NEGOTIATION 本身即合法阶段，
 *   用它建单既过 CHECK 又能驱动上述按钮，无需使用非法值。
 * 真实 toast 文案（locales/zh-CN.ts crmOpportunities.message.*）：赢单含「成交」、输单含「流失」、
 *   跟进保存成功提示含「跟进成功」。
 */
const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

async function ensureCustomerId(page: import('@playwright/test').Page): Promise<number> {
  const list = await apiCallRaw<{ items?: Array<{ id: number }> }>(
    page,
    'GET',
    '/crm/customers?page=1&page_size=1'
  );
  if (list.items?.[0]?.id) return list.items[0].id;
  const created = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
    customer_name: `E2E 客户 ${Date.now()}`,
  });
  if (!created.data?.id) throw new Error('无法准备客户用于建商机');
  return created.data.id;
}

/** 建一条指定阶段的商机，返回 { id, oppNo } */
async function seedOpportunity(
  page: import('@playwright/test').Page,
  stage: string
): Promise<{ id: number; oppNo: string }> {
  const customerId = await ensureCustomerId(page);
  const oppNo = genCode('E2E-OPP');
  const created = await apiCall<{ id?: number }>(page, 'POST', '/crm/opportunities', {
    opportunity_no: oppNo,
    opportunity_name: oppNo,
    customer_id: customerId,
    opportunity_stage: stage,
    estimated_amount: 100000,
    win_probability: 60,
    expected_close_date: '2026-12-31',
  });
  if (!created.data?.id) throw new Error(`建商机失败：${JSON.stringify(created)}`);
  CLEANUP.push({ path: `/crm/opportunities/${created.data.id}`, label: 'crm_opportunity' });
  return { id: created.data.id, oppNo };
}

async function gotoOpportunities(page: import('@playwright/test').Page): Promise<void> {
  await page.goto('/crm/opportunities');
  await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
}

test.describe('03 商机管理', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('03-01 进入商机管理页面', async ({ page }) => {
    // 真实新建按钮文案为「新建商机」（src/views/crm/opportunities/index.vue:24 → crmOpportunities.create='新建商机'）
    await page.goto('/crm/opportunities');
    await expect(page.getByRole('heading', { name: '商机管理' })).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('button', { name: '新建商机' })).toBeVisible();
  });

  test('03-02 创建新商机', async ({ page }) => {
    await page.goto('/crm/opportunities');
    await page.getByRole('button', { name: '新建商机' }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    // 弹窗字段真实 label（crmOpportunityForm.*）：商机名称/客户/商机类型/预估金额/成交概率/预计成交；
    // 提交按钮真实文案为「确定」（crmOpportunityForm.confirm）。限定到 .el-dialog 作用域。
    // 注意：OpportunityFormTab 表单规则还要求「商机阶段 opportunity_stage」「负责人 owner_id」必填，
    // 本用例未填写二者，且 /crm/opportunities 页面当前存在前端加载崩溃（另一前端专家修复中）——
    // 此用例在页面修复前会因校验/崩溃而红，非选择器问题，不做凑数。
    const dlg = page.locator('.el-dialog');
    await dlg.getByLabel('商机名称').fill(`E2E 商机 ${Date.now()}`);
    await dlg.getByLabel('客户').click();
    await page.getByRole('option').first().click();
    await dlg.getByLabel('商机类型').click();
    await page.getByRole('option').first().click();
    await dlg.getByLabel('预估金额').fill('100000');
    await dlg.getByLabel('预计成交').fill('2026-12-31');
    await dlg.getByRole('button', { name: '确定' }).click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({
      timeout: 30000,
    });
  });

  test('03-03 商机可添加跟进记录', async ({ page }) => {
    // 方法一：建 NEGOTIATION 商机 → 跟进按钮渲染 → 定位自身行点击跟进并保存
    const { oppNo } = await seedOpportunity(page, 'NEGOTIATION');
    await gotoOpportunities(page);
    const row = page.getByRole('row').filter({ hasText: oppNo });
    const followBtn = row.getByText('跟进', { exact: false }).first();
    await expect(followBtn, `定位商机 ${oppNo} 的跟进按钮失败`).toBeVisible({ timeout: 10000 });
    await followBtn.click();
    await expect(page.locator('.el-dialog')).toBeVisible();
    // 跟进弹窗真实 label「跟进内容」（crmOpportunityFollow.content），confirm「确定」，成功 toast「跟进成功」
    const dlg = page.locator('.el-dialog');
    await dlg.getByLabel('跟进内容').fill('E2E 测试跟进：客户确认需求');
    await dlg.getByRole('button', { name: '确定' }).click();
    await expect(page.getByText('跟进成功')).toBeVisible({ timeout: 30000 });
  });

  test('03-04 谈判阶段商机可赢单', async ({ page }) => {
    // 方法一：建 NEGOTIATION 商机 → 「成交」按钮（赢单）渲染 → 点击 → 断言真实 toast「已标记为成交」
    const { oppNo } = await seedOpportunity(page, 'NEGOTIATION');
    await gotoOpportunities(page);
    const row = page.getByRole('row').filter({ hasText: oppNo });
    const winBtn = row.getByText('成交', { exact: false }).first();
    await expect(winBtn, `定位商机 ${oppNo} 的成交(赢单)按钮失败`).toBeVisible({ timeout: 10000 });
    await winBtn.click();
    await page.getByRole('button', { name: /确定|确认/ }).click();
    await expect(page.locator('.el-message--success', { hasText: '成交' })).toBeVisible({
      timeout: 30000,
    });
  });

  test('03-05 商机可标记为输单', async ({ page }) => {
    // 方法一：建 NEGOTIATION 商机 → 「流失」按钮（输单）渲染 → 点击 → 断言真实 toast「已标记为流失」
    const { oppNo } = await seedOpportunity(page, 'NEGOTIATION');
    await gotoOpportunities(page);
    const row = page.getByRole('row').filter({ hasText: oppNo });
    const loseBtn = row.getByText('流失', { exact: false }).first();
    await expect(loseBtn, `定位商机 ${oppNo} 的流失(输单)按钮失败`).toBeVisible({ timeout: 10000 });
    await loseBtn.click();
    await page.getByRole('button', { name: /确定|确认/ }).click();
    await expect(page.locator('.el-message--success', { hasText: '流失' })).toBeVisible({
      timeout: 30000,
    });
  });
});
