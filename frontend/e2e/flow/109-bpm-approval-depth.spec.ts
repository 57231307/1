// BPM 审批实例深度链（与 fullflow 里既有的 BPM 审批链用例互补。那条只钉流程定义状态门与
// 单链流转，本文件补四段从未断言过的真值）
//
// 一、实例与首工单的落库字段。发起实例的写入方是 services 层 bpm_ops 的 instance 子模块，
//   单号前缀 BPM，状态 PROCESSING 全大写，标题由写入方拼成 "流程审批-业务单据 id"（请求体
//   的 title 不入库），initiator_name 落空串，请求体申报的 initiator_id 与 initiator_name
//   一律被已认证登录用户覆写（挂名发起在进服务层之前就被掐掉）。首工单只落
//   actual_handler_id，而 actual_handler_name 与 approval_opinion 与 handled_at 三列全空，
//   裁决信息只在审批落库时才写，创建期不预填。
// 二、待办查询真值。工单状态词表由 models 的 status 目录中 bpm_crm_contract 模块的
//   bpm_task 子模块定义，四值全小写，与实例词表 bpm_instance 的大写四值是两套写入方，
//   本文件一律逐字符比较，绝不做 toLowerCase 归一。待办按 user_id 过滤打在
//   actual_handler_id 列上（assignee_ids 列在创建期从不写，恒为空），所以 user_id 的真实
//   语义是"处理人是我"。待办端点会把 status 强制覆写为 pending，完成面端点强制覆写为
//   completed，调用方传入的 status 一律无效，本文件用词表外的大写 PENDING 反证过滤是
//   大小写敏感的精确匹配。
// 三、审批裁决与业务单据联动。裁决事件 BpmProcessFinished 在事务提交之后发布，监听器按
//   business_type 分发，只认 purchase_order 与 sales_order 与 production_order 三种，其余
//   只记一条告警不做回写。生产工单走专用的 approve_order_via_bpm 与 reject_order_via_bpm，
//   二者不回调 BPM，避免 BPM 到事件再到审批的死循环。工单状态机只允许 PENDING_APPROVAL
//   转 APPROVED 或 REJECTED，而 REJECTED 只允许回 DRAFT，所以驳回后的可重提路径是先退回
//   DRAFT 再提交，原地重提必被状态门拒。事件幂等键含 approved 位，同一单据先拒后批是两个
//   不同的键，不会被去重吞掉。
// 四、越权审批。权限键由 URL 第四段消歧派生，审批提交端点派生的是 bpm-tasks 资源的
//   approve 键，后端权限矩阵把审批执行面只授给销售与采购与生产三类经理岗，system_admin
//   只拿到流程定义与流程模板两个读面，口径是"配置流程的人不是执行审批的人"。所以本文件
//   用 system_admin 账号发起审批，期望 HTTP 403 加机器码 FORBIDDEN 加权限不足文案（该文案
//   只有 RBAC 拒绝分支才出，可用来区分"未知路由"与"没有关联角色"两类同码拒绝），随后回读
//   工单与实例与关联计数零位移，最后由有权的处理人照常审结，证明拒绝只发生在鉴权层，
//   没有把工单打坏。
//
// 未固化为绿灯断言的真实缺陷（源码证据与定级见交付回复）。bpm_ops 的 task 子模块
// load_approve_context 只校验任务存在与状态为 pending，完全没有"调用者是否该任务处理人"
// 的归属门，而 update_task_status 把 actual_handler_id 覆写成调用者，因此任何持
// bpm-tasks 的 approve 键的角色都能替他人审结工单并把处理人写成自己（代审）。本文件的
// 每一次裁决都由工单自己的处理人发起（跨账号场景先用转办把处理人改成本人），既不依赖
// 也不迁就这条缺失的归属门，待该归属门落地时这些断言仍然成立。
//
// 造数纪律。每支用例自建唯一编码的流程定义与唯一的 business_type 与 business_id 与全新
// 生产工单。business_type 唯一化是为了避开在途实例复用幂等，即同一 business_type 与
// business_id 的 PROCESSING 实例会被直接复用而不建新单，撞名会让读回指向别的 worker 的
// 实例。全部打真实后端，不用页面路由拦截造响应，不硬编码任何实体 id。
import { test, expect } from '../diagnose-fixture';
import type { Page } from '@playwright/test';
import {
  APP_ERROR_CODES,
  apiCall,
  apiCallExpectFail,
  apiCallRaw,
  ensureTestEntities,
  failureCode,
  genCode,
  getCtx,
  getRoleCredential,
  loginInIsolatedContext,
  loginViaUI,
  tryCleanup,
} from './helpers';

type Dict = Record<string, unknown>;

/** 实例状态词表，写入方是 models status 目录里 bpm_crm_contract 的 bpm_instance 子模块，全大写 */
const INSTANCE = {
  PROCESSING: 'PROCESSING',
  COMPLETED: 'COMPLETED',
  TERMINATED: 'TERMINATED',
} as const;

/** 工单状态词表，同源模块的 bpm_task 子模块，全小写，与实例词表不同源不可互换 */
const TASK = {
  PENDING: 'pending',
  COMPLETED: 'completed',
  REJECTED: 'rejected',
} as const;

/** 裁决动作词表，写入方是 bpm_ops 的 task 子模块常量，全小写 */
const ACTION = {
  APPROVE: 'approve',
  REJECT: 'reject',
} as const;

/** 生产工单状态词表，写入方是 models status 目录里 production 子模块与 common 子模块，全大写 */
const PROD = {
  DRAFT: 'DRAFT',
  PENDING_APPROVAL: 'PENDING_APPROVAL',
  APPROVED: 'APPROVED',
  REJECTED: 'REJECTED',
} as const;

/** BPM 联动监听器真实识别的 business_type 字面量，来源是事件监听器的分发分支 */
const BIZ_PRODUCTION = 'production_order';

