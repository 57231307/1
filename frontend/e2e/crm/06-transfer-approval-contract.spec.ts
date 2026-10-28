// CRM 客户转移审批流 E2E 契约用例 — crm/06
//
// 功能（被测业务规则，逐条对后端源码核实，非编造）：
//   客户转移审批是「申请 → 销售经理审批 →（大客户才有的）总监审批」的多级流，
//   申请落库即 pending / current_level=1，审批动作只允许在 pending 且层级匹配的单据上发生，
//   拒绝与取消都不执行实际转移（线索 owner_id 必须保持原值）。
// 被测端点与注册点（backend/src/routes/crm.rs:293-319 `crm_transfer_approvals()`，
//   nest 前缀 /api/v1/erp/crm）：
//   - POST   /crm/transfer-approvals                handler customer_transfer_approval_handler::create_approval
//   - GET    /crm/transfer-approvals/{id}           customer_transfer_approval_handler::get_approval
//   - POST   /crm/transfer-approvals/{id}/cancel    customer_transfer_approval_handler::cancel_approval
//   - POST   /crm/transfer-approvals/{id}/manager-approve  customer_transfer_approval_handler::manager_approve
//   规则实现：backend/src/services/crm/customer_transfer_approval_service.rs
//   （create_approval:159 / validate_create_request:208 / fetch_and_validate_lead:218 /
//     ensure_no_pending_approval:244 / build_approval_active:266 / manager_approve:304 /
//     cancel_approval:458 / get_pending_approval:603）
// 入参（请求体字段，逐字符对齐 handler 的 CreateTransferApprovalRequest / ApproveRequest）：
//   - 创建：lead_id（必填，线索 id）、to_user_id（必填，新归属人用户 id）、reason（必填，
//     申请原因；`reason.trim().is_empty()` 即拒，故纯空白同样非法）
//   - 审批：approval_id、comment、approved；approval_id 被路径参数覆盖
//     （handler:114 与 handler:144 `req.approval_id = id;`），故用例故意在体内提交一个不存在的
//     approval_id 来钉这条覆盖语义：若覆盖失效，请求会打到不存在的单据上并 404 判红。
// 传给谁 / 存哪里：落库表 customer_transfer_approvals
//   （backend/migration/src/domain/v15/mod.rs:2666 建表），审批通过的普通客户分支会再调
//   CrmAssignService::transfer_lead 改 crm_lead.owner_id；本文件所有用例只走「拒绝 / 取消」
//   分支，不触发转移，因此不留归属漂移。
// 出参形状：成功信封 { code, message, data }，data 即 TransferApprovalDto（service:82-105）；
//   失败信封 { code, message, trace_id, timestamp }，机器码与文案口径按 utils/error.rs：
//   `validation_displayable`→400+VALIDATION_ERROR+原文外显，
//   `business_displayable`→400+BUSINESS_ERROR+原文外显。
//
// 清理与隔离事实（必读）：
//   customer_transfer_approvals 没有 DELETE 端点（routes/crm.rs 该组只注册 GET/POST），
//   本卡严禁为凑绿向后端新增删除端点，因此改为「唯一标记 + 过滤隔离」：
//   - 每条审批单只按用例自建的 approval id 与 lead id 读写，绝不用列表页做全量断言；
//   - 列表位断言一律带 applicant_id=本 worker 用户 + status=pending，并用 total 的
//     前后相等证「零写入」，items 再按自建 lead_id 过滤；
//   - 用例自建的线索用 DELETE /crm/leads/{id} 清理；该表 lead_id 列在 DDL 里没有外键约束
//     （v15/mod.rs:2668 仅 INTEGER NOT NULL），故删线索不会被审批单挡住，留下的审批单行
//     指向已删除的 lead_id（这一条是既存的引用完整性缺口，本卡只登记不修）。
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

/** TransferApprovalDto 的出参字段子集（service:82-105，字段名逐字符对齐 Rust 侧） */
interface ApprovalRow {
  id: number;
  approval_no: string;
  lead_id: number;
  company_name: string | null;
  from_user_id: number;
  from_user_name: string | null;
  to_user_id: number;
  to_user_name: string | null;
  applicant_id: number;
  reason: string;
  is_large_customer: boolean;
  approval_status: string;
  current_level: number;
  max_level: number;
  manager_approver_id: number | null;
  manager_comment: string | null;
  manager_approved_at: string | null;
  director_approver_id: number | null;
  director_comment: string | null;
  director_approved_at: string | null;
  completed_at: string | null;
}

