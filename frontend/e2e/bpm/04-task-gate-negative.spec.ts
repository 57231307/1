// BPM 状态门负例 E2E — 04 防回潮（转办/催办/撤回三处状态门）
//
// 背景：后端错误族归一后，BPM 的三个状态门均归 BUSINESS_ERROR 且文案外显：
//   - 转办非待处理任务 → backend/src/services/bpm_ops/task.rs:447
//     `AppError::business_displayable("只能转办待处理任务")`
//   - 催办非待处理任务 → backend/src/services/bpm_ops/task.rs:469
//     `AppError::business_displayable("只能催办待处理任务")`
//   - 撤回已结束流程   → backend/src/services/bpm_ops/instance.rs:191
//     `AppError::business_displayable("流程已结束，无法撤回")`
// 此前 frontend/e2e/bpm/** 对这三处零负例覆盖（仅 03-approval-e2e 覆盖"重复审批"一例）：
// 族被改回、文案被脱敏化都不会有测试报警。本文件补负例 + 正向对照（防"一刀切拒绝"假安全）。
//
// 用例清单：
//   04-00 已注册端点严格健康探活（verifyEndpointHealthy strict 版）
//   04-01 负例：completed 任务转办 → 400 + BUSINESS_ERROR + 原文案，回查任务字段零变化
//   04-02 负例：rejected 任务催办（边界：另一种非待处理态）→ 同上，回查零变化
//   04-03 负例：COMPLETED 流程撤回 → 同上，回查实例与任务零变化
//   04-04 正向对照：pending 任务转办成功 → 回查 actual_handler_id/approval_opinion 真实变化
//   04-05 正向对照+边界：PROCESSING 流程撤回成功 → 实例 CANCELLED、pending 任务级联 cancelled；
//         再次撤回（终态第二样本 CANCELLED）→ 同 04-03 门被拒
//
// 取数口径：每例用本例 API 自建唯一流程定义（genCode 唯一 processKey）+ 唯一流程实例，
// 用后端返回的 instance_id/task_id 精确锚定，回查统一走 GET /bpm/instances/{instance_id}/detail
// （只含本例实例的任务，天然免疫共享 seed / 并发串扰）；afterEach 尽力清理（未结束实例 cancel + definition DELETE）。
//
// 状态词表事实来源 = 写入方 backend/src/models/status/bpm_crm_contract.rs：
//   bpm_task（:100-112，小写）: pending / completed / rejected / cancelled
//   bpm_instance（:84-96，大写）: PROCESSING / COMPLETED / TERMINATED / CANCELLED
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
  verifyEndpointHealthy,
} from '../flow/helpers';

// —— 期望文案常量：逐字符照抄后端源码原文（后端文案非 i18n key），锚点见行尾注释 ——
const MSG_TRANSFER_GATE = '只能转办待处理任务'; // backend/src/services/bpm_ops/task.rs:447
const MSG_URGE_GATE = '只能催办待处理任务'; // backend/src/services/bpm_ops/task.rs:469
const MSG_RECALL_GATE = '流程已结束，无法撤回'; // backend/src/services/bpm_ops/instance.rs:191

interface BpmTaskLite {
  id: number;
  task_no: string;
  status: string | null;
  actual_handler_id: number | null;
  approval_opinion: string | null;
}
interface InstanceDetailLite {
  instance: { id: number; instance_no: string; status: string | null };
  definition_name: string;
  tasks: BpmTaskLite[];
}

const CLEANUP: Array<{ method: 'DELETE' | 'POST'; path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  // 先 cancel 未结束实例（让残留 pending 任务级联 cancelled），再删流程定义
  for (const c of CLEANUP.reverse()) await tryCleanup(page, c.method, c.path, c.label);
  CLEANUP.length = 0;
});