/** 读回响应里的一个键。键缺席直接抛错并点名实际键集，不用空值合并兜底掩盖契约漂移。 */
function readKey(obj: Dict, key: string, label: string): unknown {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label} 响应缺少后端真实键 ${key}，实际键=${Object.keys(obj).join(',')}`);
  }
  return obj[key];
}

/** 读回一个必须是字符串的键，null 与其它类型都判负（字符串空值是合法落库值，不剔除）。 */
function readStr(obj: Dict, key: string, label: string): string {
  const raw = readKey(obj, key, label);
  if (typeof raw !== 'string') {
    throw new Error(`${label} 键 ${key} 不是字符串，raw=${String(raw)}`);
  }
  return raw;
}

/** 读回一个必须可解析为有限数字的键，缺键与不可解析都抛错，不当 0 混过去。 */
function readNum(obj: Dict, key: string, label: string): number {
  const raw = readKey(obj, key, label);
  const value = Number(raw);
  if (!Number.isFinite(value)) {
    throw new Error(`${label} 键 ${key} 不可解析为有限数字，raw=${String(raw)}`);
  }
  return value;
}

/** 读回一个必须是对象且不能为 null 的键。 */
function readObj(obj: Dict, key: string, label: string): Dict {
  const raw = readKey(obj, key, label);
  if (raw === null || typeof raw !== 'object' || Array.isArray(raw)) {
    throw new Error(`${label} 键 ${key} 不是对象，raw=${JSON.stringify(raw)}`);
  }
  return raw as Dict;
}

/** 读回一个必须是数组的键，元素按对象处理。 */
function readArr(obj: Dict, key: string, label: string): Dict[] {
  const raw = readKey(obj, key, label);
  if (!Array.isArray(raw)) {
    throw new Error(`${label} 键 ${key} 不是数组，raw=${JSON.stringify(raw).slice(0, 200)}`);
  }
  return raw as Dict[];
}

/** 读回一个必须是数字数组的键（发起实例出参的 task_ids）。 */
function readNumArr(obj: Dict, key: string, label: string): number[] {
  const raw = readKey(obj, key, label);
  if (!Array.isArray(raw) || raw.some(v => typeof v !== 'number' || !Number.isFinite(v))) {
    throw new Error(`${label} 键 ${key} 不是数字数组，raw=${JSON.stringify(raw)}`);
  }
  return raw as number[];
}

/** 断言一个键当前为 null（落库真值就是空），缺席也算失败。 */
function expectNull(obj: Dict, key: string, label: string): void {
  const raw = readKey(obj, key, label);
  expect(raw, `${label} 键 ${key} 应为 null，实际=${String(raw)}`).toBeNull();
}

/**
 * 轮询读回直到达标或超时。超时抛错并带末次读回原文，绝不以"最后一次的值"直接通过。
 * 用途是裁决事件提交后异步回写业务单据的可见窗口，不是给断言兜底。
 */
async function pollUntil(
  page: Page,
  label: string,
  readPath: string,
  expected: string,
  deadlineMs = 30000,
  intervalMs = 500
): Promise<Dict> {
  const startedAt = Date.now();
  let last: Dict = {};
  for (;;) {
    last = await apiCallRaw<Dict>(page, 'GET', readPath);
    if (readStr(last, 'status', label) === expected) {
      return last;
    }
    if (Date.now() - startedAt > deadlineMs) {
      throw new Error(
        `${label} 在 ${deadlineMs} 毫秒内未达期望状态 ${expected}，末次读回=${JSON.stringify(
          last
        ).slice(0, 400)}`
      );
    }
    await new Promise<void>(resolve => setTimeout(resolve, intervalMs));
  }
}

/** 两级直线审批流程 config，节点与边的消费形状来自 bpm_ops 的 instance 与 task 子模块 */
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

/** 本 worker 登录用户 id。一律现读，不硬编码，也不用共享上下文里的历史值。 */
async function currentUserId(page: Page): Promise<number> {
  const me = await apiCallRaw<Dict>(page, 'GET', '/auth/me');
  return readNum(me, 'id', '当前登录用户');
}

/**
 * 处理人姓名的后端权威推导：bpm_ops 的 task 子模块 resolve_actor_name 取 users 表
 * real_name 去空白非空则用它，否则取 username，两者皆空即判未授权。这里按同一规则
 * 从用户档案端点推导期望值，而不是把某个账号名写死进断言。
 */
async function expectedActorName(page: Page, userId: number): Promise<string> {
  const profile = await apiCallRaw<Dict>(page, 'GET', `/users/${userId}`);
  const realName = readKey(profile, 'real_name', `用户 ${userId} 档案`);
  if (typeof realName === 'string' && realName.trim() !== '') {
    return realName.trim();
  }
  const username = readStr(profile, 'username', `用户 ${userId} 档案`);
  if (username.trim() === '') {
    throw new Error(`用户 ${userId} 既无 real_name 也无 username，无法推导处理人姓名`);
  }
  return username;
}

/** 建定义，返回定义 id 与 code。默认 status 与 version 由调用方按需断言，此处不激活。 */
async function createDefinition(
  page: Page,
  tag: string,
  assigneeId: number
): Promise<{ id: number; code: string; name: string }> {
  const code = genCode(`E109${tag}`);
  const name = `E2E BPM109 ${tag} 两级审批流程`;
  const created = await apiCallRaw<Dict>(page, 'POST', '/bpm/definitions', {
    name,
    code,
    description: `E2E BPM109 ${tag} 专属流程定义`,
    category: 'production',
    config: twoStepFlowConfig(assigneeId),
  });
  const id = readNum(created, 'id', `流程定义创建出参 ${code}`);
  // 定义被实例引用后写入方拒绝删除（引用守卫），此处登记的是尽力清理，
  // 失败仅告警；用例之间靠 code 与 business_type 与 business_id 的唯一取值隔离。
  CLEANUP.push({ path: `/bpm/definitions/${id}`, label: `bpm109 定义 ${id}` });
  return { id, code, name };
}

/** 激活定义版本，同 code 的其它 ACTIVE 行由写入方置 INACTIVE */
async function activateDefinition(page: Page, id: number): Promise<void> {
  await apiCall(page, 'POST', `/bpm/versions/${id}/activate`);
}

/** 建定义并激活（供结构性用例与越权用例直接发起实例） */
async function createActivatedDefinition(
  page: Page,
  tag: string,
  assigneeId: number
): Promise<{ id: number; code: string }> {
  const def = await createDefinition(page, tag, assigneeId);
  await activateDefinition(page, def.id);
  return { id: def.id, code: def.code };
}

/** 发起实例，返回写入方出参三件套。business_type 与 business_id 由调用方保证唯一。 */
async function startInstance(
  page: Page,
  processKey: string,
  businessType: string,
  businessId: number,
  claimedInitiatorId: number,
  claimedInitiatorName: string,
  claimedTitle: string
): Promise<{ instanceId: number; instanceNo: string; taskIds: number[] }> {
  const res = await apiCallRaw<Dict>(page, 'POST', '/bpm/process/start', {
    process_key: processKey,
    business_type: businessType,
    business_id: businessId,
    title: claimedTitle,
    initiator_id: claimedInitiatorId,
    initiator_name: claimedInitiatorName,
  });
  return {
    instanceId: readNum(res, 'instance_id', '发起实例出参'),
    instanceNo: readStr(res, 'instance_no', '发起实例出参'),
    taskIds: readNumArr(res, 'task_ids', '发起实例出参'),
  };
}

/** 实例详情读回（出参形状为 instance 与 tasks 与 approval_chain 与 definition_name 四键） */
async function instanceDetail(
  page: Page,
  instanceId: number
): Promise<{ inst: Dict; tasks: Dict[]; chain: Dict[]; definitionName: string }> {
  const detail = await apiCallRaw<Dict>(page, 'GET', `/bpm/instances/${instanceId}/detail`);
  return {
    inst: readObj(detail, 'instance', '实例详情'),
    tasks: readArr(detail, 'tasks', '实例详情'),
    chain: readArr(detail, 'approval_chain', '实例详情'),
    definitionName: readStr(detail, 'definition_name', '实例详情'),
  };
}

/** 按 node_id 定位工单，定位不到即抛错点名实际节点集，绝不拿第一条蒙过。 */
function pickTaskByNode(tasks: Dict[], nodeId: string, label: string): Dict {
  const rows = tasks.filter(t => readStr(t, 'node_id', `${label} 工单`) === nodeId);
  if (rows.length !== 1) {
    throw new Error(
      `${label} 里节点 ${nodeId} 的工单数应为 1，实际=${rows.length}，全部节点=${tasks
        .map(t => readStr(t, 'node_id', `${label} 工单`))
        .join(',')}`
    );
  }
  return rows[0];
}

/** 按 node_id 定位审批链节点 */
function pickChainNode(chain: Dict[], nodeId: string, label: string): Dict {
  const rows = chain.filter(n => readStr(n, 'node_id', `${label} 链节点`) === nodeId);
  if (rows.length !== 1) {
    throw new Error(
      `${label} 里节点 ${nodeId} 的链节点数应为 1，实际=${rows.length}，全部节点=${chain
        .map(n => readStr(n, 'node_id', `${label} 链节点`))
        .join(',')}`
    );
  }
  return rows[0];
}

/** 业务关联读回（写入方出参含 has_process 与实例与计数三段） */
async function businessRelation(
  page: Page,
  businessType: string,
  businessId: number
): Promise<Dict> {
  return apiCallRaw<Dict>(
    page,
    'GET',
    `/bpm/business-relation?business_type=${encodeURIComponent(
      businessType
    )}&business_id=${businessId}`
  );
}

/** 提交一次裁决。handler_id 与 handler_name 是 DTO 必填但不被消费的申报值，固定送伪造串。 */
async function submitVerdict(
  page: Page,
  taskId: number,
  action: string,
  opinion: string
): Promise<void> {
  await apiCall(page, 'POST', '/bpm/tasks/approve', {
    task_id: taskId,
    handler_id: 900000001,
    handler_name: '冒名办理人',
    action,
    approval_opinion: opinion,
  });
}

/** 待办分页信封读回，同时逐条钉住状态与处理人过滤都被服务端 honored */
async function readTaskPage(
  page: Page,
  path: string,
  expectStatus: string,
  expectHandlerId: number,
  label: string
): Promise<{ items: Dict[]; total: number; page: number; page_size: number }> {
  const payload = await apiCallRaw<Dict>(page, 'GET', path);
  const items = readArr(payload, 'items', label);
  for (const item of items) {
    expect(
      readStr(item, 'status', `${label} 工单`),
      `${label} 返回的工单状态应逐字符等于 ${expectStatus}，实际=${String(item.status)}`
    ).toBe(expectStatus);
    expect(
      readNum(item, 'actual_handler_id', `${label} 工单`),
      `${label} 返回的工单处理人应等于过滤条件里的 ${expectHandlerId}，实际=${String(
        item.actual_handler_id
      )}`
    ).toBe(expectHandlerId);
  }
  return {
    items,
    total: readNum(payload, 'total', label),
    page: readNum(payload, 'page', label),
    page_size: readNum(payload, 'page_size', label),
  };
}

/** 按工单 id 定位工单行，定位不到即抛错点名实际工单 id 集，绝不拿第一条蒙过。 */
function pickTaskById(tasks: Dict[], taskId: number, label: string): Dict {
  const row = tasks.find(t => readNum(t, 'id', `${label} 工单`) === taskId);
  if (!row) {
    throw new Error(
      `${label} 里没有工单 ${taskId}，实际工单=${tasks
        .map(t => readNum(t, 'id', `${label} 工单`))
        .join(',')}`
    );
  }
  return row;
}

/** 取实例里唯一那条 pending 工单行，条数不为 1 即抛错（多实例并行或状态漂移都要显式暴露）。 */
function pickSinglePendingTask(tasks: Dict[], label: string): Dict {
  const rows = tasks.filter(t => readStr(t, 'status', `${label} 工单`) === TASK.PENDING);
  if (rows.length !== 1) {
    throw new Error(
      `${label} 的待办工单数应为 1，实际=${rows.length}，全部工单=${tasks
        .map(t => `${String(readNum(t, 'id', `${label} 工单`))} 状态 ${String(t.status)}`)
        .join(',')}`
    );
  }
  return rows[0];
}

/** 新建生产工单并读回，返回详情对象（联动用例的业务单据主体） */
async function seedProductionOrder(page: Page, productId: number): Promise<Dict> {
  const created = await apiCallRaw<Dict>(page, 'POST', '/production/production-orders/orders', {
    product_id: productId,
    planned_quantity: 100,
    planned_start_date: new Date().toISOString().slice(0, 10),
  });
  const id = readNum(created, 'id', '生产工单创建出参');
  // 终态工单删除会被状态门拒，此处登记尽力清理，失败仅告警
  CLEANUP.push({ path: `/production/production-orders/orders/${id}`, label: `生产工单 ${id}` });
  const detail = await apiCallRaw<Dict>(page, 'GET', `/production/production-orders/orders/${id}`);
  expect(readNum(detail, 'id', '生产工单读回'), '新建工单读回 id 应等于创建出参 id').toBe(id);
  expect(readNum(detail, 'product_id', '生产工单读回')).toBe(productId);
  expect(
    readStr(detail, 'status', '生产工单读回'),
    '新建工单初始状态应逐字符等于写入方词表值 DRAFT'
  ).toBe(PROD.DRAFT);
  return detail;
}

/** 提交工单审批（服务层用固定流程编码拉起实例并把单据置为待审批），返回 business_id */
async function submitOrderForApproval(page: Page, orderId: number): Promise<void> {
  await apiCall(page, 'POST', `/production/production-orders/orders/${orderId}/submit-approval`);
  const after = await apiCallRaw<Dict>(page, 'GET', `/production/production-orders/orders/${orderId}`);
  expect(
    readStr(after, 'status', '生产工单提交后读回'),
    '提交后应逐字符等于写入方词表值 PENDING_APPROVAL'
  ).toBe(PROD.PENDING_APPROVAL);
}

/**
 * 把工单转办给本 worker 登录用户，使随后的裁决由工单自己的处理人发起。
 * 生产域实例由服务层用固定流程编码拉起，其首节点审批人是当初建该定义的账号，
 * 未必是本 worker；转办是主上已稳定的合法通路，用它对齐身份，避免用例依赖
 * 尚未落地的审批人归属门。
 */
async function transferToSelf(
  page: Page,
  instanceId: number,
  taskId: number,
  myId: number
): Promise<void> {
  await apiCall(page, 'POST', `/bpm/tasks/${taskId}/transfer`, {
    new_assignee_id: myId,
    transfer_reason: 'E2E109 对齐处理人身份',
  });
  const detail = await instanceDetail(page, instanceId);
  const task = pickTaskById(detail.tasks, taskId, '转办后实例详情');
  expect(
    readNum(task, 'actual_handler_id', '转办后工单'),
    '转办后处理人应等于受让人本人'
  ).toBe(myId);
  expect(
    readStr(task, 'approval_opinion', '转办后工单'),
    '转办理由应由写入方加前缀落进 approval_opinion'
  ).toContain('E2E109 对齐处理人身份');
  expect(
    readStr(task, 'status', '转办后工单'),
    '转办不应改变工单状态，仍应为 pending'
  ).toBe(TASK.PENDING);
}

const CLEANUP: Array<{ path: string; label: string }> = [];

test.afterEach(async ({ page }) => {
  const queue = CLEANUP.slice().reverse();
  CLEANUP.length = 0;
  for (const c of queue) {
    await tryCleanup(page, 'DELETE', c.path, c.label);
  }
});

test.describe('109 BPM 审批实例深度链', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('109-01 定义激活与实例与首工单落库真值（申报身份被覆写 裁决列全空 审批链默认态）', async ({
    page,
  }) => {
    const myId = await currentUserId(page);
    const def = await createDefinition(page, 'A', myId);
    const created = await apiCallRaw<Dict>(page, 'GET', `/bpm/definitions/${def.id}`);

    expect(
      readStr(created, 'status', '新定义未激活前读回'),
      '未传 status 时写入方应落 DRAFT'
    ).toBe('DRAFT');
    expect(
      readStr(created, 'version', '新定义读回'),
      '未传 version 时写入方应落 v1'
    ).toBe('v1');
    expect(readStr(created, 'code', '新定义读回'), '实体真源键 code').toBe(def.code);
    expect(
      readStr(created, 'process_key', '新定义读回'),
      '别名键 process_key 应与实体真源 code 同值'
    ).toBe(def.code);
    expect(readStr(created, 'name', '新定义读回'), '实体真源键 name').toBe(def.name);
    expect(
      readStr(created, 'process_name', '新定义读回'),
      '别名键 process_name 应与实体真源 name 同值'
    ).toBe(def.name);
    expect(readStr(created, 'category', '新定义读回'), 'category 应原样落库').toBe('production');
    expect(readArr(created, 'nodes', '新定义读回'), 'config 节点数应原样持久化为 4').toHaveLength(
      4
    );

    await activateDefinition(page, def.id);
    const activated = await apiCallRaw<Dict>(page, 'GET', `/bpm/definitions/${def.id}`);
    expect(
      readStr(activated, 'status', '激活后读回'),
      '激活写入方词表值 ACTIVE，逐字符比较'
    ).toBe('ACTIVE');

    const businessType = genCode('E109BIZA');
    const businessId = Number(String(Date.now()).slice(-8));
    const started = await startInstance(
      page,
      def.code,
      businessType,
      businessId,
      // 故意申报一个不等于登录用户的发起人与标题，验证写入方一律以已认证身份覆写
      900000002,
      '冒名发起人',
      '请求标题不该入库'
    );

    const detail = await instanceDetail(page, started.instanceId);
    expect(
      readStr(detail.inst, 'status', '实例读回'),
      '实例状态应逐字符等于写入方词表值 PROCESSING'
    ).toBe(INSTANCE.PROCESSING);
    expect(
      readStr(detail.inst, 'instance_no', '实例读回'),
      '实例单号应由写入方以 BPM 前缀取号'
    ).toContain('BPM');
    expect(started.instanceNo, '发起出参单号应与实例落库单号一致').toBe(
      readStr(detail.inst, 'instance_no', '实例读回')
    );
    expect(
      readStr(detail.inst, 'title', '实例读回'),
      '标题应由写入方拼成 流程审批-业务单据 id，请求体 title 不入库'
    ).toBe(`流程审批-${businessId}`);
    expect(
      readNum(detail.inst, 'initiator_id', '实例读回'),
      '发起人 id 应被覆写为已认证登录用户，而非请求体申报值'
    ).toBe(myId);
    expect(
      readNum(detail.inst, 'applicant_id', '实例读回'),
      '申请人 id 应与发起人同源落库'
    ).toBe(myId);
    expect(
      readStr(detail.inst, 'initiator_name', '实例读回'),
      '写入方把 initiator_name 落空串，请求体申报的姓名不入库'
    ).toBe('');
    expect(
      readNum(detail.inst, 'process_definition_id', '实例读回'),
      '实例应引用本用例刚激活的定义'
    ).toBe(def.id);
    expect(readStr(detail.inst, 'business_type', '实例读回')).toBe(businessType);
    expect(readNum(detail.inst, 'business_id', '实例读回')).toBe(businessId);
    expect(
      readKey(detail.inst, 'started_at', '实例读回'),
      '在途实例应有开始时间'
    ).not.toBeNull();
    expectNull(detail.inst, 'completed_at', '在途实例');
    expect(detail.definitionName, '详情里的定义名应是本用例自建定义').toBe(def.name);

    expect(detail.tasks, '首工单应只有一条').toHaveLength(1);
    const first = pickTaskByNode(detail.tasks, 't1', '首工单');
    expect(started.taskIds, '发起出参 task_ids 应只含首工单').toHaveLength(1);
    expect(readNum(first, 'id', '首工单'), '发起出参工单 id 应与落库工单一致').toBe(
      started.taskIds[0]
    );
    expect(
      readStr(first, 'status', '首工单'),
      '工单状态应逐字符等于写入方小写词表值 pending'
    ).toBe(TASK.PENDING);
    expect(
      readStr(first, 'task_no', '首工单'),
      '工单单号应由写入方以 TSK 前缀取号'
    ).toContain('TSK');
    expect(readStr(first, 'node_name', '首工单'), '节点名应原样落库').toBe('一级审批');
    expect(readStr(first, 'node_type', '首工单'), '节点类型应原样落库').toBe('user_task');
    expect(readNum(first, 'instance_id', '首工单')).toBe(started.instanceId);
    expect(readNum(first, 'actual_handler_id', '首工单'), '首工单处理人应为节点指定用户').toBe(
      myId
    );
    // 创建期只落处理人 id，姓名与裁决三列一律为空，这是"待办可见但无人办过"的真实形状
    expectNull(first, 'actual_handler_name', '首工单');
    expectNull(first, 'approval_opinion', '首工单');
    expectNull(first, 'handled_at', '首工单');
    expectNull(first, 'assignee_ids', '首工单');

    expect(detail.chain, '审批链应覆盖定义的 4 个节点').toHaveLength(4);
    const t1Node = pickChainNode(detail.chain, 't1', '审批链');
    expect(readStr(t1Node, 'status', '审批链一级节点')).toBe(TASK.PENDING);
    expect(readNum(t1Node, 'assignee_id', '审批链一级节点'), '链上处理人取工单 actual_handler_id').toBe(
      myId
    );
    expectNull(t1Node, 'comment', '审批链一级节点');
    const t2Node = pickChainNode(detail.chain, 't2', '审批链');
    expect(
      readStr(t2Node, 'status', '审批链二级节点'),
      '尚未生成工单的节点由写入方填默认小写 pending'
    ).toBe(TASK.PENDING);
    expectNull(t2Node, 'assignee_id', '审批链二级节点');
    const startNode = pickChainNode(detail.chain, 'start', '审批链开始节点');
    expect(readStr(startNode, 'node_type', '审批链开始节点')).toBe('start_event');
    expect(
      readStr(startNode, 'status', '审批链开始节点'),
      '事件节点没有工单行，链上落默认 pending（现状口径，读回即证据）'
    ).toBe(TASK.PENDING);

    const rel = await businessRelation(page, businessType, businessId);
    expect(readKey(rel, 'has_process', '业务关联')).toBe(true);
    expect(readNum(rel, 'instance_id', '业务关联')).toBe(started.instanceId);
    expect(readStr(rel, 'instance_no', '业务关联')).toBe(started.instanceNo);
    expect(readStr(rel, 'process_status', '业务关联'), '关联视图状态与实例同源').toBe(
      INSTANCE.PROCESSING
    );
    expect(readNum(rel, 'task_count', '业务关联')).toBe(1);
    expect(readNum(rel, 'completed_tasks', '业务关联')).toBe(0);
    expect(readNum(rel, 'pending_tasks', '业务关联')).toBe(1);
    expectNull(rel, 'completed_at', '业务关联');
  });

  test('109-02 待办查询真值与两级推进（状态与处理人双过滤 端点覆写 status 计数与链同步）', async ({
    page,
  }) => {
    const myId = await currentUserId(page);
    const def = await createActivatedDefinition(page, 'C', myId);
    const businessType = genCode('E109BIZC');
    const businessId = Number(String(Date.now()).slice(-8));
    const started = await startInstance(page, def.code, businessType, businessId, myId, 'e2e', 'C');

    const mine = await readTaskPage(
      page,
      `/bpm/tasks?user_id=${myId}&status=${encodeURIComponent(TASK.PENDING)}&page=1&page_size=100`,
      TASK.PENDING,
      myId,
      '按处理人与小写 pending 过滤待办'
    );
    expect(
      mine.items.some(t => readNum(t, 'id', '待办项') === started.taskIds[0]),
      '首工单应出现在本人的待办里'
    ).toBe(true);

    // 待办端点强制覆写 status，调用方传 completed 无效
    const forced = await readTaskPage(
      page,
      `/bpm/tasks/pending?user_id=${myId}&status=${encodeURIComponent(
        TASK.COMPLETED
      )}&page=1&page_size=100`,
      TASK.PENDING,
      myId,
      '待办端点覆写 status'
    );
    expect(
      forced.items.some(t => readNum(t, 'id', '待办项') === started.taskIds[0]),
      '待办端点应无视调用方传入的 completed'
    ).toBe(true);

    // 词表外的大写 PENDING 精确匹配不到任何行，反证状态过滤大小写敏感且写入方词表唯一
    const wrongCase = await apiCallRaw<Dict>(
      page,
      'GET',
      `/bpm/tasks?user_id=${myId}&status=PENDING&page=1&page_size=100`
    );
    expect(readNum(wrongCase, 'total', '大写 PENDING 过滤'), '词表外取值应查不到任何行').toBe(0);
    expect(readArr(wrongCase, 'items', '大写 PENDING 过滤')).toHaveLength(0);

    // 一级审批
    const opinion1 = '一级同意 E2E109C';
    await submitVerdict(page, started.taskIds[0], ACTION.APPROVE, opinion1);
    const after1 = await instanceDetail(page, started.instanceId);
    const t1 = pickTaskByNode(after1.tasks, 't1', '一级审批后实例详情');
    expect(
      readStr(t1, 'status', '一级工单'),
      '一级工单应逐字符等于写入方小写词表值 completed'
    ).toBe(TASK.COMPLETED);
    expect(
      readStr(t1, 'approval_opinion', '一级工单'),
      '审批意见应原样落库'
    ).toBe(opinion1);
    expect(
      readStr(t1, 'actual_handler_name', '一级工单'),
      '处理人姓名应由后端按已认证用户解析，请求体申报的冒名不入库'
    ).toBe(await expectedActorName(page, myId));
    expect(
      readNum(t1, 'actual_handler_id', '一级工单'),
      '处理人 id 应是被覆写后的已认证用户，而非请求体申报的 900000001'
    ).toBe(myId);
    expect(readKey(t1, 'handled_at', '一级工单'), '裁决时间应落库').not.toBeNull();
    // 写入方在裁决时不落 action 列，裁决结果只体现在 status 与实例状态与审计上（现状证据）
    expectNull(t1, 'action', '一级工单');

    expect(
      readStr(after1.inst, 'status', '一级审批后实例'),
      '还有二级节点未走，实例应仍停在 PROCESSING'
    ).toBe(INSTANCE.PROCESSING);
    expectNull(after1.inst, 'completed_at', '一级审批后实例');

    const t2 = pickTaskByNode(after1.tasks, 't2', '一级审批后实例详情');
    expect(readStr(t2, 'status', '二级工单'), '推进出的二级工单应为 pending').toBe(TASK.PENDING);
    expect(readNum(t2, 'actual_handler_id', '二级工单')).toBe(myId);
    expect(
      readStr(t2, 'task_no', '二级工单'),
      '二级工单应独立取号'
    ).toContain('TSK');
    expect(
      readStr(t2, 'task_no', '二级工单'),
      '二级工单号不应与一级重号'
    ).not.toBe(readStr(t1, 'task_no', '一级工单'));

    const rel1 = await businessRelation(page, businessType, businessId);
    expect(readNum(rel1, 'task_count', '一级审批后关联计数')).toBe(2);
    expect(readNum(rel1, 'completed_tasks', '一级审批后关联计数')).toBe(1);
    expect(readNum(rel1, 'pending_tasks', '一级审批后关联计数')).toBe(1);
    expect(readStr(rel1, 'process_status', '一级审批后关联状态')).toBe(INSTANCE.PROCESSING);

    // 完成面端点同样覆写 status
    const done = await readTaskPage(
      page,
      `/bpm/tasks/completed?user_id=${myId}&status=${encodeURIComponent(
        TASK.PENDING
      )}&page=1&page_size=100`,
      TASK.COMPLETED,
      myId,
      '完成端点覆写 status'
    );
    expect(done.items.some(t => readNum(t, 'id', '完成项') === readNum(t1, 'id', '一级工单'))).toBe(
      true
    );
    expect(done.items.some(t => readNum(t, 'id', '完成项') === readNum(t2, 'id', '二级工单'))).toBe(
      false
    );

    // 二级审批通过后流程进入终态
    const opinion2 = '二级同意 E2E109C';
    await submitVerdict(page, readNum(t2, 'id', '二级工单'), ACTION.APPROVE, opinion2);
    const after2 = await instanceDetail(page, started.instanceId);
    expect(
      readStr(after2.inst, 'status', '二级审批后实例'),
      '末节点通过后实例应逐字符等于大写词表值 COMPLETED'
    ).toBe(INSTANCE.COMPLETED);
    expect(readKey(after2.inst, 'completed_at', '二级审批后实例')).not.toBeNull();
    const t2Final = pickTaskByNode(after2.tasks, 't2', '二级审批后实例详情');
    expect(readStr(t2Final, 'status', '二级工单终态')).toBe(TASK.COMPLETED);
    expect(readStr(t2Final, 'approval_opinion', '二级工单终态')).toBe(opinion2);

    const rel2 = await businessRelation(page, businessType, businessId);
    expect(readStr(rel2, 'process_status', '二级审批后关联状态')).toBe(INSTANCE.COMPLETED);
    expect(readNum(rel2, 'task_count', '二级审批后关联计数')).toBe(2);
    expect(readNum(rel2, 'completed_tasks', '二级审批后关联计数')).toBe(2);
    expect(readNum(rel2, 'pending_tasks', '二级审批后关联计数')).toBe(0);
    expect(readKey(rel2, 'completed_at', '二级审批后关联计数')).not.toBeNull();

    const doneAfter = await readTaskPage(
      page,
      `/bpm/tasks/completed?user_id=${myId}&page=1&page_size=100`,
      TASK.COMPLETED,
      myId,
      '末节点通过后的完成面'
    );
    expect(
      doneAfter.items.some(t => readNum(t, 'id', '完成项') === readNum(t2Final, 'id', '二级工单')),
      '二级工单应进入完成面'
    ).toBe(true);
    const pendingAfter = await apiCallRaw<Dict>(
      page,
      'GET',
      `/bpm/tasks?user_id=${myId}&status=${encodeURIComponent(TASK.PENDING)}&page=1&page_size=100`
    );
    expect(
      readArr(pendingAfter, 'items', '末节点通过后的待办面').some(
        t => readNum(t, 'id', '待办项') === readNum(t2Final, 'id', '二级工单')
      ),
      '已完成的工单不应残留在待办面'
    ).toBe(false);
  });

  test('109-03 审批通过经事件回写业务单据（生产工单由待审批变已审批）', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    if (!productId) {
      throw new Error('ensureTestEntities 未提供产品 id，生产工单无法造数，前提缺失判红');
    }
    const myId = await currentUserId(page);

    const order = await seedProductionOrder(page, productId);
    const orderId = readNum(order, 'id', '生产工单');
    await submitOrderForApproval(page, orderId);

    const rel = await businessRelation(page, BIZ_PRODUCTION, orderId);
    expect(readKey(rel, 'has_process', '工单流程关联'), '提交后应已存在在途流程实例').toBe(true);
    const instanceId = readNum(rel, 'instance_id', '工单流程关联');
    expect(readStr(rel, 'process_status', '工单流程关联')).toBe(INSTANCE.PROCESSING);
    expect(readNum(rel, 'pending_tasks', '工单流程关联')).toBe(1);
    expect(readNum(rel, 'completed_tasks', '工单流程关联')).toBe(0);

    const before = await instanceDetail(page, instanceId);
    const pendingTask = pickSinglePendingTask(before.tasks, '提交审批后实例详情');
    const taskId = readNum(pendingTask, 'id', '待办工单');
    expect(
      readNum(pendingTask, 'actual_handler_id', '待办工单'),
      '服务层拉起的工单处理人应是定义节点上解析出的真实用户 id'
    ).toBeGreaterThan(0);
    expect(
      readNum(pendingTask, 'instance_id', '待办工单'),
      '工单应挂在业务关联读回的同一实例上'
    ).toBe(instanceId);

    // 转办对齐身份，使裁决由工单自己的处理人发起
    await transferToSelf(page, instanceId, taskId, myId);

    const opinion = 'BPM 审批通过 E2E109';
    await submitVerdict(page, taskId, ACTION.APPROVE, opinion);

    const after = await instanceDetail(page, instanceId);
    expect(
      readStr(after.inst, 'status', '审批后实例'),
      '单节点流程通过末节点后实例应为 COMPLETED'
    ).toBe(INSTANCE.COMPLETED);
    const taskAfter = pickTaskById(after.tasks, taskId, '审批后实例详情');
    expect(readStr(taskAfter, 'status', '审批后工单')).toBe(TASK.COMPLETED);
    expect(readStr(taskAfter, 'approval_opinion', '审批后工单')).toBe(opinion);
    expect(readNum(taskAfter, 'actual_handler_id', '审批后工单'), '处理人应是已认证调用者').toBe(
      myId
    );
    expect(
      readStr(taskAfter, 'actual_handler_name', '审批后工单'),
      '处理人姓名应取后端解析的真实档案值，不是请求体冒名'
    ).toBe(await expectedActorName(page, myId));

    // 事件在事务提交后异步回写业务单据，这是本用例的核心断言
    const docAfter = await pollUntil(page, '生产工单审批联动', `/production/production-orders/orders/${orderId}`, PROD.APPROVED);
    expect(
      readStr(docAfter, 'status', '生产工单联动终态'),
      '联动后应逐字符等于写入方词表值 APPROVED'
    ).toBe(PROD.APPROVED);
    expect(readNum(docAfter, 'id', '生产工单联动终态'), '联动不应换单').toBe(orderId);
    expect(readNum(docAfter, 'planned_quantity', '生产工单联动终态')).toBe(100);

    const relAfter = await businessRelation(page, BIZ_PRODUCTION, orderId);
    expect(readStr(relAfter, 'process_status', '联动后关联状态')).toBe(INSTANCE.COMPLETED);
    expect(readNum(relAfter, 'pending_tasks', '联动后关联计数')).toBe(0);
    expect(readNum(relAfter, 'completed_tasks', '联动后关联计数')).toBe(1);
  });

  test('109-03b 驳回经事件回写业务单据并支持退回草稿后重提至通过', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    if (!productId) {
      throw new Error('ensureTestEntities 未提供产品 id，生产工单无法造数，前提缺失判红');
    }
    const myId = await currentUserId(page);

    const order = await seedProductionOrder(page, productId);
    const orderId = readNum(order, 'id', '生产工单');
    await submitOrderForApproval(page, orderId);

    const rel1 = await businessRelation(page, BIZ_PRODUCTION, orderId);
    const instance1 = readNum(rel1, 'instance_id', '首次提交流程关联');
    const detail1 = await instanceDetail(page, instance1);
    const task1Row = pickSinglePendingTask(detail1.tasks, '首次提交实例详情');
    const task1 = readNum(task1Row, 'id', '首次待办');
    await transferToSelf(page, instance1, task1, myId);

    const rejectOpinion = '面料色号不符 驳回 E2E109';
    await submitVerdict(page, task1, ACTION.REJECT, rejectOpinion);

    const detail1After = await instanceDetail(page, instance1);
    expect(
      readStr(detail1After.inst, 'status', '驳回后实例'),
      '驳回应把实例逐字符置为大写词表值 TERMINATED'
    ).toBe(INSTANCE.TERMINATED);
    expect(readKey(detail1After.inst, 'completed_at', '驳回后实例')).not.toBeNull();
    const task1After = pickTaskById(detail1After.tasks, task1, '驳回后实例详情');
    expect(
      readStr(task1After, 'status', '驳回后工单'),
      '工单应逐字符等于小写词表值 rejected'
    ).toBe(TASK.REJECTED);
    expect(
      readStr(task1After, 'approval_opinion', '驳回后工单'),
      '驳回意见应覆写转办时写入的理由并原样落库'
    ).toBe(rejectOpinion);

    const docRejected = await pollUntil(
      page,
      '生产工单驳回联动',
      `/production/production-orders/orders/${orderId}`,
      PROD.REJECTED
    );
    expect(
      readStr(docRejected, 'status', '驳回后单据'),
      '联动后应逐字符等于写入方词表值 REJECTED'
    ).toBe(PROD.REJECTED);
    const relAfterReject = await businessRelation(page, BIZ_PRODUCTION, orderId);
    expect(readStr(relAfterReject, 'process_status', '驳回后流程关联')).toBe(INSTANCE.TERMINATED);
    expect(readNum(relAfterReject, 'pending_tasks', '驳回后流程关联')).toBe(0);

    // 可重提性的真实边界：REJECTED 只能先退 DRAFT，原地重提必被状态门拒
    const resubmitBlocked = await apiCallExpectFail(
      page,
      'POST',
      `/production/production-orders/orders/${orderId}/submit-approval`
    );
    expect(resubmitBlocked.status, '驳回态直接重提应 HTTP 400').toBe(400);
    expect(
      failureCode(resubmitBlocked),
      '驳回态重提机器码应为 BUSINESS_ERROR（状态机拒绝）'
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    await apiCall(page, 'PUT', `/production/production-orders/orders/${orderId}/status`, {
      status: PROD.DRAFT,
    });
    const backToDraft = await apiCallRaw<Dict>(
      page,
      'GET',
      `/production/production-orders/orders/${orderId}`
    );
    expect(
      readStr(backToDraft, 'status', '退回草稿后读回'),
      '驳回态经唯一合法边退回 DRAFT，这才是可重提态'
    ).toBe(PROD.DRAFT);

    // 重提：上一实例已 TERMINATED，写入方的在途幂等不拦，应新建实例而非复用
    await submitOrderForApproval(page, orderId);
    const rel2 = await businessRelation(page, BIZ_PRODUCTION, orderId);
    const instance2 = readNum(rel2, 'instance_id', '重提后流程关联');
    expect(instance2, '重提应生成新实例，而不是复活已终止实例').not.toBe(instance1);
    expect(readStr(rel2, 'process_status', '重提后流程关联')).toBe(INSTANCE.PROCESSING);
    expect(readNum(rel2, 'task_count', '重提后流程关联')).toBe(1);
    expect(readNum(rel2, 'pending_tasks', '重提后流程关联')).toBe(1);

    const detail2 = await instanceDetail(page, instance2);
    const task2Row = pickSinglePendingTask(detail2.tasks, '重提后实例详情');
    const task2 = readNum(task2Row, 'id', '重提后待办');
    expect(task2, '重提应生成新工单').not.toBe(task1);
    expect(
      readNum(task2Row, 'instance_id', '重提后待办'),
      '重提生成的工单应挂在新实例上，而不是历史实例'
    ).toBe(instance2);
    await transferToSelf(page, instance2, task2, myId);

    const approveOpinion = '整改后通过 E2E109';
    await submitVerdict(page, task2, ACTION.APPROVE, approveOpinion);

    const docApproved = await pollUntil(
      page,
      '重提后审批联动',
      `/production/production-orders/orders/${orderId}`,
      PROD.APPROVED
    );
    expect(
      readStr(docApproved, 'status', '重提后单据终态'),
      '同一单据的先拒后批不会被事件幂等吞掉，应逐字符等于 APPROVED'
    ).toBe(PROD.APPROVED);

    // 两条实例并存，历史实例保持驳回终态不被改写
    const detail1History = await instanceDetail(page, instance1);
    expect(readStr(detail1History.inst, 'status', '历史实例')).toBe(INSTANCE.TERMINATED);
    expect(
      readStr(
        pickTaskById(detail1History.tasks, task1, '历史实例详情'),
        'approval_opinion',
        '历史实例工单'
      ),
      '历史实例的驳回理由应仍可追溯'
    ).toBe(rejectOpinion);
    const detail2After = await instanceDetail(page, instance2);
    expect(readStr(detail2After.inst, 'status', '新实例')).toBe(INSTANCE.COMPLETED);
  });

  test('109-04 无审批执行权限的账号审批被拒且工单实例零副作用', async ({ page, browser }) => {
    const myId = await currentUserId(page);
    const def = await createActivatedDefinition(page, 'D', myId);
    const businessType = genCode('E109BIZD');
    const businessId = Number(String(Date.now()).slice(-8));
    const started = await startInstance(page, def.code, businessType, businessId, myId, 'e2e', 'D');
    const taskId = started.taskIds[0];

    const snapshot = await instanceDetail(page, started.instanceId);
    const snapTask = pickTaskByNode(snapshot.tasks, 't1', '越权前实例详情');
    const snapRel = await businessRelation(page, businessType, businessId);

    const cred = getRoleCredential('system_admin');
    if (!cred) {
      throw new Error(
        'role-credentials 里没有 system_admin 账号凭证，越权前提未落地，属 setup 缺陷判红'
      );
    }
    const attacker = await loginInIsolatedContext(browser, cred.username, cred.password);
    try {
      const attackerId = await currentUserId(attacker.page);
      expect(attackerId, '越权账号必须是与本 worker 不同的身份').not.toBe(myId);

      // 后端权限矩阵把 bpm-tasks 与 bpm-approvals 两个审批执行面都从 system_admin 剔除了，
      // 两个入口都应被 RBAC 拒，且拒绝文案只有权限不足分支才出
      const denyApprove = await apiCallExpectFail(attacker.page, 'POST', '/bpm/tasks/approve', {
        task_id: taskId,
        handler_id: attackerId,
        handler_name: 'system_admin 越权尝试',
        action: ACTION.APPROVE,
        approval_opinion: '越权审批尝试',
      });
      expect(denyApprove.status, '审批提交端点应对无权限角色 403').toBe(403);
      expect(failureCode(denyApprove), '机器码应为 FORBIDDEN').toBe(APP_ERROR_CODES.FORBIDDEN);
      expect(
        String(denyApprove.message ?? ''),
        '403 应来自 RBAC 权限不足分支，而非未知路由或未分配角色'
      ).toContain('权限不足');

      const denyExecute = await apiCallExpectFail(attacker.page, 'POST', '/bpm/approval/execute', {
        task_id: taskId,
        handler_id: attackerId,
        handler_name: 'system_admin 越权尝试',
        action: ACTION.APPROVE,
        approval_opinion: '越权审批尝试',
      });
      expect(denyExecute.status, '审批中心执行入口同样应对无权限角色 403').toBe(403);
      expect(failureCode(denyExecute), '执行入口机器码应为 FORBIDDEN').toBe(
        APP_ERROR_CODES.FORBIDDEN
      );
    } finally {
      await attacker.close();
    }

    // 零副作用回读，逐字段钉住工单与实例与计数都没被动过
    const after = await instanceDetail(page, started.instanceId);
    const afterTask = pickTaskByNode(after.tasks, 't1', '越权后实例详情');
    expect(readStr(afterTask, 'status', '越权后工单'), '被拒不得改变工单状态').toBe(TASK.PENDING);
    expect(
      readNum(afterTask, 'actual_handler_id', '越权后工单'),
      '被拒不得把处理人改写成越权账号'
    ).toBe(myId);
    expect(afterTask.id, '被拒不应新建或替换工单').toBe(snapTask.id);
    expectNull(afterTask, 'actual_handler_name', '越权后工单');
    expectNull(afterTask, 'approval_opinion', '越权后工单');
    expectNull(afterTask, 'handled_at', '越权后工单');
    expect(readStr(after.inst, 'status', '越权后实例'), '被拒不得改变实例状态').toBe(
      INSTANCE.PROCESSING
    );
    expectNull(after.inst, 'completed_at', '越权后实例');
    expect(after.chain, '被拒不得改变审批链节点数').toHaveLength(snapshot.chain.length);
    expect(readNum(afterTask, 'id', '越权后工单')).toBe(readNum(snapTask, 'id', '越权前工单'));

    const relAfter = await businessRelation(page, businessType, businessId);
    expect(readNum(relAfter, 'task_count', '越权后关联计数')).toBe(
      readNum(snapRel, 'task_count', '越权前关联计数')
    );
    expect(readNum(relAfter, 'completed_tasks', '越权后关联计数')).toBe(
      readNum(snapRel, 'completed_tasks', '越权前关联计数')
    );
    expect(readNum(relAfter, 'pending_tasks', '越权后关联计数')).toBe(
      readNum(snapRel, 'pending_tasks', '越权前关联计数')
    );
    expect(readStr(relAfter, 'process_status', '越权后关联状态')).toBe(
      readStr(snapRel, 'process_status', '越权前关联状态')
    );

    // 有权处理人随后照常审结，证明工单未被拒绝动作打坏
    const opinion = '本人审结 E2E109D';
    await submitVerdict(page, taskId, ACTION.APPROVE, opinion);
    const closed = await instanceDetail(page, started.instanceId);
    const closedTask = pickTaskByNode(closed.tasks, 't1', '审结后实例详情');
    expect(readStr(closedTask, 'status', '审结后工单')).toBe(TASK.COMPLETED);
    expect(readStr(closedTask, 'approval_opinion', '审结后工单')).toBe(opinion);
    expect(readStr(closed.inst, 'status', '审结后实例'), '一级通过后应推进而非终止').toBe(
      INSTANCE.PROCESSING
    );
    const t2 = pickTaskByNode(closed.tasks, 't2', '审结后实例详情');
    expect(readStr(t2, 'status', '推进出的二级工单')).toBe(TASK.PENDING);
    const relClosed = await businessRelation(page, businessType, businessId);
    expect(readNum(relClosed, 'task_count', '推进后关联计数')).toBe(2);
    expect(readNum(relClosed, 'pending_tasks', '推进后关联计数')).toBe(1);
    expect(readNum(relClosed, 'completed_tasks', '推进后关联计数')).toBe(1);
  });
});
