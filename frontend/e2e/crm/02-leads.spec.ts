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
 *   联系按钮 lead_status===LEAD_STATUS.NEW('new')（index.vue:221）、
 *   转化按钮 lead_status===LEAD_STATUS.QUALIFIED('qualified')（index.vue:229）、
 *   流失按钮 lead_status!==LEAD_STATUS.CONVERTED('converted')（index.vue:237）。
 * 后端 crm_lead 词表（models/status/bpm_crm_contract.rs::crm_lead::ALL）为小写
 *   new/contacted/qualified/assigned/converted/pool/lost；create_lead/update_lead_status 按此
 *   校验取值（services/crm/lead.rs::ensure_valid_lead_status，非法值→400 VALIDATION_ERROR），
 *   DB 侧另有 chk_crm_lead_lead_status CHECK。前端 LEAD_STATUS（utils/crm-status.ts）须与之逐字一致——
 *   这是「易失配的约定」而非既成事实：历史上前端 LEAD_STATUS 曾漏 assigned，导致该态线索进入列表即
 *   整页崩（normalizeLeadStatus 对词表外取值抛错），而本套件此前只 seed new/qualified，
 *   从未触发该缺口，故此句「逐字一致」在补齐 assigned 前是靠注释自证的假绿来源。
 *   现由 02-06 专门回归 assigned 的渲染，一致性缺口不再靠本注释背书（后端词表/CHECK 与前端映射的
 *   静态同源另由 backend/tests/crm_status_word_list_test.rs 锁死）。
 *   故此处按后端权威小写码建单：既过入参校验与 DB CHECK，又能在前端小写门控下正确渲染目标按钮。
 * 成功提示文案（前端实际 toast）：联系=「标记已联系成功」、转化=「转化成功」、
 *   流失=「标记已流失成功」（locales/zh-CN.ts crmLeads.message.*）。
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
    // 线索来源是 el-select（LeadFormTab.vue:23-39），getByLabel 命中 readonly combobox input，
    // EP 拦截其直接 click → 30s 超时。改按含该 label 的 form-item 锚定其内 .el-select 触发，选项取
    // body-level popper 的 .el-select-dropdown__item（对齐 purchase/inventory 既有写法）。
    const srcItem = dlg
      .locator('.el-form-item')
      .filter({ has: dlg.locator('.el-form-item__label', { hasText: '线索来源' }) })
      .first();
    await srcItem.locator('.el-select').first().click();
    await page.locator('.el-select-dropdown:visible .el-select-dropdown__item').first().click();
    await dlg.getByLabel('备注').fill('E2E 测试线索');
    await dlg.getByRole('button', { name: '确定' }).click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({
      timeout: 30000,
    });
  });

  test('02-03 线索可标记为已联系（new → contacted）', async ({ page }) => {
    // 方法一：建 new 线索 → 联系按钮渲染 → 定位自身行点击联系
    const { leadNo } = await seedLead(page, 'new');
    await gotoLeads(page);
    const row = page.getByRole('row').filter({ hasText: leadNo });
    const contactBtn = row.getByText('联系', { exact: false }).first();
    await expect(contactBtn, `定位 new 线索 ${leadNo} 的联系按钮失败`).toBeVisible({
      timeout: 10000,
    });
    await contactBtn.click();
    await page.getByRole('button', { name: /确定/ }).click();
    await expect(page.getByText(/联系成功/)).toBeVisible({ timeout: 30000 });
  });

  test('02-04 合格线索可转化为客户', async ({ page }) => {
    // 方法一：建 qualified 线索 → 转化按钮渲染 → 定位自身行点击转化
    const { leadNo } = await seedLead(page, 'qualified');
    await gotoLeads(page);
    const row = page.getByRole('row').filter({ hasText: leadNo });
    const convertBtn = row.getByText('转化', { exact: false }).first();
    await expect(convertBtn, `定位 qualified 线索 ${leadNo} 的转化按钮失败`).toBeVisible({
      timeout: 10000,
    });
    await convertBtn.click();
    await page.getByRole('button', { name: /确定|确认/ }).click();
    await expect(page.getByText(/转化成功/)).toBeVisible({ timeout: 30000 });
  });

  test('02-05 线索可标记为丢失', async ({ page }) => {
    // 方法一：建 new 线索（lead_status!=='converted'）→ 流失按钮渲染 → 定位自身行点击流失
    const { leadNo } = await seedLead(page, 'new');
    await gotoLeads(page);
    const row = page.getByRole('row').filter({ hasText: leadNo });
    // 行内按钮真实文案为「流失」（crmLeads.table.lost='流失'），非「丢失」
    const loseBtn = row.getByText('流失', { exact: false }).first();
    await expect(loseBtn, `定位线索 ${leadNo} 的流失按钮失败`).toBeVisible({ timeout: 10000 });
    await loseBtn.click();
    await page.getByRole('button', { name: /确定|确认/ }).click();
    await expect(page.getByText(/流失成功/)).toBeVisible({ timeout: 30000 });
  });

  test('02-06 已分配（assigned）线索状态标签正常渲染（回归：前端漏值致整页崩）', async ({
    page,
  }) => {
    // assigned 是后端 crm_lead::ALL 与 chk_crm_lead_lead_status 的合法态（services/crm/assign.rs
    // 自动分配/认领写入），前端 utils/crm-status.ts 的 normalizeLeadStatus 对词表外取值抛错，
    // 会使 leads/index.vue 的状态列（getStatusLabel → t(leadStatusLabelKey)）在渲染 assigned 行时
    // 让整页崩溃。历史缺陷：LEAD_STATUS 曾漏 assigned，而本套件此前无 assigned 种子从未触发该缺口。
    // 本用例把该覆盖缺口钉成回归：seed 一条 assigned 线索 → 打开列表页（gotoLeads 内含表格可见断言，
    // 即页面未崩的代理判据）→ 断该行的状态标签正常渲染为「已分配」（crmLeads.leadStatus.assigned）。
    const { leadNo } = await seedLead(page, 'assigned');
    await gotoLeads(page);
    const row = page.getByRole('row').filter({ hasText: leadNo });
    await expect(
      row,
      `列表未渲染 assigned 线索 ${leadNo}（疑似前端漏 assigned 致整页崩或状态列抛错）`
    ).toHaveCount(1);
    // 状态标签文案须为「已分配」（assigned 的 i18n 值），且页面存活至该行可见。
    await expect(row.getByText('已分配', { exact: true }).first()).toBeVisible({
      timeout: 10000,
    });
  });
});
