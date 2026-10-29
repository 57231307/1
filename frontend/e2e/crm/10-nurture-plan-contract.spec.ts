// CRM 线索培育计划创建与执行 契约 E2E — crm/10
//
// 功能（被测业务规则，逐条对后端源码核实，非编造）：
//   培育计划是挂在线索上的可执行工单：创建即 pending 且不写执行时间与结果；
//   执行动作是一次性的状态跃迁（pending → executed，写 executed_at 与固定结果文案），
//   已执行的计划不可重复执行；执行之前还要过所属线索的行级可见门与跨 owner 写门。
// 被测端点与注册点：
//   - POST /crm/leads/nurture-plans → handlers/crm_handler::create_nurture_plan
//     → services/crm/lead::create_nurture_plan（status 固定 "pending"、executed_at/result
//     显式置 None、created_by 注入操作人，写列口径见该函数体）
//   - GET  /crm/leads/nurture-plans → handlers/crm_handler::list_nurture_plans
//     （入参 NurturePlanQuery {lead_id, status}，带 lead_id 时先过线索可见门，
//     data 是裸数组——list_nurture_plans 返回 Vec）
//   - POST /crm/leads/nurture-plans/{id}/execute → handlers/crm_handler
//     execute_nurture_plan（计划不存在→not_found，行级归属门）
//     → services/crm/lead::execute_nurture_plan（非 pending 拒，
//     跃迁写 status/executed_at/result）
// 入参（字段名逐字符对齐 CreateNurturePlanRequest，services/crm/lead）：
//   lead_id（必填，线索 id）、plan_name（必填，计划名，同时作本用例唯一标记）、
//   nurture_type（必填，培育类型 email/sms/visit/call 一类）、trigger_condition（可空触发条件）、
//   template_id（可空模板 id）、scheduled_at（可空计划执行时间，RFC3339）
//   执行端点无请求体（handler 只取 Path(id)）。
// 传给谁 / 存哪里：lead_nurture_plan 表（models/lead_nurture_plan，列 id/lead_id/plan_name/
//   nurture_type/trigger_condition/template_id/scheduled_at/executed_at/status/result/created_by/
//   created_at，status 词表 pending/executed/failed/cancelled）；执行只改该行三列，不动线索行。
//   成功信封 { code, message, data }；失败信封按 utils/error.rs：`business`（状态门）出
//   400 + BUSINESS_ERROR + 脱敏常量 err_msg::BUSINESS_PUBLIC="业务处理失败"
//   （utils/messages，文案含状态 token 故不外显），`not_found` 出 404 + NOT_FOUND。
//
// 清理与隔离事实（必读）：
//   lead_nurture_plan 没有 DELETE/PUT 端点（routes/crm.rs 该组只注册 GET/POST 与 POST execute），
//   本卡不为凑绿新增删除端点；用例自建线索与唯一 plan_name，所有回读只按自建 id 过滤，
//   残留计划行指向已清理的自建线索，不参与任何全局断言。
import { test, expect } from '../diagnose-fixture';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  genCode,
  tryCleanup,
} from '../flow/helpers';

/** lead_nurture_plan 行（models/lead_nurture_plan） */
interface NurturePlanRow {
  id: number;
  lead_id: number;
  plan_name: string;
  nurture_type: string;
  trigger_condition: string | null;
  template_id: string | null;
  scheduled_at: string | null;
  executed_at: string | null;
  status: string | null;
  result: string | null;
  created_by: number | null;
}

/** 培育计划状态词表（models/lead_nurture_plan.rs 注释与 services/crm/lead 的字面量） */
const PLAN_STATUS_PENDING = 'pending';
const PLAN_STATUS_EXECUTED = 'executed';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

async function selfUserId(page: import('@playwright/test').Page): Promise<number> {
  const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');
  expect(typeof me.id, `GET /auth/me 出参 id 应为数字，实际 ${JSON.stringify(me.id)}`).toBe(
    'number'
  );
  return me.id;
}

/** 建一条自建线索并登记清理 */
async function seedLead(page: import('@playwright/test').Page, marker: string): Promise<number> {
  const leadNo = genCode(marker);
  const created = await apiCall<{ id: number }>(page, 'POST', '/crm/leads', {
    lead_no: leadNo,
    lead_status: 'new',
    lead_source: 'WEBSITE',
    company_name: `E2E 培育线索 ${leadNo}`,
    contact_name: 'E2E 联系人',
    mobile_phone: '13800138005',
    priority: 'MEDIUM',
  });
  CLEANUP.push({ path: `/crm/leads/${created.data.id}`, label: 'crm_lead' });
  return created.data.id;
}

