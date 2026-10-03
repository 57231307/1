// BPM 审批端到端 E2E — 03 审批通过/驳回全流程
// 前置：bpm_task 状态大小写缺陷已修复——handler 与写入侧同源使用
//   `task_status::PENDING`("pending")/`task_status::COMPLETED`("completed")，
//   待办/已办列表查询不再恒空。本文件对真实审批链路做端到端回读断言。
// 覆盖范围：
//   - 建流程定义 → 发起流程实例 → 待办出现 → 审批同意 → 已办出现
//   - 审批拒绝 → 任务状态变为 rejected
//   - 非法审批动作 → 400
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  APP_ERROR_CODES,
  genCode,
  tryCleanup,
} from '../flow/helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** 建流程定义并发起流程，返回 { instanceId, taskId, processKey } */
async function seedPendingTask(
  page: import('@playwright/test').Page
): Promise<{ instanceId: number; taskId: number; processKey: string }> {
  // 获取当前用户 ID（作为审批人 assignee_value）
  const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');
  const userIdStr = String(me.id);

  const processKey = genCode('e2e-approval');
  // 创建流程定义，config 含 user_task 节点且 assignee_value = 当前用户 ID
  const defResp = await apiCall<{ id?: number }>(page, 'POST', '/bpm/definitions', {
    name: `E2E 审批流程 ${processKey}`,
    code: processKey,
    description: 'E2E 审批端到端测试',
    category: 'e2e_test',
    version: '1.0',
    config: {
      nodes: [
        { id: 'start', name: '提交审批', type: 'start_event' },
        { id: 'approve_task', name: '审批节点', type: 'user_task', assignee_value: userIdStr },
        { id: 'end', name: '完成', type: 'end_event' },
      ],
      edges: [
        { source: 'start', target: 'approve_task' },
        { source: 'approve_task', target: 'end' },
      ],
    },
    status: 'ACTIVE',
  });
  if (!defResp.data?.id) throw new Error(`建流程定义失败：${JSON.stringify(defResp)}`);
  const defId = defResp.data.id;
  CLEANUP.push({ path: `/bpm/definitions/${defId}`, label: 'bpm_definition' });

  // 发起流程实例
  const startResp = await apiCall<{ instance_id?: number; task_ids?: number[] }>(
    page,
    'POST',
    '/bpm/process/start',
    {
      process_key: processKey,
      business_type: 'e2e_test',
      business_id: defId,
      title: `E2E 审批测试 ${processKey}`,
      initiator_id: me.id,
      initiator_name: `e2e_user_${me.id}`,
    }
  );
  if (!startResp.data?.instance_id) {
    throw new Error(`发起流程失败：${JSON.stringify(startResp)}`);
  }
  const instanceId = startResp.data.instance_id;
  const taskIds = startResp.data.task_ids ?? [];
  if (taskIds.length === 0) {
    throw new Error(`流程发起后未创建任务：${JSON.stringify(startResp)}`);
  }
  return { instanceId, taskId: taskIds[0], processKey };
}