/** 本例自建唯一「流程定义 + 流程实例（含一个 pending 任务）」，返回精确锚点 */
async function seedPendingTask(
  page: import('@playwright/test').Page
): Promise<{ instanceId: number; taskId: number; myId: number }> {
  const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');
  const myId = Number(me.id);
  if (!Number.isInteger(myId) || myId <= 0) {
    throw new Error(`前置失败：/auth/me 未返回有效用户 id：${JSON.stringify(me)}`);
  }

  const processKey = genCode('e2e-gate');
  const defResp = await apiCall<{ id?: number }>(page, 'POST', '/bpm/definitions', {
    name: `E2E 状态门流程 ${processKey}`,
    code: processKey,
    description: 'E2E 04 状态门负例专用（本例自建）',
    category: 'e2e_test',
    version: '1.0',
    config: {
      nodes: [
        { id: 'start', name: '提交审批', type: 'start_event' },
        { id: 'approve_task', name: '审批节点', type: 'user_task', assignee_value: String(myId) },
        { id: 'end', name: '完成', type: 'end_event' },
      ],
      edges: [
        { source: 'start', target: 'approve_task' },
        { source: 'approve_task', target: 'end' },
      ],
    },
    status: 'ACTIVE',
  });
  const defId = defResp.data?.id;
  if (!defId) throw new Error(`建流程定义失败：${JSON.stringify(defResp)}`);
  CLEANUP.push({ method: 'DELETE', path: `/bpm/definitions/${defId}`, label: 'bpm_definition' });

  const startResp = await apiCall<{ instance_id?: number; task_ids?: number[] }>(
    page,
    'POST',
    '/bpm/process/start',
    {
      process_key: processKey,
      business_type: 'e2e_test',
      business_id: defId,
      title: `E2E 状态门测试 ${processKey}`,
      initiator_id: myId,
      initiator_name: `e2e_user_${myId}`,
    }
  );
  const instanceId = startResp.data?.instance_id;
  const taskIds = startResp.data?.task_ids ?? [];
  if (!instanceId || taskIds.length === 0) {
    throw new Error(`发起流程后未返回实例/任务：${JSON.stringify(startResp)}`);
  }
  CLEANUP.push({
    method: 'POST',
    path: `/bpm/instances/${instanceId}/cancel`,
    label: 'bpm_instance_cancel',
  });
  return { instanceId, taskId: taskIds[0], myId };
}

/** 经实例详情精确回查本例任务（GET /bpm/instances/{id}/detail → ProcessInstanceDetail，bpm_handler.rs:156-163） */
async function fetchTaskOfMine(
  page: import('@playwright/test').Page,
  instanceId: number,
  taskId: number
): Promise<BpmTaskLite> {
  const detail = await apiCallRaw<InstanceDetailLite>(
    page,
    'GET',
    `/bpm/instances/${instanceId}/detail`
  );
  if (!Array.isArray(detail?.tasks)) {
    throw new Error(
      `GET /bpm/instances/${instanceId}/detail 未返回 tasks 数组（契约漂移？）：${JSON.stringify(detail).slice(0, 300)}`
    );
  }
  const task = detail.tasks.find(t => t.id === taskId);
  if (!task) throw new Error(`实例 ${instanceId} 详情中找不到本例任务 id=${taskId}`);
  return task;
}

async function fetchInstanceStatus(
  page: import('@playwright/test').Page,
  instanceId: number
): Promise<string | null> {
  const detail = await apiCallRaw<InstanceDetailLite>(
    page,
    'GET',
    `/bpm/instances/${instanceId}/detail`
  );
  return detail?.instance?.status ?? null;
}

/** 审批推进本例任务（approve 或 reject）；handler_id/handler_name 为 DTO 必填字段（bpm_dto.rs:83-90），后端落库取已认证用户 */
async function approveMine(
  page: import('@playwright/test').Page,
  taskId: number,
  myId: number,
  action: 'approve' | 'reject',
  opinion: string
): Promise<void> {
  await apiCall(page, 'POST', '/bpm/tasks/approve', {
    task_id: taskId,
    handler_id: myId,
    handler_name: `e2e_user_${myId}`,
    action,
    approval_opinion: opinion,
  });
}

/** 从用户列表取一个≠当前用户的真实用户 id（正向转办的合法受让人；无第二账号立即判红，不许伪造 id） */
async function resolveOtherUserId(
  page: import('@playwright/test').Page,
  myId: number
): Promise<number> {
  const list = await apiCallRaw<{ users: Array<{ id: number; username: string }>; total: number }>(
    page,
    'GET',
    '/users?page=1&page_size=100'
  );
  if (!Array.isArray(list?.users)) {
    throw new Error(
      `GET /users 未返回 users 数组（UserListResponse 契约漂移？）：${JSON.stringify(list).slice(0, 300)}`
    );
  }
  const other = list.users.find(u => Number(u.id) !== myId);
  if (!other) {
    throw new Error(`用户列表中找不到与当前用户(${myId})不同的第二用户，无法构造转办正向对照`);
  }
  return Number(other.id);
}

