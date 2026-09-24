// CRM 客户关系管理 E2E 套件 — 02 线索管理
// 创建时间: 2026-08-19
// 覆盖范围：线索创建 → 联系 → 转化 → 丢失（完整线索生命周期）
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, genCode, tryCleanup } from '../flow/helpers';

/**
 * 前置数据构造（方法一）：
 * 原 `if (await btn.isVisible())` 在无对应状态线索时零断言假绿。
 * 现按前端 leads/index.vue 的按钮渲染条件构造目标态线索，再按 lead_no 定位自身行操作。
 *   联系按钮 lead_status==='NEW'（:234）、转化 lead_status==='QUALIFIED'（:242）、
 *   丢失 lead_status!=='CONVERTED'（:250）。
 * 后端 crm_lead 词表（backend/src/models/status/bpm_crm_contract.rs:116）为小写
 *   new/converted/pool/lost，且无 contacted/qualified——与前端大写词表（NEW/CONTACTED/
 *   QUALIFIED/CONVERTED/LOST）不一致，属真缺陷（见 .monkeycode/doto.md）。后端 create 原样
 *   存 lead_status 字符串（services/crm/lead.rs），故此处按前端期望的大写值建单以驱动按钮渲染。
 * 成功提示文案（前端实际 toast）：联系=「标记已联系成功」、转化=「转化成功」、
 *   丢失=「标记已流失成功」（locales/zh-CN.ts crmLeads.message.*）。原用例误写「更新成功」，已修正为真实文案。
 */
const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** 建一条指定 lead_status 的线索，返回 { id, leadNo }；登记清理 */
async function seedLead(
  page: import('@playwright/test').Page,
  leadStatus: string
): Promise<{ id: number; leadNo: string }> {
  const leadNo = genCode('E2E-LD');
  const created = await apiCall<{ id?: number }>(page, 'POST', '/crm/leads', {
    lead_no: leadNo,
    lead_status: leadStatus,
    lead_source: 'WEBSITE',
    company_name: `E2E 测试公司 ${leadNo}`,
    contact_name: '张三',
    mobile_phone: '13900139000',
    email: 'e2e-lead@test.com',
    priority: 'MEDIUM',
  });
  if (!created.data?.id) throw new Error(`建线索失败：${JSON.stringify(created)}`);
  CLEANUP.push({ path: `/crm/leads/${created.data.id}`, label: 'crm_lead' });
  return { id: created.data.id, leadNo };
}

async function gotoLeads(page: import('@playwright/test').Page): Promise<void> {
  await page.goto('/crm/leads');
  await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
}

test.describe('02 线索管理', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('02-01 进入线索管理页面', async ({ page }) => {
    // 真实新建按钮文案为「新建线索」（src/views/crm/leads/index.vue:27 → i18n crmLeads.create='新建线索'）
    await page.goto('/crm/leads');
    await expect(page.getByRole('heading', { name: '线索管理' })).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('button', { name: '新建线索' })).toBeVisible();
  });

  test('02-02 创建新线索', async ({ page }) => {
    await page.goto('/crm/leads');
    await page.getByRole('button', { name: '新建线索' }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    // 弹窗内「线索来源」与筛选栏同名 label，限定到 .el-dialog 作用域避免 strict-mode 命中多元素；
    // 提交按钮真实文案为「确定」（crmLeads.leadForm.confirm）。
    // 注意：LeadFormTab 表单规则要求「负责人 owner_id」必填，本用例未填写负责人，
    // 且 /crm/leads 页面当前存在前端加载崩溃（另一前端专家修复中）——
    // 此用例在页面修复前会因校验/崩溃而红，非选择器问题，不做凑数。
    const dlg = page.locator('.el-dialog');
    await dlg.getByLabel('公司名称').fill('E2E 测试公司');
    await dlg.getByLabel('联系人').fill('李四');
    await dlg.getByLabel('手机号').fill('13900139000');
    await dlg.getByLabel('邮箱').fill('li@test.com');
    await dlg.getByLabel('线索来源').click();
    await page.getByRole('option').first().click();
    await dlg.getByLabel('备注').fill('E2E 测试线索');
    await dlg.getByRole('button', { name: '确定' }).click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({
      timeout: 30000,
    });
  });

  test('02-03 线索可标记为已联系（NEW → CONTACTED）', async ({ page }) => {
    // 方法一：建 NEW 线索 → 联系按钮渲染 → 定位自身行点击联系
    const { leadNo } = await seedLead(page, 'NEW');
    await gotoLeads(page);
    const row = page.getByRole('row').filter({ hasText: leadNo });
    const contactBtn = row.getByText('联系', { exact: false }).first();
    await expect(contactBtn, `定位 NEW 线索 ${leadNo} 的联系按钮失败`).toBeVisible({
      timeout: 10000,
    });
    await contactBtn.click();
    await page.getByRole('button', { name: /确定/ }).click();
    await expect(page.getByText(/联系成功/)).toBeVisible({ timeout: 30000 });
  });

  test('02-04 合格线索可转化为客户', async ({ page }) => {
    // 方法一：建 QUALIFIED 线索 → 转化按钮渲染 → 定位自身行点击转化
    const { leadNo } = await seedLead(page, 'QUALIFIED');
    await gotoLeads(page);
    const row = page.getByRole('row').filter({ hasText: leadNo });
    const convertBtn = row.getByText('转化', { exact: false }).first();
    await expect(convertBtn, `定位 QUALIFIED 线索 ${leadNo} 的转化按钮失败`).toBeVisible({
      timeout: 10000,
    });
    await convertBtn.click();
    await page.getByRole('button', { name: /确定|确认/ }).click();
    await expect(page.getByText(/转化成功/)).toBeVisible({ timeout: 30000 });
  });

  test('02-05 线索可标记为丢失', async ({ page }) => {
    // 方法一：建 NEW 线索（lead_status!=='CONVERTED'）→ 丢失按钮渲染 → 定位自身行点击丢失
    const { leadNo } = await seedLead(page, 'NEW');
    await gotoLeads(page);
    const row = page.getByRole('row').filter({ hasText: leadNo });
    // 行内按钮真实文案为「流失」（crmLeads.table.lost='流失'），非「丢失」
    const loseBtn = row.getByText('流失', { exact: false }).first();
    await expect(loseBtn, `定位线索 ${leadNo} 的流失按钮失败`).toBeVisible({ timeout: 10000 });
    await loseBtn.click();
    await page.getByRole('button', { name: /确定|确认/ }).click();
    await expect(page.getByText(/流失成功/)).toBeVisible({ timeout: 30000 });
  });
});