/** list_approvals 出参（handler:62-65 手写 { items, total }，非 PaginatedResponse） */
interface ApprovalList {
  items: ApprovalRow[];
  total: number;
}

/** 审批状态词表：models/customer_transfer_approval.rs:15-21 的字面量，逐字符照抄 */
const APPROVAL_STATUS_PENDING = 'pending';
const APPROVAL_STATUS_REJECTED = 'rejected';
const APPROVAL_STATUS_CANCELLED = 'cancelled';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** 取本 worker 登录用户 id（GET /auth/me，handlers/auth_handler_misc.rs get_current_user） */
async function selfUserId(page: import('@playwright/test').Page): Promise<number> {
  const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');
  expect(typeof me.id, `GET /auth/me 出参 id 应为数字，实际=${JSON.stringify(me.id)}`).toBe(
    'number'
  );
  return me.id;
}

/** 取一个非自身的活跃用户 id 作转移目标（GET /users 仅 admin 可用，本 worker 即 admin 会话） */
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
  if (!candidate) throw new Error('库内没有可用作转移目标的活跃用户');
  return candidate.id;
}

/**
 * 建一条 new 状态线索（owner 即当前登录用户，同 crm/05 已证的 owner 落库口径），
 * 并登记清理。lead_no 由本函数生成并回给调用方做唯一标记。
 */
async function seedLead(
  page: import('@playwright/test').Page,
  marker: string
): Promise<{ id: number; leadNo: string }> {
  const leadNo = genCode(marker);
  const created = await apiCall<{ id: number }>(page, 'POST', '/crm/leads', {
    lead_no: leadNo,
    lead_status: 'new',
    lead_source: 'WEBSITE',
    company_name: `E2E 转移审批线索 ${leadNo}`,
    contact_name: 'E2E 联系人',
    mobile_phone: '13800138000',
    priority: 'MEDIUM',
  });
  CLEANUP.push({ path: `/crm/leads/${created.data.id}`, label: 'crm_lead' });
  return { id: created.data.id, leadNo };
}

/** 按「本 worker + pending」拉审批列表（唯一标记隔离用的读位） */
async function listMyPending(
  page: import('@playwright/test').Page,
  selfId: number
): Promise<ApprovalList> {
  return apiCallRaw<ApprovalList>(
    page,
    'GET',
    `/crm/transfer-approvals?page=1&page_size=100&status=${APPROVAL_STATUS_PENDING}&applicant_id=${selfId}`
  );
}

/** 建一条指向该线索的转移审批申请（申请原因带唯一标记），并登记「清不掉」的事实说明 */
async function seedApproval(
  page: import('@playwright/test').Page,
  leadId: number,
  toUserId: number,
  reason: string
): Promise<ApprovalRow> {
  const created = await apiCall<ApprovalRow>(page, 'POST', '/crm/transfer-approvals', {
    lead_id: leadId,
    to_user_id: toUserId,
    reason,
  });
  return created.data;
}

