// BPM 域全流程契约级 E2E — 28 流程定义→激活→发起→多级审批推进→拒绝/撤回（实例与任务真实落库回读）
//
// **声明：本文件未在本地实跑（本机禁跑 Playwright），仅按后端源码契约编写，待 CI/联调验证。**
//
// 本链证明什么（覆盖此前只测"待办列表能看到行"的 BPM 链）：
//   1) 定义状态门：流程定义创建默认 DRAFT（services/bpm_process_definition_service.rs:58），
//      DRAFT 定义发起流程必须 404（services/bpm_ops/instance.rs:114-119 仅找 status=ACTIVE）；
//      版本激活后 status=ACTIVE（bpm_process_definition_service.rs:224）才可发起——
//      "未发布的流程不可运行"。编码唯一（:39-55 business_displayable "流程编码已存在"）。
//   2) 发起实例：instance_no 服务端取号 BPM 前缀（instance.rs:121-130）、status=PROCESSING
//      （词表 models/status/bpm_crm_contract.rs:84-96 大写；写入点 instance.rs:141）、
//      首任务 task_no=TSK 前缀、status=pending（小写词表 bpm_crm_contract.rs:100-112，
//      写入点 instance.rs:48-83）。
//   3) 审批动作取值域：action ∉ {approve,reject} → 400 VALIDATION_ERROR
//      （services/bpm_ops/task.rs:45-61），且校验先于任何写（被拒后任务零漂移）；
//      非 pending 任务再审批 → 400 BUSINESS（task.rs:119-124 状态门）。
//   4) approve 链路推进：完成 t1 → 任务 completed + actual_handler_id=**已认证操作人**
//      （task.rs:79-82/:161-162，前端伪造 handler_id 不采信）+ handled_at + 意见回读；
//      自动创建 t2 pending（task.rs:299-316）；完成 t2 → 走到 end_event → 实例 COMPLETED
//      + completed_at（task.rs:256-269/:367-387）；业务关联端点计数回读
//      （instance.rs:249-297：task_count/completed_tasks/pending_tasks）。
//   5) reject 语义：任务 rejected、实例 TERMINATED+completed_at（task.rs:214-235）。
//   6) 撤回语义：处理中实例 cancel → 实例 CANCELLED、pending 任务批量 CANCELLED 且意见=撤回
//      原因（instance.rs:169-245）；终态再撤回 → 400 BUSINESS "流程已结束"（:186-191）；
//      转办/催办只允许 pending 任务（task.rs:445-448/:467-470）。
//
// 处理人取值口径：节点 assignee_value 配为当前分片 admin 的用户 id（字符串），
// 审批动作的真实处理人取 AuthContext（不取 body.handler_id），断言按 auth user id 回读。
//
// CI 测不到（显式声明）：
//   - 实例/任务无删除端点，跑完的 bpm_process_instance/bpm_task 行留在活库（唯一性靠
//     服务端取号，不会互相冲突；定义删除会因实例外键失败，tryCleanup 仅告警）；
//   - BpmProcessFinished 事件总线的下游订阅联动（各业务单据"审批完成回写"由各业务域链自证）；
//   - 边条件表达式 ${amount}>N 的路由分支（evaluate_bpm_condition 有后端单测锁，本链用无条件直线流程）。
import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  ensureTestEntities,
  getCtx,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  genCode,
  tryCleanup,
  APP_ERROR_CODES,
} from '../flow/helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** AppError 机器码取源 flow/helpers 的 APP_ERROR_CODES（NOT_FOUND 登记依据 utils/error.rs:739） */
const ERR_NOT_FOUND = APP_ERROR_CODES.NOT_FOUND;

function requireNum(v: unknown, label: string): number {
  const n = Number(v);
  if (!Number.isFinite(n) || n <= 0)
    throw new Error(`${label}：无有效数值，raw=${JSON.stringify(v)}`);
  return n;
}

/**
 * 两节点直线流程 config：start→t1(user_task)→t2(user_task)→end
 * （节点/边形状 = services/bpm_service.rs:138-173 resolve_first_task_node 与
 *   bpm_ops/task.rs:272-319 try_advance_to_next_node 的实际消费形状）
 */
