/**
 * bpmApprovalSubmit.ts - BPM 审批办理的唯一提交口（两个办理入口共用的写口）
 *
 * 功能：把「审批请求的载荷组装」与「发送」收在一处。载荷键与后端
 * `handlers/bpm_handler.rs` 的 ExecuteApprovalRequest 逐字一致
 * （task_id / handler_id / handler_name / action / approval_opinion），
 * action 取值域 approve 与 reject（`services/bpm_ops/task.rs` 状态机同源）。
 * 出参端点是 POST /bpm/approval/execute（该端点在服务层内再经 approve_task
 * 单点推进任务态），另一形态 POST /bpm/tasks/approve 不再作为界面办理入口。
 *
 * 调用方：审批中心 `useBpmApProc.ts` 的 confirmApproval（对话框办理入口）、
 * 审批管理页 `views/bpm/index.vue` 的 handleApprove（待办行内快捷办理入口）。
 * 入参：BpmApprovalSubmitInput，其中 handlerId 与 handlerName 必须由调用方从
 * 登录态取到真实值后传入；本模块不接受 0 或空串充当办理人，也不做默认值兜底。
 * 传给谁：`api/bpm-enhanced.ts` 的 executeBpmApproval（唯一出参消费面）。
 * 存什么／存哪里：不落盘、不缓存、不写全局状态；原样回传后端的
 * ExecuteApprovalResponse（message、business_type、business_id、instance_id），
 * 由调用方决定回跳与列表刷新，使办理后回读的状态与各页任务列表同源。
 */
import {
  executeBpmApproval,
  type ApprovalAction,
  type ExecuteApprovalResponse,
} from '@/api/bpm-enhanced';

/** 审批动作取值域（与后端状态机逐字符一致，别处不得自造第三值） */
export type BpmApprovalAction = ApprovalAction['action'];

/**
 * 办理入参。
 * taskId 为 bpm_task 主键；handlerId 与 handlerName 为已认证的办理人身份；
 * opinion 为审批意见，空串与缺省一律按「不提交该键」处理（后端为 Option）。
 */
export interface BpmApprovalSubmitInput {
  taskId: number;
  handlerId: number;
  handlerName: string;
  action: BpmApprovalAction;
  opinion?: string;
}

/**
 * 组装审批载荷（纯函数，唯一一处）。
 * 身份缺失或取值域外直接抛错，让入口在发请求前就把问题暴露出来，
 * 而不是发一个后端必然拒绝或会记错审批人的请求。
 */
export function buildBpmApprovalPayload(input: BpmApprovalSubmitInput): ApprovalAction {
  if (!Number.isInteger(input.taskId) || input.taskId <= 0) {
    throw new Error(`审批任务主键非法：taskId=${input.taskId}`);
  }
  if (!Number.isInteger(input.handlerId) || input.handlerId <= 0) {
    throw new Error(`办理人身份非法：handlerId=${input.handlerId}（须取登录态真实用户 ID）`);
  }
  if (!input.handlerName) {
    throw new Error('办理人名称为空，拒绝提交审批（handler_name 为后端必填字段）');
  }
  if (input.action !== 'approve' && input.action !== 'reject') {
    throw new Error(`审批动作取值域外：action=${String(input.action)}`);
  }
  const opinion = input.opinion?.trim();
  const payload: ApprovalAction = {
    task_id: input.taskId,
    handler_id: input.handlerId,
    handler_name: input.handlerName,
    action: input.action,
  };
  // 无意见时整键不出现（后端 approval_opinion 是 Option，缺键即 None；
  // 空串不是「未填」的合法编码，也不给键塞 undefined 让形状含糊）
  if (opinion) payload.approval_opinion = opinion;
  return payload;
}

/**
 * 提交一次审批办理，原样回传后端出参（message、business_type、business_id、
 * instance_id），导航与列表刷新都留给调用方，使办理后回读与各页任务列表同源。
 * 请求失败原样上抛，不吞错、不降级、不在这里编造成功。
 */
export async function submitBpmApproval(
  input: BpmApprovalSubmitInput
): Promise<ExecuteApprovalResponse> {
  const res = await executeBpmApproval(buildBpmApprovalPayload(input));
  return res.data;
}