test.describe('03 审批端到端（通过/驳回）', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('03-01 审批同意：待办出现 → 审批 → 已办出现', async ({ page }) => {
    const { taskId } = await seedPendingTask(page);
    const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');

    // 验证待办列表出现该任务（GET /bpm/tasks/pending?user_id=X）
    const pendingList = await apiCallRaw<{
      items?: Array<{ id: number; status: string }>;
    }>(page, 'GET', `/bpm/tasks/pending?user_id=${me.id}&page=1&page_size=50`);
    const pendingItems = pendingList?.items ?? (Array.isArray(pendingList) ? pendingList : []);
    const foundPending = (pendingItems as Array<{ id: number; status: string }>).find(
      t => t.id === taskId
    );
    expect(
      foundPending,
      `待办列表应包含任务 id=${taskId}（验证 pending 大小写修复后过滤正确）`
    ).toBeDefined();
    expect(foundPending!.status).toBe('pending');

    // 执行审批同意
    const approveResp = await apiCall<string>(page, 'POST', '/bpm/tasks/approve', {
      task_id: taskId,
      handler_id: me.id,
      handler_name: `e2e_user_${me.id}`,
      action: 'approve',
      approval_opinion: 'E2E 测试：审批同意',
    });
    expect(approveResp.data, '审批应返回成功').toContain('success');

    // 验证已办列表出现该任务（GET /bpm/tasks/completed?user_id=X）
    const completedList = await apiCallRaw<{
      items?: Array<{ id: number; status: string }>;
    }>(page, 'GET', `/bpm/tasks/completed?user_id=${me.id}&page=1&page_size=50`);
    const completedItems =
      completedList?.items ?? (Array.isArray(completedList) ? completedList : []);
    const foundCompleted = (completedItems as Array<{ id: number; status: string }>).find(
      t => t.id === taskId
    );
    expect(
      foundCompleted,
      `已办列表应包含任务 id=${taskId}（审批通过后 status 变为 completed，小写）`
    ).toBeDefined();
    expect(foundCompleted!.status).toBe('completed');
  });

  test('03-02 审批拒绝：待办出现 → 拒绝 → 任务状态变 rejected', async ({ page }) => {
    const { taskId } = await seedPendingTask(page);
    const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');

    // 验证待办出现
    const pendingList = await apiCallRaw<{ items?: Array<{ id: number }> }>(
      page,
      'GET',
      `/bpm/tasks/pending?user_id=${me.id}&page=1&page_size=50`
    );
    const pendingItems = pendingList?.items ?? (Array.isArray(pendingList) ? pendingList : []);
    const foundPending = (pendingItems as Array<{ id: number }>).find(t => t.id === taskId);
    expect(foundPending, `待办列表应包含任务 id=${taskId}`).toBeDefined();

    // 执行审批拒绝
    await apiCall(page, 'POST', '/bpm/tasks/approve', {
      task_id: taskId,
      handler_id: me.id,
      handler_name: `e2e_user_${me.id}`,
      action: 'reject',
      approval_opinion: 'E2E 测试：审批拒绝',
    });

    // 通过通用查询端点回读任务状态
    const taskList = await apiCallRaw<{ items?: Array<{ id: number; status: string }> }>(
      page,
      'GET',
      `/bpm/tasks?page=1&page_size=100`
    );
    const allItems = taskList?.items ?? (Array.isArray(taskList) ? taskList : []);
    const task = (allItems as Array<{ id: number; status: string }>).find(t => t.id === taskId);
    expect(task, `应能查到任务 id=${taskId}`).toBeDefined();
    expect(task!.status, '拒绝后任务状态应为 rejected（小写）').toBe('rejected');
  });

  test('03-03 非法审批动作被 400 拒绝', async ({ page }) => {
    const { taskId } = await seedPendingTask(page);
    const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');

    // action='invalid' 不在 {approve, reject} 词表中：属提交字段取值/枚举白名单校验，
    // 源码装配点 services/bpm_ops/task.rs:56 `AppError::validation_displayable`
    // → 400 + VALIDATION_ERROR（可外显真实文案），错误族归一后仍为 validation 族，保持。
    const fail = await apiCallExpectFail(page, 'POST', '/bpm/tasks/approve', {
      task_id: taskId,
      handler_id: me.id,
      handler_name: `e2e_user_${me.id}`,
      action: 'invalid_action',
    });
    expect(fail.status).toBe(400);
    expect(failureCode(fail)).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
  });

  test('03-04 重复审批已完成的任务被拒绝', async ({ page }) => {
    const { taskId } = await seedPendingTask(page);
    const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');

    // 第一次审批
    await apiCall(page, 'POST', '/bpm/tasks/approve', {
      task_id: taskId,
      handler_id: me.id,
      handler_name: `e2e_user_${me.id}`,
      action: 'approve',
    });

    // 第二次审批同一任务（状态已非 pending）→ 应被拒绝
    const fail = await apiCallExpectFail(page, 'POST', '/bpm/tasks/approve', {
      task_id: taskId,
      handler_id: me.id,
      handler_name: `e2e_user_${me.id}`,
      action: 'approve',
    });
    // 状态门：load_approve_context 检查 status != pending（task.rs:121）
    // 错误族归一（763c7ab9）后走 AppError::business_displayable「任务当前不处于待处理状态…」
    // → 400 + BUSINESS_ERROR
    expect(fail.status).toBe(400);
    expect(failureCode(fail)).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
  });

  test('03-05 流程定义不存在时发起流程返回 404', async ({ page }) => {
    const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');
    const fail = await apiCallExpectFail(page, 'POST', '/bpm/process/start', {
      process_key: 'non_existent_key_xyz',
      business_type: 'e2e_test',
      business_id: 1,
      title: 'E2E 不存在流程测试',
      initiator_id: me.id,
      initiator_name: `e2e_user_${me.id}`,
    });
    expect(fail.status).toBe(404);
  });
});