function twoStepFlowConfig(assigneeId: number) {
  return {
    nodes: [
      { id: 'start', type: 'start_event', name: '开始' },
      { id: 't1', type: 'user_task', name: '一级审批', assignee_value: String(assigneeId) },
      { id: 't2', type: 'user_task', name: '二级审批', assignee_value: String(assigneeId) },
      { id: 'end', type: 'end_event', name: '结束' },
    ],
    edges: [
      { source: 'start', target: 't1' },
      { source: 't1', target: 't2' },
      { source: 't2', target: 'end' },
    ],
  };
}

/** 建定义并激活（返回定义 id/code；实例引用该 code），清理登记尽力删定义 */
async function createActivatedDefinition(
  page: Page,
  tag: string,
  assigneeId?: number
): Promise<{ id: number; code: string }> {
  const ctx = getCtx();
  const code = genCode(`F28${tag}`);
  const def = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/bpm/definitions', {
    name: `E2E两级审批流程-${code}`,
    code,
    config: twoStepFlowConfig(assigneeId ?? ctx.userIds[0]),
  });
  const id = requireNum(def.id, '建流程定义');
  CLEANUP.push({ path: `/bpm/definitions/${id}`, label: `bpm_definition#${id}` });
  // 版本激活 → ACTIVE（bpm_process_definition_service.rs:188-229，路由 /bpm/versions/{id}/activate）
  await apiCall(page, 'POST', `/bpm/versions/${id}/activate`);
  return { id, code };
}

/** 发起实例（initiator 用当前用户 id；business_id 用时间戳保证唯一） */
async function startProcess(
  page: Page,
  processKey: string,
  bizTag: string
): Promise<{ instanceId: number; instanceNo: string; taskIds: number[] }> {
  const ctx = getCtx();
  const res = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/bpm/process/start', {
    process_key: processKey,
    business_type: 'E2E_F28_BIZ',
    business_id: Number(`${Date.now()}`.slice(-8)),
    title: `E2E-F28-${bizTag}`,
    initiator_id: ctx.userIds[0],
    initiator_name: 'e2e',
  });
  const instanceId = requireNum(res.instance_id, '发起实例');
  const taskIds = Array.isArray(res.task_ids) ? (res.task_ids as number[]) : [];
  return { instanceId, instanceNo: String(res.instance_no ?? ''), taskIds };
}

/** 审批任务（action 词表 task.rs:45-46；handler_id/handler_name 为 DTO 必填但服务端不采信） */
function approveBody(taskId: number, action: 'approve' | 'reject', opinion: string) {
  const ctx = getCtx();
  return {
    task_id: taskId,
    handler_id: ctx.userIds[0],
    handler_name: 'e2e',
    action,
    approval_opinion: opinion,
  };
}

/** GET 实例详情（ProcessInstanceDetail：{instance,tasks,approval_chain,definition_name}，
 *  键名源 services/bpm_service_dto.rs:47-57） */
async function instanceDetail(page: Page, instanceId: number) {
  const detail = await apiCallRaw<Record<string, unknown>>(
    page,
    'GET',
    `/bpm/instances/${instanceId}/detail`
  );
  const inst = detail.instance as Record<string, unknown> | undefined;
  const tasks = Array.isArray(detail.tasks) ? (detail.tasks as Record<string, unknown>[]) : null;
  if (!inst || !tasks) {
    throw new Error(
      `[F28] 实例详情信封失配（期望 {instance:{...},tasks:[...]}），实际=${JSON.stringify(
        detail
      ).slice(0, 300)}`
    );
  }
  return { inst, tasks };
}