test.describe('04 BPM 状态门负例（转办/催办/撤回，防回潮）', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('04-00 相关已注册端点严格健康探活', async ({ page }) => {
    // strict 版 verifyEndpointHealthy：404/403/其它 4xx 一律判红（helpers.ts:2614），
    // 证明本文件依赖的两个只读锚点端点真实注册。
    await verifyEndpointHealthy(page, '/bpm/tasks?page=1&page_size=1');
    await verifyEndpointHealthy(page, '/users?page=1&page_size=1');
  });

  test('04-01 负例：completed 任务调转办被拒 400 BUSINESS_ERROR，且回查任务零变化', async ({
    page,
  }) => {
    const { instanceId, taskId, myId } = await seedPendingTask(page);
    const opinionAtApprove = 'E2E-04-01 前置审批意见';
    await approveMine(page, taskId, myId, 'approve', opinionAtApprove);

    // 前置自检：任务确已流转为 completed（非 pending 前提成立才构成状态门负例；不满足立即抛错而非空跑）
    const before = await fetchTaskOfMine(page, instanceId, taskId);
    expect(before.status, `前置失败：审批后任务应为 completed，实际 ${before.status}`).toBe(
      'completed'
    );
    const otherId = await resolveOtherUserId(page, myId);

    const fail = await apiCallExpectFail(page, 'POST', `/bpm/tasks/${taskId}/transfer`, {
      new_assignee_id: otherId,
      transfer_reason: 'E2E 负例：这次转办必须被状态门拒绝',
    });
    expect(
      fail.status,
      `期望 400（business_displayable 的状态映射，backend/src/utils/error.rs）；` +
        `实际 ${fail.status}；门：backend/src/services/bpm_ops/task.rs:445-447`
    ).toBe(400);
    expect(
      failureCode(fail),
      `期望机器码 ${APP_ERROR_CODES.BUSINESS_ERROR}（error.rs:468-469 BusinessErrorDisplayable）；` +
        `实际 ${JSON.stringify(fail.code)}；对应 task.rs:447`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(
      fail.message,
      `期望逐字符外显原文「${MSG_TRANSFER_GATE}」（task.rs:447）；实际「${fail.message}」`
    ).toBe(MSG_TRANSFER_GATE);

    // 回查任务零变化：失败的转办不得改动状态/处理人/意见（task.rs:450-454 的写入绝不能发生）
    const after = await fetchTaskOfMine(page, instanceId, taskId);
    expect(after.status, `转办被拒后任务状态应仍为 completed，实际 ${after.status}`).toBe(
      'completed'
    );
    expect(
      after.actual_handler_id,
      `转办被拒后 actual_handler_id 应仍为审批人 ${myId}（不得变成受让人 ${otherId}），` +
        `实际 ${after.actual_handler_id}；对应 task.rs:451`
    ).toBe(myId);
    expect(
      after.approval_opinion,
      `转办被拒后 approval_opinion 应仍为审批意见原文，不得被覆写为 "[转办] …"（task.rs:452）；` +
        `期望「${opinionAtApprove}」实际「${after.approval_opinion}」`
    ).toBe(opinionAtApprove);
  });

  test('04-02 负例：rejected 任务调催办被拒 400 BUSINESS_ERROR（边界：另一种非待处理态），回查零变化', async ({
    page,
  }) => {
    const { instanceId, taskId, myId } = await seedPendingTask(page);
    const opinionAtReject = 'E2E-04-02 前置驳回意见';
    await approveMine(page, taskId, myId, 'reject', opinionAtReject);

    const before = await fetchTaskOfMine(page, instanceId, taskId);
    expect(before.status, `前置失败：驳回后任务应为 rejected，实际 ${before.status}`).toBe(
      'rejected'
    );

    const fail = await apiCallExpectFail(page, 'POST', `/bpm/tasks/${taskId}/urge`, {
      urge_message: 'E2E 负例：这次催办必须被状态门拒绝',
    });
    expect(
      fail.status,
      `期望 400；实际 ${fail.status}；门：backend/src/services/bpm_ops/task.rs:467-469`
    ).toBe(400);
    expect(
      failureCode(fail),
      `期望机器码 ${APP_ERROR_CODES.BUSINESS_ERROR}；实际 ${JSON.stringify(fail.code)}；对应 task.rs:469`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(
      fail.message,
      `期望逐字符外显原文「${MSG_URGE_GATE}」（task.rs:469）；实际「${fail.message}」`
    ).toBe(MSG_URGE_GATE);

    // 回查任务零变化：被拒的催办不得改动任务字段，也不得给处理人落催办通知（task.rs:482-516 分支不可达）
    const after = await fetchTaskOfMine(page, instanceId, taskId);
    expect(after.status, `催办被拒后任务状态应仍为 rejected，实际 ${after.status}`).toBe(
      'rejected'
    );
    expect(
      after.actual_handler_id,
      `催办被拒后 actual_handler_id 应仍为 ${myId}，实际 ${after.actual_handler_id}`
    ).toBe(myId);
    expect(
      after.approval_opinion,
      `催办被拒后 approval_opinion 不应变化；期望「${opinionAtReject}」实际「${after.approval_opinion}」`
    ).toBe(opinionAtReject);
    const instStatus = await fetchInstanceStatus(page, instanceId);
    expect(
      instStatus,
      `驳回使实例进入终态 TERMINATED（task.rs:223）；催办被拒后实例状态不应再变化，实际 ${instStatus}`
    ).toBe('TERMINATED');
  });

  test('04-03 负例：COMPLETED 流程调撤回被拒 400 BUSINESS_ERROR，回查实例与任务零变化', async ({
    page,
  }) => {
    const { instanceId, taskId, myId } = await seedPendingTask(page);
    const opinionAtApprove = 'E2E-04-03 前置审批意见';
    await approveMine(page, taskId, myId, 'approve', opinionAtApprove);

    // 前置自检：唯一审批节点通过后流程推进到 end_event → 实例 COMPLETED（task.rs:263-267/376）
    const instBefore = await fetchInstanceStatus(page, instanceId);
    expect(instBefore, `前置失败：审批完成后实例应为 COMPLETED，实际 ${instBefore}`).toBe(
      'COMPLETED'
    );

    const fail = await apiCallExpectFail(page, 'POST', `/bpm/instances/${instanceId}/cancel`, {
      cancel_reason: 'E2E 负例：已结束流程的撤回必须被状态门拒绝',
    });
    expect(
      fail.status,
      `期望 400；实际 ${fail.status}；门：backend/src/services/bpm_ops/instance.rs:186-191`
    ).toBe(400);
    expect(
      failureCode(fail),
      `期望机器码 ${APP_ERROR_CODES.BUSINESS_ERROR}；实际 ${JSON.stringify(fail.code)}；对应 instance.rs:191`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(
      fail.message,
      `期望逐字符外显原文「${MSG_RECALL_GATE}」（instance.rs:191）；实际「${fail.message}」`
    ).toBe(MSG_RECALL_GATE);

    // 回查零变化：实例保持 COMPLETED（不得被改写为 CANCELLED），任务字段原样
    const instAfter = await fetchInstanceStatus(page, instanceId);
    expect(instAfter, `撤回被拒后实例状态应仍为 COMPLETED，实际 ${instAfter}`).toBe('COMPLETED');
    const task = await fetchTaskOfMine(page, instanceId, taskId);
    expect(task.status, `撤回被拒后任务状态应仍为 completed，实际 ${task.status}`).toBe(
      'completed'
    );
    expect(
      task.approval_opinion,
      `撤回被拒后任务 approval_opinion 不应被 cancel_reason 覆写（instance.rs:208 分支不可达）；` +
        `期望「${opinionAtApprove}」实际「${task.approval_opinion}」`
    ).toBe(opinionAtApprove);
    expect(task.actual_handler_id, `任务处理人应仍为 ${myId}`).toBe(myId);
  });

  test('04-04 正向对照：pending 任务转办成功，回查 handler 真实变更（防一刀切拒绝假安全）', async ({
    page,
  }) => {
    const { instanceId, taskId, myId } = await seedPendingTask(page);
    const otherId = await resolveOtherUserId(page, myId);
    const reason = 'E2E-04-04 正向转办原因';

    const before = await fetchTaskOfMine(page, instanceId, taskId);
    expect(before.status, `前置失败：新建任务应为 pending，实际 ${before.status}`).toBe('pending');
    expect(before.actual_handler_id, `前置：转办前处理人应为指派用户 ${myId}`).toBe(myId);

    const ok = await apiCall<string>(page, 'POST', `/bpm/tasks/${taskId}/transfer`, {
      new_assignee_id: otherId,
      transfer_reason: reason,
    });
    // 成功信封文案来源：bpm_handler.rs:229-232 success_with_message(data=「任务转办成功」, message=「任务转办成功」)
    expect(ok.message, `期望成功 message「任务转办成功」；实际「${ok.message}」`).toBe(
      '任务转办成功'
    );
    expect(ok.data, `期望成功 data「任务转办成功」；实际 ${JSON.stringify(ok.data)}`).toBe(
      '任务转办成功'
    );

    // 回查真实变化（task.rs:450-454 的三条写入必须可见）：
    // 转办只换处理人、不推进状态 —— 若三处门被改成"一刀切拒绝"，此例必红
    const after = await fetchTaskOfMine(page, instanceId, taskId);
    expect(
      after.actual_handler_id,
      `转办成功后 actual_handler_id 应由 ${myId} 变为 ${otherId}（task.rs:451），实际 ${after.actual_handler_id}`
    ).toBe(otherId);
    expect(
      after.approval_opinion,
      `转办成功后 approval_opinion 应逐字符等于 "[转办] ${reason}"（format 串：task.rs:452），实际「${after.approval_opinion}」`
    ).toBe(`[转办] ${reason}`);
    expect(after.status, `转办不推进任务状态，应仍为 pending，实际 ${after.status}`).toBe(
      'pending'
    );
    const instStatus = await fetchInstanceStatus(page, instanceId);
    expect(instStatus, `转办不影响实例，应仍为 PROCESSING，实际 ${instStatus}`).toBe('PROCESSING');
  });

  test('04-05 正向对照+边界：PROCESSING 流程撤回成功后，再次撤回（CANCELLED 终态）被同门拒绝', async ({
    page,
  }) => {
    const { instanceId, taskId } = await seedPendingTask(page);
    const cancelReason = 'E2E-04-05 撤回原因';

    const ok = await apiCall<string>(page, 'POST', `/bpm/instances/${instanceId}/cancel`, {
      cancel_reason: cancelReason,
    });
    expect(ok.data, `期望撤回成功 data「撤回成功」；实际 ${JSON.stringify(ok.data)}`).toBe(
      '撤回成功'
    );

    // 回查级联真实效果（instance.rs:205-218）：实例 CANCELLED、本例 pending 任务 → cancelled，
    // 且任务 approval_opinion 被写入 cancel_reason —— 防门被改成"一刀切拒绝"
    const instStatus = await fetchInstanceStatus(page, instanceId);
    expect(instStatus, `撤回成功后实例应为 CANCELLED，实际 ${instStatus}`).toBe('CANCELLED');
    const task = await fetchTaskOfMine(page, instanceId, taskId);
    expect(task.status, `撤回成功后 pending 任务应级联为 cancelled，实际 ${task.status}`).toBe(
      'cancelled'
    );
    expect(
      task.approval_opinion,
      `撤回成功后任务 approval_opinion 应等于 cancel_reason（instance.rs:208），实际「${task.approval_opinion}」`
    ).toBe(cancelReason);

    // 边界第二样本：对 CANCELLED 终态再次撤回 → 命中同一终态门（instance.rs:188 的 CANCELLED 分支）
    const fail = await apiCallExpectFail(page, 'POST', `/bpm/instances/${instanceId}/cancel`, {
      cancel_reason: 'E2E-04-05 第二次撤回必须被拒绝',
    });
    expect(
      fail.status,
      `期望 400；实际 ${fail.status}；门：backend/src/services/bpm_ops/instance.rs:186-191`
    ).toBe(400);
    expect(
      failureCode(fail),
      `期望机器码 ${APP_ERROR_CODES.BUSINESS_ERROR}；实际 ${JSON.stringify(fail.code)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(
      fail.message,
      `期望逐字符外显原文「${MSG_RECALL_GATE}」（instance.rs:191）；实际「${fail.message}」`
    ).toBe(MSG_RECALL_GATE);
  });
});
