// CRM 批量分配 / 商机跟进 / 增强客户入口 写口契约 E2E — crm/08
//
// 功能（被测业务规则，逐条对后端源码核实，非编造）：
//   1) 批量分配是"先判后写 + 逐行错误"的批次语义：批次内任一存在行未过跨 owner 写门即整笔 403
//      （原因不外显），不存在的 id 不整笔失败而是逐行计入 errors；返回体是给调用方对账用的
//      { total, assigned, failed, errors }，与提交的 lead_ids 长度同源。
//   2) 自动分配在触库之前有两道入参门：销售列表为空即拒，指定的销售全部不存在或已停用即拒；
//      两道门都在"取待分配线索"之前，因此被拒的请求不会改写任何线索行。
//   3) 商机跟进记录落 opportunity_follow_up 表，跟进人身份由会话注入（不由请求体伪造）。
//   4) POST /crm/customers/enhanced 落的是**线索域行**（crm_lead），与同前缀 PUT 落 customers 表
//      并不同域——本文件把这一域分裂当既成事实钉住（用 GET /crm/leads/{id} 回读该 id），
//      不改源码、不为本用例新增端点。
// 被测端点与注册点：
//   - POST /crm/assignments/batch  backend/src/routes/crm.rs `/assignments/batch`
//     → handlers/crm_assignment_handler::batch_assign（先判后写门，
//       逐行循环 run_batch_assign_loop，单行语义 assign_single_lead）
//   - POST /crm/assignments/auto-assign → handlers/crm_assignment_handler auto_assign
//     → services/crm/assign::auto_assign_leads（空列表门与无效销售门均在函数体开头）
//   - POST/GET /crm/opportunities/{id}/follow-ups
//     → handlers/crm_handler::create_opportunity_follow_up 与 crm_handler::list_opportunity_follow_ups
//       → services/crm/opp::create_opportunity_follow_up
//   - POST /crm/customers/enhanced → handlers/crm_customer_handler::create_customer
//     （入参 CreateLeadRequest，service 落 crm_lead::Model，写响应走线索侧字段权限门）
// 入参（字段名逐字符对齐 handler 的 Deserialize 结构）：
//   - 批量分配 BatchAssignRequest（crm_assignment_handler）：lead_ids: 线索 id 数组、
//     assignee_id: 目标销售用户 id、assignee_name: 目标销售展示名（自由文本，落分配历史 to_user_name）、
//     notes: 可空备注
//   - 自动分配 AutoAssignRequest（services/crm/assign）：assignee_user_ids: 参与轮询的销售
//     用户 id 数组（空即拒）、limit: 本次分配上限（缺省 100，钳到 1-1000）
//   - 商机跟进 CreateOpportunityFollowUpRequest（services/crm/opp）：follow_up_type（
//     phone/email/visit/meeting/wechat 一类方式串）、content（跟进内容，必填）、
//     follow_up_time（可选，缺省服务端取当前时间）、next_follow_up_date（可选，YYYY-MM-DD）
//   - 增强客户创建 CreateLeadRequest：lead_no、company_name、contact_name、mobile_phone、
//     lead_source、priority 等（与 POST /crm/leads 同一结构）
// 传给谁 / 存哪里：
//   crm_lead（批量分配改写 lead_status）、assignment_histories（批量分配落分配历史）、
//   opportunity_follow_up（models/opportunity_follow_up，列 id/opportunity_id/follow_up_type/
//   content/follow_up_time/next_follow_up_date/user_id/user_name/created_at）。
//   出参一律 { code, message, data } 成功信封；GET /crm/opportunities/{id}/follow-ups 的 data
//   是裸数组（list_opportunity_follow_ups 返回 Vec）。
//
// 本片的取舍与红线登记（不靠改测试迁就源码）：
//   - POST /crm/assignments/auto-assign 的**正向**不在本片写：services/crm/assign
//     fetch_pending_leads 取的是全库 `lead_status='new'` 按 id 升序的前 limit 条，
//     真实调用会把与本用例无关的他人待分配线索一并改 owner 与状态，且无法回滚，
//     属跨用例副作用；故只写"空销售列表"与"指定销售全部无效"两道零写入门。
//     这两条是业务态负例（在册方法、断具体机器码与外显文案，并回读哨兵线索证零落库），
//     不等价于该端点正向契约已被行使，覆盖面上须按此判读。
//   - 批量分配单行实现（crm_assignment_handler::assign_single_lead）只把 lead_status
//     置 assigned，**不改写 crm_lead.owner_id**（响应却按 assignee_id 记分配历史）。
//     本片按"lead_status 真的落库"这条可回归的契约断言，不把 owner_id 未变当成期望值钉死，
//     以免将来修复该缺陷时反向被测试锁住；缺陷本身交回研发处置。
//   - 分配历史读取端点的出参形状未在本片取证，故不断历史行，避免影子断言。
//   - PUT/POST /crm/recycle-rules 一族不在本片写：按派工只做取证，结论是——
//     handlers/missing_handlers update_recycle_rule 真实调用
//     services/crm/recycle_rule update_rule（find_by_id → not_found 门 → 逐列 if let Some
//     → update），GET/POST/PUT/DELETE 四方法均已在 frontend/scripts/route-snapshot.txt 在册，
//     不是不可达空壳；days 受 validator 的 1-365 取值域门约束（recycle_rule），
//     失败经 utils/error 的 validation_displayable 通道出 400 + VALIDATION_ERROR。
//   - 商机跟进记录没有 DELETE 端点（routes/crm.rs 该组只注册 GET/POST）：本卡严禁为凑绿
//     新增删除端点，改为用例自建商机与唯一 content 标记，所有读位只按自建 id 过滤，
//     残留行不影响任何断言。
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