test.describe('06 客户转移审批流契约', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('06-01 创建转移审批落库 pending/一级，GET 详情逐字段回读一致', async ({ page }) => {
    const me = await selfUserId(page);
    const other = await pickOtherUserId(page, me);
    const { id: leadId, leadNo } = await seedLead(page, 'E2E-TA');
    const reason = `E2E 转移申请原因 ${leadNo}`;

    const created = await seedApproval(page, leadId, other, reason);

    // 单据号由 DocumentNumberGenerator.generate_no(prefix="TA") 产出（service:172-178），
    // 形态为 TA+YYYYMMDD+3 位流水，用例只钉「以 TA 开头 + 非空」，不钉当日日期避免跨零点抖动
    expect(
      created.approval_no.startsWith('TA'),
      `审批单号应以 TA 前缀起头，实际=${created.approval_no}`
    ).toBe(true);
    expect(created.approval_status, '新建审批单必须落 pending').toBe(APPROVAL_STATUS_PENDING);
    expect(created.current_level, '新建审批单起始层级应为 1').toBe(1);
    expect(created.max_level, '未转化线索 converted_customer_id 为空 ⇒ 恒为一级流').toBe(1);
    expect(created.is_large_customer, '普通线索不应判为大客户').toBe(false);
    expect(created.lead_id, 'lead_id 应等于提交的线索 id').toBe(leadId);
    expect(created.applicant_id, 'applicant_id 应落当前登录用户').toBe(me);
    expect(created.from_user_id, 'from_user_id 应取线索当前归属人').toBe(me);
    expect(created.to_user_id, 'to_user_id 应等于提交的新归属人').toBe(other);
    expect(created.reason, 'reason 应原样落库，不做 trim 之外的加工').toBe(reason);
    // 审批链字段在创建时全部为空（build_approval_active:290-296）
    expect(created.manager_approver_id).toBeNull();
    expect(created.manager_comment).toBeNull();
    expect(created.manager_approved_at).toBeNull();
    expect(created.director_approver_id).toBeNull();
    expect(created.completed_at).toBeNull();

    // 落库回读：详情端点独立查库，不得只信写响应
    const readback = await apiCallRaw<ApprovalRow>(
      page,
      'GET',
      `/crm/transfer-approvals/${created.id}`
    );
    expect(readback.approval_no, '回读单号应与创建返回一致').toBe(created.approval_no);
    expect(readback.approval_status).toBe(APPROVAL_STATUS_PENDING);
    expect(readback.current_level).toBe(1);
    expect(readback.max_level).toBe(1);
    expect(readback.lead_id).toBe(leadId);
    expect(readback.applicant_id).toBe(me);
    expect(readback.to_user_id).toBe(other);
    expect(readback.reason).toBe(reason);
    expect(readback.manager_approver_id).toBeNull();
    expect(readback.completed_at).toBeNull();
  });

  test('06-02 申请人取消 pending 审批，回读 cancelled 且线索归属未被转移', async ({ page }) => {
    const me = await selfUserId(page);
    const other = await pickOtherUserId(page, me);
    const { id: leadId, leadNo } = await seedLead(page, 'E2E-TA');

    const created = await seedApproval(page, leadId, other, `E2E 取消用例原因 ${leadNo}`);
    const cancelled = await apiCall<ApprovalRow>(
      page,
      'POST',
      `/crm/transfer-approvals/${created.id}/cancel`
    );

    expect(cancelled.message, '取消成功文案由 handler:99-102 固定').toBe('审批单已取消');
    expect(cancelled.data.approval_status, '取消后应落终态 cancelled').toBe(
      APPROVAL_STATUS_CANCELLED
    );
    // cancel_approval 只改 approval_status 与 updated_at（service:479-481）：
    // 审批人与 completed_at 必须仍是空，不得被取消动作冒充成「已审批完成」
    expect(cancelled.data.manager_approver_id).toBeNull();
    expect(cancelled.data.completed_at).toBeNull();

    const readback = await apiCallRaw<ApprovalRow>(
      page,
      'GET',
      `/crm/transfer-approvals/${created.id}`
    );
    expect(readback.approval_status, '回读应同为 cancelled').toBe(APPROVAL_STATUS_CANCELLED);
    expect(readback.approval_no).toBe(created.approval_no);

    // 取消不执行转移：线索 owner 仍为申请人
    const leadAfter = await apiCallRaw<{ owner_id: number; lead_status: string | null }>(
      page,
      'GET',
      `/crm/leads/${created.lead_id}`
    );
    expect(leadAfter.owner_id, `取消后线索 owner 应仍为 ${me}，实际 ${leadAfter.owner_id}`).toBe(
      me
    );
    expect(leadAfter.lead_status, '取消不应改动线索状态').toBe('new');
  });

  test('06-03 经理拒绝审批落 rejected，路径参数覆盖体内 approval_id 且线索未转移', async ({
    page,
  }) => {
    const me = await selfUserId(page);
    const other = await pickOtherUserId(page, me);
    const { id: leadId, leadNo } = await seedLead(page, 'E2E-TA');

    const created = await seedApproval(page, leadId, other, `E2E 拒绝用例原因 ${leadNo}`);
    const comment = `E2E 经理拒绝意见 ${leadNo}`;
    // approval_id 故意填一个不存在的值：handler:114 用路径参数覆盖，
    // 一旦覆盖失效本请求就会命中不存在的单据（404），本用例即判红
    const rejected = await apiCall<ApprovalRow>(
      page,
      'POST',
      `/crm/transfer-approvals/${created.id}/manager-approve`,
      { approval_id: 999_999, comment, approved: false }
    );

    expect(rejected.message, '经理审批完成文案由 handler:129-132 固定').toBe('经理审批完成');
    expect(rejected.data.id, '被审批的应是路径指定的那张单').toBe(created.id);
    expect(rejected.data.approval_status, 'approved=false 应落 rejected').toBe(
      APPROVAL_STATUS_REJECTED
    );
    expect(rejected.data.manager_approver_id, '经理审批人应落当前登录用户').toBe(me);
    expect(rejected.data.manager_comment, '拒绝意见应原样落库').toBe(comment);
    expect(
      typeof rejected.data.manager_approved_at,
      `manager_approved_at 应写入时间戳，实际 ${JSON.stringify(rejected.data.manager_approved_at)}`
    ).toBe('string');
    // 拒绝分支不写 completed_at、不进总监层（service:371-383）
    expect(rejected.data.completed_at).toBeNull();
    expect(rejected.data.current_level).toBe(1);
    expect(rejected.data.director_approver_id).toBeNull();

    const readback = await apiCallRaw<ApprovalRow>(
      page,
      'GET',
      `/crm/transfer-approvals/${created.id}`
    );
    expect(readback.approval_status, '回读应同为 rejected').toBe(APPROVAL_STATUS_REJECTED);
    expect(readback.manager_comment).toBe(comment);

    const leadAfter = await apiCallRaw<{ owner_id: number }>(page, 'GET', `/crm/leads/${leadId}`);
    expect(
      leadAfter.owner_id,
      `拒绝后线索 owner 应仍为 ${me}（拒绝不得执行转移），实际 ${leadAfter.owner_id}`
    ).toBe(me);
  });

  test('06-04 空白申请原因被拒 400 VALIDATION_ERROR 外显原文，且零审批单落库', async ({ page }) => {
    const me = await selfUserId(page);
    const other = await pickOtherUserId(page, me);
    const { id: leadId } = await seedLead(page, 'E2E-TA');

    const before = await listMyPending(page, me);

    const fail = await apiCallExpectFail(page, 'POST', '/crm/transfer-approvals', {
      lead_id: leadId,
      to_user_id: other,
      reason: '   ',
    });
    expect(fail.status, `纯空白原因应恰为 400，实际 ${fail.status}`).toBe(400);
    expect(failureCode(fail), `应 VALIDATION_ERROR，实际 code=${fail.code}`).toBe(
      'VALIDATION_ERROR'
    );
    expect(
      String(fail.message ?? ''),
      `validate_create_request 用 validation_displayable，出参必须外显真实原因，实际 message=${fail.message}`
    ).toBe('转移审批申请失败：申请原因不能为空');

    // 零写入：可见集 total 前后相等，且按自建 lead_id 过滤必须查不到任何单
    const after = await listMyPending(page, me);
    expect(after.total, `非法申请应零落库，pending total ${before.total}→${after.total}`).toBe(
      before.total
    );
    expect(
      after.items.filter(row => row.lead_id === leadId),
      '非法申请不得为该线索留下任何 pending 审批单'
    ).toHaveLength(0);
  });

  test('06-05 同线索重复 pending 申请被拒 400 BUSINESS_ERROR，原单保持 pending', async ({
    page,
  }) => {
    const me = await selfUserId(page);
    const other = await pickOtherUserId(page, me);
    const { id: leadId, leadNo } = await seedLead(page, 'E2E-TA');

    const first = await seedApproval(page, leadId, other, `E2E 重复申请首单 ${leadNo}`);
    expect(first.approval_status).toBe(APPROVAL_STATUS_PENDING);

    const fail = await apiCallExpectFail(page, 'POST', '/crm/transfer-approvals', {
      lead_id: leadId,
      to_user_id: other,
      reason: `E2E 重复申请次单 ${leadNo}`,
    });
    expect(fail.status, `重复 pending 申请应恰为 400，实际 ${fail.status}`).toBe(400);
    expect(failureCode(fail), `应 BUSINESS_ERROR，实际 code=${fail.code}`).toBe('BUSINESS_ERROR');
    expect(
      String(fail.message ?? ''),
      `ensure_no_pending_approval 用 business_displayable，出参必须外显真实原因，实际 message=${fail.message}`
    ).toBe('转移审批申请失败：该线索已存在待审批的转移申请');

    // 该线索名下仍应只有一张 pending 单，且就是首单（重复请求不得顶替或追加）
    const after = await listMyPending(page, me);
    const rowsForLead = after.items.filter(row => row.lead_id === leadId);
    expect(rowsForLead, '同一线索只能保留一张待审批单').toHaveLength(1);
    expect(rowsForLead[0].approval_no, '留下的应是首单单号').toBe(first.approval_no);
    expect(rowsForLead[0].approval_status, '首单状态不得被重复请求改动').toBe(
      APPROVAL_STATUS_PENDING
    );
  });
});