test.describe('28 BPM 审批链契约', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await ensureTestEntities(page);
  });

  test('28-01 定义状态门：DRAFT 不可发起（404），激活后 ACTIVE 可发起；编码重复 400 BUSINESS', async ({
    page,
  }) => {
    const ctx = getCtx();
    const code = genCode('F28D');
    const def = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/bpm/definitions', {
      name: `E2E门控流程-${code}`,
      code,
      config: twoStepFlowConfig(ctx.userIds[0]),
    });
    const defId = requireNum(def.id, '建定义');
    CLEANUP.push({ path: `/bpm/definitions/${defId}`, label: `bpm_definition#${defId}` });
    // 出参键=实体真源 code/name（bpm_definition_handler.rs:30-52 model_to_frontend_json）
    expect(def.code, '定义 code 回读').toBe(code);
    expect(def.status, '新定义默认 DRAFT（bpm_process_definition_service.rs:58）').toBe('DRAFT');

    // DRAFT 发起 → 404（instance.rs:114-119 "not found or inactive"，NOT_FOUND 码）
    const failDraft = await apiCallExpectFail(page, 'POST', '/bpm/process/start', {
      process_key: code,
      business_type: 'E2E_F28_BIZ',
      business_id: 90000001,
      title: 'E2E-F28-DRAFT',
      initiator_id: ctx.userIds[0],
      initiator_name: 'e2e',
    });
    expect(failDraft.status, '未激活定义发起应 404').toBe(404);
    expect(failureCode(failDraft), '状态门机器码').toBe(ERR_NOT_FOUND);

    // 激活 → ACTIVE（词表写入点 bpm_process_definition_service.rs:224）→ 可发起
    await apiCall(page, 'POST', `/bpm/versions/${defId}/activate`);
    const activated = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/bpm/definitions/${defId}`
    );
    expect(activated.status, '激活后 status=ACTIVE').toBe('ACTIVE');
    const { instanceId } = await startProcess(page, code, 'OK');
    const { inst } = await instanceDetail(page, instanceId);
    expect(inst.status, '实例 PROCESSING（bpm_crm_contract.rs:86）').toBe('PROCESSING');
    expect(instanceId > 0, '发起成功返回实例 id').toBe(true);

    // 编码重复 → 400 BUSINESS（bpm_process_definition_service.rs:39-55）
    const dup = await apiCallExpectFail(page, 'POST', '/bpm/definitions', {
      name: `E2E重复编码-${code}`,
      code,
      config: twoStepFlowConfig(ctx.userIds[0]),
    });
    expect(dup.status, '重复编码应 400').toBe(400);
    expect(failureCode(dup), '重复编码机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
  });

  test('28-02 两级串行审批：pending→completed 链推进、真实处理人回读、非法动作零漂移、重复审批被拒', async ({
    page,
  }) => {
    const ctx = getCtx();
    const { code } = await createActivatedDefinition(page, 'F28A');
    const { instanceId, instanceNo, taskIds } = await startProcess(page, code, 'CHAIN');
    expect(instanceNo, '实例号服务端取号 BPM 前缀').toMatch(/^BPM/);
    expect(taskIds.length, '发起应产出 1 个首任务').toBe(1);
    const t1 = taskIds[0];

    let d = await instanceDetail(page, instanceId);
    expect(d.tasks.length, '仅首任务存在').toBe(1);
    expect(d.tasks[0].status, '首任务 pending（小写词表 bpm_crm_contract.rs:102）').toBe('pending');
    expect(String(d.tasks[0].task_no ?? ''), '任务号 TSK 前缀（instance.rs:52-60）').toMatch(
      /^TSK/
    );

    // 非法动作取值 → 400 VALIDATION（task.rs:52-61），校验先于写：任务零漂移
    const badAction = await apiCallExpectFail(page, 'POST', '/bpm/tasks/approve', {
      ...approveBody(t1, 'approve', ''),
      action: 'transfer',
    });
    expect(badAction.status, '词表外审批动作应 400').toBe(400);
    expect(failureCode(badAction), '动作取值机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    d = await instanceDetail(page, instanceId);
    expect(d.tasks[0].status, '非法动作后任务仍 pending（零漂移）').toBe('pending');

    // 审批 t1 → completed，处理人=已认证用户（task.rs:79-82 不采信 body.handler_id），
    // 意见/时间落库；t2 自动创建 pending（task.rs:299-316）
    await apiCall(page, 'POST', '/bpm/tasks/approve', approveBody(t1, 'approve', 'E2E一级同意'));
    d = await instanceDetail(page, instanceId);
    const done1 = d.tasks.find(t => Number(t.id) === t1);
    expect(done1?.status, 't1 completed（词表 :105）').toBe('completed');
    expect(Number(done1?.actual_handler_id), '真实处理人=当前用户').toBe(ctx.userIds[0]);
    expect(done1?.approval_opinion, '审批意见回读').toBe('E2E一级同意');
    expect(done1?.handled_at, 'handled_at 真实写入').toBeTruthy();
    const t2 = d.tasks.find(t => Number(t.id) !== t1);
    expect(t2, 't2 已创建').toBeTruthy();
    expect(t2!.status, 't2 pending —— 未审批不得完成流程').toBe('pending');
    const midInst = await instanceDetail(page, instanceId);
    expect(midInst.inst.status, 't2 未完成时实例必须仍 PROCESSING').toBe('PROCESSING');

    // 重复审批 t1 → 400 BUSINESS（task.rs:119-124 状态门）
    const dupApprove = await apiCallExpectFail(page, 'POST', '/bpm/tasks/approve', {
      ...approveBody(t1, 'approve', ''),
      action: 'approve',
    });
    expect(dupApprove.status, '重复审批应 400').toBe(400);
    expect(failureCode(dupApprove), '重复审批机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // 审批 t2 → 走到 end_event → 实例 COMPLETED + completed_at（task.rs:263-268/:367-387）
    await apiCall(
      page,
      'POST',
      '/bpm/tasks/approve',
      approveBody(Number(t2!.id), 'approve', 'E2E二级同意')
    );
    d = await instanceDetail(page, instanceId);
    expect(d.inst.status, '全链审批后实例 COMPLETED（大写词表 :89）').toBe('COMPLETED');
    expect(d.inst.completed_at, 'completed_at 真实写入').toBeTruthy();

    // 业务关联端点计数回读（instance.rs:249-297）
    const rel = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/bpm/business-relation?business_type=E2E_F28_BIZ&business_id=${Number(d.inst.business_id)}`
    );
    expect(rel.has_process, '业务关联应查到流程').toBe(true);
    expect(Number(rel.instance_id), '关联实例 id').toBe(instanceId);
    expect(rel.process_status, '关联回读 COMPLETED').toBe('COMPLETED');
    expect(Number(rel.task_count), '任务总数 2').toBe(2);
    expect(Number(rel.completed_tasks), '完成任务 2').toBe(2);
    expect(Number(rel.pending_tasks), '待处理 0').toBe(0);
  });

  test('28-03 拒绝链：任务 rejected + 实例 TERMINATED，之后不可再审批', async ({ page }) => {
    const { code } = await createActivatedDefinition(page, 'F28R');
    const { instanceId, taskIds } = await startProcess(page, code, 'REJ');
    const t1 = taskIds[0];

    await apiCall(page, 'POST', '/bpm/tasks/approve', approveBody(t1, 'reject', 'E2E拒绝'));
    const d = await instanceDetail(page, instanceId);
    const task = d.tasks.find(t => Number(t.id) === t1);
    expect(task?.status, 'reject → rejected（词表 :108）').toBe('rejected');
    expect(d.inst.status, 'reject → 实例 TERMINATED（大写词表 :92，task.rs:222-223）').toBe(
      'TERMINATED'
    );
    expect(d.inst.completed_at, '终止时间写入').toBeTruthy();

    const again = await apiCallExpectFail(page, 'POST', '/bpm/tasks/approve', {
      ...approveBody(t1, 'approve', ''),
    });
    expect(again.status, '终止后任务再审批应 400').toBe(400);
    expect(failureCode(again), '非 pending 门机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
  });

  test('28-04 撤回链：实例 CANCELLED + pending 任务批量 CANCELLED（意见=撤回原因）；终态再撤回被拒', async ({
    page,
  }) => {
    const { code } = await createActivatedDefinition(page, 'F28C');
    const { instanceId, taskIds } = await startProcess(page, code, 'CAN');

    await apiCall(page, 'POST', `/bpm/instances/${instanceId}/cancel`, {
      cancel_reason: 'E2E发起人撤回',
    });
    const d = await instanceDetail(page, instanceId);
    expect(d.inst.status, '撤回 → CANCELLED（词表 :95）').toBe('CANCELLED');
    expect(d.inst.completed_at, '撤回写 completed_at').toBeTruthy();
    const t1 = d.tasks.find(t => Number(t.id) === taskIds[0]);
    expect(t1?.status, 'pending 任务联动 cancelled（小写词表 :111，instance.rs:205-218）').toBe(
      'cancelled'
    );
    expect(t1?.approval_opinion, '任务意见=撤回原因').toBe('E2E发起人撤回');

    const twice = await apiCallExpectFail(page, 'POST', `/bpm/instances/${instanceId}/cancel`, {
      cancel_reason: 'E2E再次撤回',
    });
    expect(twice.status, '终态再撤回应 400').toBe(400);
    expect(failureCode(twice), '撤回状态门机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(twice.message, 'displayable 拒绝原因外显（instance.rs:191）').toBeTruthy();
  });

  test('28-05 转办/催办仅 pending：转办改写处理人字段（前后值必须不同），终态任务操作一律 400 BUSINESS', async ({
    page,
  }) => {
    const ctx = getCtx();
    // 节点 assignee 配成哨兵 id（999999，bpm_task.actual_handler_id 无 FK，instance.rs:73-77 原样落库），
    // 转办目标才是当前用户——两值不同，"转办生效"不可被"配置本就等于目标"假绿
    const { code } = await createActivatedDefinition(page, 'F28T', 999999);
    const { instanceId, taskIds } = await startProcess(page, code, 'TRF');
    const t1 = taskIds[0];

    let d = await instanceDetail(page, instanceId);
    let task = d.tasks.find(t => Number(t.id) === t1);
    expect(
      Number(task?.actual_handler_id),
      'config assignee_value 原样落库（前置：与转办目标不同）'
    ).toBe(999999);

    // 转办 pending 任务 → actual_handler_id 改写（task.rs:432-458）
    const newAssignee = ctx.userIds[0];
    await apiCall(page, 'POST', `/bpm/tasks/${t1}/transfer`, {
      new_assignee_id: newAssignee,
      transfer_reason: 'E2E转办负例',
    });
    d = await instanceDetail(page, instanceId);
    task = d.tasks.find(t => Number(t.id) === t1);
    expect(Number(task?.actual_handler_id), '转办后处理人字段回读=新处理人').toBe(newAssignee);
    expect(String(task?.approval_opinion ?? ''), '转办留痕 [转办] 前缀').toMatch(/^\[转办\]/);

    // 审批完成该任务后：转办/催办状态门生效（task.rs:445-448/:467-470）
    await apiCall(page, 'POST', '/bpm/tasks/approve', approveBody(t1, 'approve', 'E2E完成t1'));
    const trfDone = await apiCallExpectFail(page, 'POST', `/bpm/tasks/${t1}/transfer`, {
      new_assignee_id: newAssignee,
      transfer_reason: 'E2E转办已完成任务',
    });
    expect(trfDone.status, '已完成任务转办应 400').toBe(400);
    expect(failureCode(trfDone), '转办状态门机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const urgeDone = await apiCallExpectFail(page, 'POST', `/bpm/tasks/${t1}/urge`, {
      urge_message: 'E2E催办已完成任务',
    });
    expect(urgeDone.status, '已完成任务催办应 400').toBe(400);
    expect(failureCode(urgeDone), '催办状态门机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // 剩余 t2 仍 pending：清理现场——撤回实例（不影响断言，仅收敛活库残留）
    const detail = await instanceDetail(page, instanceId);
    if (detail.inst.status === 'PROCESSING') {
      await apiCall(page, 'POST', `/bpm/instances/${instanceId}/cancel`, {
        cancel_reason: 'E2E收尾',
      });
    }
  });

  test('28-06 不存在的流程 key 发起 → 404（引用门前置）', async ({ page }) => {
    const ctx = getCtx();
    const fail = await apiCallExpectFail(page, 'POST', '/bpm/process/start', {
      process_key: `F28NOTEXIST${Date.now()}`,
      business_type: 'E2E_F28_BIZ',
      business_id: 90000099,
      title: 'E2E-F28-404',
      initiator_id: ctx.userIds[0],
      initiator_name: 'e2e',
    });
    expect(fail.status, '幽灵流程 key 应 404').toBe(404);
    expect(failureCode(fail), '404 机器码').toBe(ERR_NOT_FOUND);
  });
});
