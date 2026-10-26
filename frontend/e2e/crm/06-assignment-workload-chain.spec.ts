// CRM 分配负载查询契约 — 06 workload 口零漂移回读与参数逐项校验负例
// 契约真相（逐段读后端源，非推测）：
//   GET crm/assignments/workload（handler crm_assignment_handler.rs list_workload，
//   service services/crm/assign.rs list_assignee_workload）查询参数 user_ids 为逗号分隔
//   整数字符串，任一元素非法即整体拒绝，不再静默丢弃非法项；载荷为空串报
//   validation_displayable「user_ids 参数不能为空」，列表内空项报「第 N 项为空」，
//   非法整数报「第 N 项「x」不是合法整数」，三类均 400 VALIDATION_ERROR 且文案外显。
//   出参为 AssigneeDistribution 数组，键 user_id、username、assigned_count；
//   仅 is_active 的用户入列，assigned_count 统计该 owner 名下 lead_status 非空且
//   非 CONVERTED 非 LOST 的活跃线索数（services/crm/lead.rs 建线 owner 恒为会话用户）。
// 正向零漂移：以本会话用户（auth/me id）为查询对象，基线数建线后加一，删线后归位。
// 假绿防线：回读用真实计数差值断言，不写恒真 is-numeric；负例钉 HTTP 码与机器码
// failureCode 并读 validation_displayable 外显文案（validation 族按 utils/error.rs
// 安全边界外显真实原因），拒绝把 400 当任意健康。
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

interface WorkloadRow {
  user_id: number;
  username: string;
  assigned_count: number;
}

const CLEANUP: Array<{ path: string; label: string }> = [];

test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** 取本会话用户 id 与用户名（auth/me 出参 UserInfo{id,username,...}，handler auth_handler_misc.rs get_current_user） */
async function me(
  page: import('@playwright/test').Page
): Promise<{ id: number; username: string }> {
  const info = await apiCallRaw<{ id: number; username: string; permissions: string[] }>(
    page,
    'GET',
    '/auth/me'
  );
  expect(typeof info.id, `auth/me 应返回数字用户 id，实际 ${JSON.stringify(info)}`).toBe('number');
  return { id: info.id, username: info.username };
}

/** workload 口按单个用户查询并取该用户行（行缺失即红，active 用户必入列） */
async function workloadCount(
  page: import('@playwright/test').Page,
  userId: number
): Promise<number> {
  const rows = await apiCallRaw<WorkloadRow[]>(
    page,
    'GET',
    `/crm/assignments/workload?user_ids=${userId}`
  );
  expect(Array.isArray(rows), `workload 应返回数组，实际 ${JSON.stringify(rows)}`).toBe(true);
  const row = rows.find(r => Number(r.user_id) === userId);
  expect(
    row,
    `user_ids=${userId} 应命中该 active 用户行，实际 ${JSON.stringify(rows)}`
  ).toBeDefined();
  return Number(row!.assigned_count);
}

/** 建一条 new 状态线索（owner 由服务端写会话用户，services/crm/lead.rs:62），交 CLEANUP 兜底删除 */
async function seedActiveLead(page: import('@playwright/test').Page): Promise<number> {
  const leadNo = genCode('E2E-WL');
  const created = await apiCall<{ id?: number }>(page, 'POST', '/crm/leads', {
    lead_no: leadNo,
    lead_status: 'new',
    lead_source: 'WEBSITE',
    company_name: `E2E 负载测试 ${leadNo}`,
    contact_name: '测试联系人',
    mobile_phone: '13800138001',
    priority: 'MEDIUM',
  });
  if (!created.data?.id) throw new Error(`建线索失败：${JSON.stringify(created)}`);
  CLEANUP.push({ path: `/crm/leads/${created.data.id}`, label: 'crm_lead' });
  return created.data.id;
}

test.describe('06 分配负载 workload 契约', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('06-01 建线与删线使本用户 assigned_count 加一归位（零漂移回读）', async ({ page }) => {
    const { id: userId } = await me(page);

    const before = await workloadCount(page, userId);

    const leadId = await seedActiveLead(page);
    const afterCreate = await workloadCount(page, userId);
    expect(
      afterCreate,
      `建一条 new 线索后 assigned_count 应为 ${before}+1=${before + 1}，实际 ${afterCreate}`
    ).toBe(before + 1);

    await apiCall(page, 'DELETE', `/crm/leads/${leadId}`);
    const afterDelete = await workloadCount(page, userId);
    expect(afterDelete, `删除线索后应回到基线 ${before}（零漂移），实际 ${afterDelete}`).toBe(
      before
    );
  });

  test('06-02 列表含非法整数项整体拒绝，400 VALIDATION_ERROR 外显第几项', async ({ page }) => {
    // handler 显式注释：原实现静默丢弃非法项导致调用方误以为覆盖全部 ID，
    // 现逐项校验，第 2 项 abc 非法即整体 400，文案点名项次与原文。
    const fail = await apiCallExpectFail(page, 'GET', '/crm/assignments/workload?user_ids=1,abc');
    expect(fail.status, `非法整数项应 400，实际 ${fail.status}`).toBe(400);
    expect(failureCode(fail), `应 VALIDATION_ERROR，实际 code=${fail.code}`).toBe(
      'VALIDATION_ERROR'
    );
    expect(
      String(fail.message ?? ''),
      `validation_displayable 必须外显真实原因（含项次与非法原文 abc），实际 message=${fail.message}`
    ).toContain('abc');
  });

  test('06-03 user_ids 空串拒绝，400 并点名参数不能为空', async ({ page }) => {
    const fail = await apiCallExpectFail(page, 'GET', '/crm/assignments/workload?user_ids=');
    expect(fail.status, `空 user_ids 应 400，实际 ${fail.status}`).toBe(400);
    expect(failureCode(fail), `应 VALIDATION_ERROR，实际 code=${fail.code}`).toBe(
      'VALIDATION_ERROR'
    );
    expect(
      String(fail.message ?? ''),
      `文案应点名 user_ids 不能为空，实际 message=${fail.message}`
    ).toContain('user_ids');
  });
});