/** batch_assign 出参（crm_assignment_handler 手写 json） */
interface BatchAssignResult {
  total: number;
  assigned: number;
  failed: number;
  errors: string[];
}

/** opportunity_follow_up 行（models/opportunity_follow_up） */
interface FollowUpRow {
  id: number;
  opportunity_id: number;
  follow_up_type: string;
  content: string;
  follow_up_time: string;
  next_follow_up_date: string | null;
  user_id: number;
  user_name: string;
  created_at: string | null;
}

/** 线索读出口用到的列 */
interface LeadRow {
  id: number;
  lead_no: string;
  company_name: string | null;
  lead_status: string | null;
  owner_id: number;
}

const LEAD_STATUS_NEW = 'new';
const LEAD_STATUS_ASSIGNED = 'assigned';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

async function selfUser(page: import('@playwright/test').Page): Promise<{ id: number }> {
  const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');
  expect(typeof me.id, `GET /auth/me 出参 id 应为数字，实际 ${JSON.stringify(me.id)}`).toBe(
    'number'
  );
  return { id: me.id };
}

/** 取一个非自身的活跃用户 id 作分配目标（GET /users 仅 admin，本 worker 即 admin 会话） */
async function pickOtherUserId(
  page: import('@playwright/test').Page,
  selfId: number
): Promise<number> {
  const res = await apiCallRaw<{ users: Array<{ id: number; is_active: boolean }> }>(
    page,
    'GET',
    '/users?page=1&page_size=100'
  );
  expect(Array.isArray(res.users), 'GET /users 响应缺 data.users 数组').toBe(true);
  const candidate = res.users.find(u => u.id !== selfId && u.is_active);
  if (!candidate) throw new Error('库内没有可用作分配目标的活跃用户');
  return candidate.id;
}