/** 建一条培育计划并登记（计划本身无删除端点，只登记其宿主线索） */
async function seedPlan(
  page: import('@playwright/test').Page,
  leadId: number,
  planName: string
): Promise<NurturePlanRow> {
  const created = await apiCall<NurturePlanRow>(page, 'POST', '/crm/leads/nurture-plans', {
    lead_id: leadId,
    plan_name: planName,
    nurture_type: 'email',
    trigger_condition: 'after_3_days_no_follow_up',
    template_id: 'TPL-E2E-001',
    scheduled_at: '2030-06-01T00:00:00Z',
  });
  return created.data;
}

/** 按 lead_id 读计划列表（data 裸数组） */
async function listPlansByLead(
  page: import('@playwright/test').Page,
  leadId: number
): Promise<NurturePlanRow[]> {
  const rows = await apiCallRaw<NurturePlanRow[]>(
    page,
    'GET',
    `/crm/leads/nurture-plans?lead_id=${leadId}`
  );
  expect(
    Array.isArray(rows),
    `GET /crm/leads/nurture-plans 出参应为数组，实际 ${JSON.stringify(rows)}`
  ).toBe(true);
  return rows;
}

test.describe('10 线索培育计划创建与执行契约', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('10-01 创建培育计划落 pending，执行时间与结果均为空并按线索回读', async ({ page }) => {
    const me = await selfUserId(page);
    const leadId = await seedLead(page, 'E2E-NP');
    const planName = genCode('E2E-PLAN');

    const created = await seedPlan(page, leadId, planName);

    expect(created.id, '计划应有主键').toBeGreaterThan(0);
    expect(created.lead_id, 'lead_id 应等于路径外的提交值').toBe(leadId);
    expect(created.plan_name).toBe(planName);
    expect(created.nurture_type, '培育类型应原样落库').toBe('email');
    expect(created.trigger_condition, '触发条件应原样落库').toBe('after_3_days_no_follow_up');
    expect(created.template_id, '模板 id 应原样落库').toBe('TPL-E2E-001');
    expect(
      typeof created.scheduled_at === 'string' && created.scheduled_at.startsWith('2030-06-01'),
      `scheduled_at 应存下提交的时间，实际 ${JSON.stringify(created.scheduled_at)}`
    ).toBe(true);
    expect(created.status, 'create_nurture_plan 固定写入 pending（lead.rs:1478）').toBe(
      PLAN_STATUS_PENDING
    );
    expect(created.executed_at, '新建计划不得有执行时间').toBeNull();
    expect(created.result, '新建计划不得有执行结果').toBeNull();
    expect(created.created_by, 'created_by 应落当前登录用户').toBe(me);

    const rows = await listPlansByLead(page, leadId);
    const readback = rows.find(row => row.id === created.id);
    expect(readback, '按 lead_id 回读应含该计划').toBeTruthy();
    expect(readback!.status).toBe(PLAN_STATUS_PENDING);
    expect(readback!.plan_name).toBe(planName);
    expect(readback!.executed_at).toBeNull();
    expect(readback!.result).toBeNull();

    // 状态过滤位：按 status=pending 也能命中，按已执行状态必须查不到
    const pendingOnly = await apiCallRaw<NurturePlanRow[]>(
      page,
      'GET',
      `/crm/leads/nurture-plans?lead_id=${leadId}&status=${PLAN_STATUS_PENDING}`
    );
    expect(
      pendingOnly.some(row => row.id === created.id),
      'pending 过滤应命中该计划'
    ).toBe(true);
    const executedOnly = await apiCallRaw<NurturePlanRow[]>(
      page,
      'GET',
      `/crm/leads/nurture-plans?lead_id=${leadId}&status=${PLAN_STATUS_EXECUTED}`
    );
    expect(
      executedOnly.filter(row => row.id === created.id),
      '未执行的计划不得出现在 executed 过滤里'
    ).toHaveLength(0);
  });

  test('10-02 执行培育计划一次性跃迁为 executed，写执行时间与固定结果', async ({ page }) => {
    const leadId = await seedLead(page, 'E2E-NP');
    const planName = genCode('E2E-PLAN');
    const created = await seedPlan(page, leadId, planName);

    const executed = await apiCall<NurturePlanRow>(
      page,
      'POST',
      `/crm/leads/nurture-plans/${created.id}/execute`
    );

    expect(executed.data.id, '执行的应是同一条计划').toBe(created.id);
    expect(executed.data.status, '跃迁后状态应为 executed').toBe(PLAN_STATUS_EXECUTED);
    expect(
      typeof executed.data.executed_at === 'string' && executed.data.executed_at.length > 0,
      `执行后应写 executed_at，实际 ${JSON.stringify(executed.data.executed_at)}`
    ).toBe(true);
    expect(executed.data.result, '结果文案由 lead.rs:1626 固定').toBe('执行成功');
    // 跃迁只动三列：其余列必须与创建时一致
    expect(executed.data.lead_id, '宿主线索不得被改动').toBe(created.lead_id);
    expect(executed.data.plan_name).toBe(planName);
    expect(executed.data.nurture_type).toBe(created.nurture_type);
    expect(executed.data.scheduled_at).toBe(created.scheduled_at);
    expect(executed.data.created_by, 'created_by 不得被执行动作改写').toBe(created.created_by);

    const rows = await listPlansByLead(page, leadId);
    const readback = rows.find(row => row.id === created.id);
    expect(readback, '回读应含该计划').toBeTruthy();
    expect(readback!.status, '回读状态应为 executed').toBe(PLAN_STATUS_EXECUTED);
    expect(readback!.result).toBe('执行成功');
    expect(readback!.executed_at).toBe(executed.data.executed_at);

    // 宿主线索状态不应因执行计划而变（执行不落线索侧）
    const leadAfter = await apiCallRaw<{ lead_status: string | null }>(
      page,
      'GET',
      `/crm/leads/${leadId}`
    );
    expect(leadAfter.lead_status, '执行培育计划不得改线索状态').toBe('new');
  });

  test('10-03 重复执行已执行计划被状态门拒 400 且三列零二次改写', async ({ page }) => {
    const leadId = await seedLead(page, 'E2E-NP');
    const planName = genCode('E2E-PLAN');
    const created = await seedPlan(page, leadId, planName);

    const first = await apiCall<NurturePlanRow>(
      page,
      'POST',
      `/crm/leads/nurture-plans/${created.id}/execute`
    );
    expect(first.data.status).toBe(PLAN_STATUS_EXECUTED);

    // services/crm/lead 的状态门用 AppError::business（文案含状态 token，
    // 按 utils/error.rs 安全边界保持脱敏）：只可断机器码与公开常量，不得断真实原因
    const fail = await apiCallExpectFail(
      page,
      'POST',
      `/crm/leads/nurture-plans/${created.id}/execute`
    );
    expect(fail.status, `重复执行应恰为 400，实际 ${fail.status}`).toBe(400);
    expect(failureCode(fail), `应 BUSINESS_ERROR，实际 code=${fail.code}`).toBe('BUSINESS_ERROR');
    expect(
      String(fail.message ?? ''),
      `business（非 displayable）出参必须脱敏，实际 message=${fail.message}`
    ).toBe('业务处理失败');

    const rows = await listPlansByLead(page, leadId);
    const readback = rows.find(row => row.id === created.id);
    expect(readback!.status, '被拒后状态应仍为 executed').toBe(PLAN_STATUS_EXECUTED);
    expect(readback!.executed_at, '被拒后 executed_at 不得被第二次请求覆盖（应为首次时间）').toBe(
      first.data.executed_at
    );
    expect(readback!.result, '结果文案不得被重复执行改写').toBe('执行成功');
  });

  test('10-04 执行不存在的培育计划返回 404 NOT_FOUND 且不产生任何计划行', async ({ page }) => {
    // 先有一条真实宿主线索，用它做"零写入"的读位：若执行请求意外落行，这里就能看见
    const leadId = await seedLead(page, 'E2E-NP404');

    const fail = await apiCallExpectFail(page, 'POST', '/crm/leads/nurture-plans/999999/execute');
    expect(fail.status, `不存在计划应恰为 404，实际 ${fail.status}`).toBe(404);
    expect(failureCode(fail), `应 NOT_FOUND，实际 code=${fail.code}`).toBe('NOT_FOUND');

    // handler 的 find_by_id 门在任何写之前（crm_handler）：
    // 该线索名下不得因为这个请求多出任何计划行
    const rows = await listPlansByLead(page, leadId);
    expect(rows, '自建线索名下应仍无任何计划行').toHaveLength(0);
  });
});