/** 建一条 new 线索（owner 即当前用户）并登记清理，返回 id */
async function seedNewLead(page: import('@playwright/test').Page, marker: string): Promise<number> {
  const leadNo = genCode(marker);
  const created = await apiCall<{ id: number }>(page, 'POST', '/crm/leads', {
    lead_no: leadNo,
    lead_status: LEAD_STATUS_NEW,
    lead_source: 'WEBSITE',
    company_name: `E2E 分配线索 ${leadNo}`,
    contact_name: 'E2E 联系人',
    mobile_phone: '13800138002',
    priority: 'MEDIUM',
  });
  CLEANUP.push({ path: `/crm/leads/${created.data.id}`, label: 'crm_lead' });
  return created.data.id;
}

async function readLead(page: import('@playwright/test').Page, leadId: number): Promise<LeadRow> {
  return apiCallRaw<LeadRow>(page, 'GET', `/crm/leads/${leadId}`);
}

/** 建一条商机（挂到自建客户上）并登记清理，返回 id */
async function seedOpportunity(
  page: import('@playwright/test').Page,
  marker: string
): Promise<number> {
  const customer = await apiCall<{ id: number }>(page, 'POST', '/crm/customers', {
    customer_name: `E2E 跟进宿主客户 ${marker}`,
  });
  CLEANUP.push({ path: `/crm/customers/${customer.data.id}`, label: 'crm_customer' });
  const oppNo = genCode('E2E-OPP-FU');
  const created = await apiCall<{ id: number }>(page, 'POST', '/crm/opportunities', {
    opportunity_no: oppNo,
    opportunity_name: oppNo,
    customer_id: customer.data.id,
    opportunity_stage: 'QUALIFICATION',
    estimated_amount: 30000,
    win_probability: 60,
    expected_close_date: '2026-12-31',
  });
  CLEANUP.push({ path: `/crm/opportunities/${created.data.id}`, label: 'crm_opportunity' });
  return created.data.id;
}

test.describe('08 批量分配 / 商机跟进 / 增强客户入口契约', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('08-01 批量分配两条本人线索，逐行 assigned 且 lead_status 真实落库', async ({ page }) => {
    const me = await selfUser(page);
    const assigneeId = await pickOtherUserId(page, me.id);
    const leadA = await seedNewLead(page, 'E2E-BAT');
    const leadB = await seedNewLead(page, 'E2E-BAT');

    const res = await apiCall<BatchAssignResult>(page, 'POST', '/crm/assignments/batch', {
      lead_ids: [leadA, leadB],
      assignee_id: assigneeId,
      assignee_name: 'E2E 目标销售',
      notes: 'E2E 批量分配备注',
    });

    expect(res.data.total, 'total 应等于提交的 lead_ids 长度').toBe(2);
    expect(res.data.assigned, '两条本人行应全部成功').toBe(2);
    expect(res.data.failed, '全成功时 failed 应为 0').toBe(0);
    expect(res.data.errors, '全成功时 errors 应为空数组').toEqual([]);

    // 落库回读：批量分配必须真的改写 lead_status，而不是只在响应里报成功
    const afterA = await readLead(page, leadA);
    const afterB = await readLead(page, leadB);
    expect(
      afterA.lead_status,
      `线索 ${leadA} 的 lead_status 应落 ${LEAD_STATUS_ASSIGNED}，实际 ${afterA.lead_status}`
    ).toBe(LEAD_STATUS_ASSIGNED);
    expect(
      afterB.lead_status,
      `线索 ${leadB} 的 lead_status 应落 ${LEAD_STATUS_ASSIGNED}，实际 ${afterB.lead_status}`
    ).toBe(LEAD_STATUS_ASSIGNED);
    // 两条应是不同的行（自建 id 各自独立，防止批次把同一条重复计数）
    expect(afterA.id).not.toBe(afterB.id);
  });

  test('08-02 批量分配混入不存在 id，整笔不失败但逐行计入 errors', async ({ page }) => {
    const me = await selfUser(page);
    const assigneeId = await pickOtherUserId(page, me.id);
    const leadA = await seedNewLead(page, 'E2E-BAT');

    const res = await apiCall<BatchAssignResult>(page, 'POST', '/crm/assignments/batch', {
      lead_ids: [leadA, 999_999],
      assignee_id: assigneeId,
      assignee_name: 'E2E 目标销售',
      notes: null,
    });

    expect(res.data.total, 'total 按提交长度计').toBe(2);
    expect(res.data.assigned, '存在的一行应成功').toBe(1);
    expect(res.data.failed, '不存在的一行应计 failed').toBe(1);
    expect(res.data.errors, 'errors 应与 failed 同数').toHaveLength(1);
    expect(
      typeof res.data.errors[0] === 'string' && res.data.errors[0].length > 0,
      `逐行失败必须给出可读原因，实际 ${JSON.stringify(res.data.errors)}`
    ).toBe(true);
    expect(res.data.assigned + res.data.failed, '成功数加失败数应等于 total').toBe(res.data.total);

    const after = await readLead(page, leadA);
    expect(after.lead_status, '存在的那行仍应被真实分配').toBe(LEAD_STATUS_ASSIGNED);
  });

  test('08-03 批量分配空 lead_ids 批次返回全零且不报错', async ({ page }) => {
    const res = await apiCall<BatchAssignResult>(page, 'POST', '/crm/assignments/batch', {
      lead_ids: [],
      assignee_id: 1,
      assignee_name: 'E2E 空批次',
      notes: null,
    });

    // load_lead_map 对空 ids 直接返回空 map（crm_assignment_handler），
    // 循环不执行 ⇒ 四值全零；把它当 500 或"默认全量"都是回归
    expect(res.data.total).toBe(0);
    expect(res.data.assigned).toBe(0);
    expect(res.data.failed).toBe(0);
    expect(res.data.errors).toEqual([]);
  });

  test('08-04 商机跟进记录落库，跟进人身份由会话注入且日期原样存取', async ({ page }) => {
    const me = await selfUser(page);
    const opportunityId = await seedOpportunity(page, genCode('E2E-FU'));
    const content = `E2E 跟进内容 ${genCode('FU')}`;

    const created = await apiCall<FollowUpRow>(
      page,
      'POST',
      `/crm/opportunities/${opportunityId}/follow-ups`,
      {
        follow_up_type: 'phone',
        content,
        next_follow_up_date: '2026-11-15',
      }
    );

    expect(created.data.id, '跟进记录应有主键').toBeGreaterThan(0);
    expect(created.data.opportunity_id, '路径指定的商机 id 应写入行').toBe(opportunityId);
    expect(created.data.follow_up_type).toBe('phone');
    expect(created.data.content, 'content 应原样落库').toBe(content);
    expect(created.data.user_id, `user_id 应取会话用户 ${me.id} 而非请求体伪造值`).toBe(me.id);
    expect(
      typeof created.data.user_name === 'string' && created.data.user_name.length > 0,
      `user_name 应由会话注入非空，实际 ${JSON.stringify(created.data.user_name)}`
    ).toBe(true);
    expect(typeof created.data.follow_up_time, '未提交时间时服务端补当前时间').toBe('string');
    expect(
      created.data.next_follow_up_date,
      `next_follow_up_date 应按 YYYY-MM-DD 原样存取，实际 ${created.data.next_follow_up_date}`
    ).toBe('2026-11-15');

    // 回读：GET 列表按自建 id 过滤，证真落库
    const rows = await apiCallRaw<FollowUpRow[]>(
      page,
      'GET',
      `/crm/opportunities/${opportunityId}/follow-ups`
    );
    expect(Array.isArray(rows), 'GET 跟进列表出参应为数组').toBe(true);
    const row = rows.find(item => item.id === created.data.id);
    expect(row, '跟进列表应含刚建的记录').toBeTruthy();
    expect(row!.content).toBe(content);
    expect(row!.follow_up_type).toBe('phone');
    expect(row!.next_follow_up_date).toBe('2026-11-15');
  });

  test('08-05 自动分配空销售列表被拒 400 外显原因，待分配线索零改动', async ({ page }) => {
    const sentinelLead = await seedNewLead(page, 'E2E-AUTO');

    const fail = await apiCallExpectFail(page, 'POST', '/crm/assignments/auto-assign', {
      assignee_user_ids: [],
      limit: 1,
    });
    expect(fail.status, `空销售列表应恰为 400，实际 ${fail.status}`).toBe(400);
    expect(failureCode(fail), `应 VALIDATION_ERROR，实际 code=${fail.code}`).toBe(
      'VALIDATION_ERROR'
    );
    expect(
      String(fail.message ?? ''),
      `auto_assign_leads:111 用 validation_displayable，须外显真实原因，实际 message=${fail.message}`
    ).toBe('自动分配失败：参与轮询的销售用户列表不能为空');

    // 门在取待分配行之前：哨兵线索必须仍是 new
    const after = await readLead(page, sentinelLead);
    expect(after.lead_status, `门拒后线索应仍为 ${LEAD_STATUS_NEW}`).toBe(LEAD_STATUS_NEW);
  });

  test('08-06 自动分配指定销售全部无效被拒 400 外显原因，待分配线索零改动', async ({ page }) => {
    const sentinelLead = await seedNewLead(page, 'E2E-AUTO');

    const fail = await apiCallExpectFail(page, 'POST', '/crm/assignments/auto-assign', {
      assignee_user_ids: [999_998, 999_999],
      limit: 1,
    });
    expect(fail.status, `全无效销售应恰为 400，实际 ${fail.status}`).toBe(400);
    expect(failureCode(fail)).toBe('VALIDATION_ERROR');
    expect(
      String(fail.message ?? ''),
      `assign.rs:121 的"均不存在或已停用"门须外显，实际 message=${fail.message}`
    ).toBe('自动分配失败：指定的销售用户均不存在或已停用');

    const after = await readLead(page, sentinelLead);
    expect(after.lead_status, `门拒后哨兵线索不应被抢走，实际 ${after.lead_status}`).toBe(
      LEAD_STATUS_NEW
    );
  });

  test('08-07 增强客户创建入口落线索域行，GET 线索详情回读同值', async ({ page }) => {
    const leadNo = genCode('E2E-ENH');
    const companyName = `E2E 增强入口客户 ${leadNo}`;

    const created = await apiCall<{ id: number; lead_no: string; company_name: string | null }>(
      page,
      'POST',
      '/crm/customers/enhanced',
      {
        lead_no: leadNo,
        lead_status: LEAD_STATUS_NEW,
        lead_source: 'WEBSITE',
        company_name: companyName,
        contact_name: 'E2E 联系人',
        mobile_phone: '13800138003',
        priority: 'MEDIUM',
      }
    );
    CLEANUP.push({ path: `/crm/leads/${created.data.id}`, label: 'crm_lead_via_enhanced' });
    expect(created.data.id, '增强入口创建应返回主键').toBeGreaterThan(0);
    expect(created.data.lead_no, '出参应带线索域列 lead_no').toBe(leadNo);
    expect(created.data.company_name, '出参 company_name 应等于提交值').toBe(companyName);

    // 域分裂事实钉桩：本端点落的是 crm_lead（crm_customer_handler create_lead），
    // 因此用线索详情端点回读同一个 id 必须能读到同一行
    const asLead = await apiCallRaw<LeadRow>(page, 'GET', `/crm/leads/${created.data.id}`);
    expect(asLead.id).toBe(created.data.id);
    expect(asLead.lead_no).toBe(leadNo);
    expect(asLead.company_name).toBe(companyName);
    expect(asLead.lead_status, '缺省状态应落 new').toBe(LEAD_STATUS_NEW);
  });
});
