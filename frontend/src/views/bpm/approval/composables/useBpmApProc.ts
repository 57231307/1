/**
 * useBpmApProc.ts - BPM 审批流程操作 composable
 * 任务编号: P14 批 2 I-3 第 4 批（拆分原 bpm/approval.vue）
 * 封装审批 / 转交 / 审批链等流程性方法与对话框状态
 *
 * 契约依据（唯一真相 = 后端入参结构体）：
 * - /bpm/approval/execute → handlers/bpm_handler.rs ExecuteApprovalRequest
 *   { task_id, handler_id, handler_name, action, approval_opinion }，
 *   action 取值域 approve | reject（services/bpm_ops/task.rs 状态机同一来源）。
 *   该载荷的组装与发送不在本文件，一律经 bpmApprovalSubmit.ts 的 submitBpmApproval
 *   这个唯一写口，与审批管理页的行内快捷办理共用同一条提交链。
 * - /bpm/tasks/{id}/transfer → handlers/bpm_handler.rs TransferTaskRequest
 *   { new_assignee_id, transfer_reason }。转办不走 execute：execute 的 action
 *   没有 transfer 语义（非 reject 即按 approve 推进流程，会把转办写成审批）。
 * - /bpm/instances/{id}/chain → services/bpm_service_dto.rs ApprovalChainNode
 */
import { ref, reactive } from 'vue';
import { useRouter } from 'vue-router';
import { ElMessage, type FormInstance, type FormRules } from 'element-plus';
import { msg } from '@/utils/message';
import { useUserStore } from '@/store/user';
import { i18n } from '@/i18n';
import { getUserList, type User } from '@/api/user';
import {
  getBpmEnhancedApprovalChain,
  transferBpmEnhancedTask,
  type ApprovalTask,
  type ApprovalChainNode,
} from '@/api/bpm-enhanced';
import { logger } from '@/utils/logger';
import { submitBpmApproval } from './bpmApprovalSubmit';

/**
 * 单据类型到前端路由路径的映射（BPM 审批完成后回跳来源单据上下文）。
 * 收录口径 = 后端 event_bus_ops/listener.rs `handle_bpm_process_finished` 的
 * business_type 分发清单（两入口同源收敛的真实族别），逐一对应前端真实存在的
 * 路由；词表外的类型（含历史假可供族别残留数据）不回跳，避免跳进不存在的页面
 * 落 404 空白页，用户留在审批中心由已办列表与审批链追溯单据上下文。
 */
const BUSINESS_ROUTE_MAP: Record<string, (id: number) => string> = {
  purchase_order: () => '/purchase',
  sales_order: id => `/sales/orders/${id}`,
  production_order: () => '/production',
  quotation: id => `/quotations/${id}`,
  ap_payment_request: () => '/ap',
  purchase_return: () => '/purchase-return',
  sales_return: () => '/sales-returns',
  export_approval: () => '/system/export-approvals',
};

/**
 * 刷新回调
 */
interface RefreshCallbacks {
  fetchPendingTasks: () => Promise<void>;
  fetchCompletedTasks: () => Promise<void>;
}

/**
 * 审批流程操作方法集合
 */
export function useBpmApProc(refresh: RefreshCallbacks) {
  const router = useRouter();
  const userStore = useUserStore();

  // 当前任务
  const currentTask = ref<ApprovalTask | null>(null);

  // 审批对话框
  const approveDialogVisible = ref(false);
  const approveAction = ref<'approve' | 'reject'>('approve');
  const submitLoading = ref(false);
  const approveForm = reactive({ comment: '' });

  // 转交对话框
  const transferDialogVisible = ref(false);
  const transferFormRef = ref<FormInstance>();
  // 接收人必须显式选定：初值留空，由 required 规则拦截（此前默认 1 会让 required 永不生效，
  // 未留意预填值的用户会把任务静默转给用户 ID=1）。
  const transferForm = reactive<{ new_assignee_id?: number; transfer_reason: string }>({
    new_assignee_id: undefined,
    transfer_reason: '',
  });
  const transferCandidates = ref<User[]>([]);
  const transferRules: FormRules = {
    new_assignee_id: [
      {
        required: true,
        message: i18n.global.t('bpm.approval.transferDialog.assigneeRequired'),
        trigger: 'change',
      },
    ],
  };

  // 审批链对话框
  const chainDialogVisible = ref(false);
  const approvalChain = ref<ApprovalChainNode[]>([]);

  /** 打开同意审批对话框 */
  const handleApprove = (row: ApprovalTask) => {
    currentTask.value = row;
    approveAction.value = 'approve';
    approveForm.comment = '';
    approveDialogVisible.value = true;
  };

  /** 打开拒绝审批对话框 */
  const handleReject = (row: ApprovalTask) => {
    currentTask.value = row;
    approveAction.value = 'reject';
    approveForm.comment = '';
    approveDialogVisible.value = true;
  };

  /**
   * 确认审批（审批中心的办理提交面）。
   * 载荷组装与发送全部交给 submitBpmApproval 这个唯一写口；办理人身份取登录态，
   * 取不到就不发请求（不给后端编一个 handler_id=0 的办理记录）。
   */
  const confirmApproval = async () => {
    if (!currentTask.value) return;
    const user = userStore.userInfo;
    if (!user) {
      ElMessage.error(i18n.global.t('bpm.message.currentUserMissing'));
      return;
    }
    submitLoading.value = true;
    try {
      const data = await submitBpmApproval({
        taskId: currentTask.value.id,
        handlerId: user.id,
        handlerName: user.real_name || user.username,
        action: approveAction.value,
        opinion: approveForm.comment,
      });
      msg.success(approveAction.value === 'approve' ? 'approvePassed' : 'approveRejected');
      approveDialogVisible.value = false;
      await Promise.all([refresh.fetchPendingTasks(), refresh.fetchCompletedTasks()]);
      // 审批完成后回跳来源单据（出参携带 business_type/business_id/instance_id 用于导航）。
      // 链路补齐口径：目标路由必须带上 instanceId 与 source，让来源单据页能定位刚办理的那条
      // 上下文并给出返回审批中心的入口；business_id 缺省时仍按 instanceId 回跳，
      // 避免"成功后没带 sourceId 回跳"落到无上下文的列表首页。
      if (data && 'business_type' in data) {
        const navFn = BUSINESS_ROUTE_MAP[data.business_type];
        if (navFn) {
          router.push({
            path: navFn(data.business_id),
            query: {
              source: 'bpm-approval',
              instanceId: String(data.instance_id),
              businessId: String(data.business_id),
            },
          });
        }
      }
    } catch (e) {
      logger.error(String(e));
    } finally {
      submitLoading.value = false;
    }
  };

  /** 打开转交对话框 */
  const handleTransfer = async (row: ApprovalTask) => {
    currentTask.value = row;
    transferForm.new_assignee_id = undefined;
    transferForm.transfer_reason = '';
    transferDialogVisible.value = true;
    try {
      const res = await getUserList({ page: 1, page_size: 200 });
      transferCandidates.value = res.data.users;
    } catch (e) {
      // 候选人取不到时不猜测接收人：清空列表并显式报错，避免用户盲填 ID。
      transferCandidates.value = [];
      logger.error(String(e));
      msg.error('loadFailed');
    }
  };

  /** 确认转交 */
  const confirmTransfer = async () => {
    if (!currentTask.value || !transferFormRef.value) return;
    await transferFormRef.value.validate(async valid => {
      if (!valid) return;
      // required 规则已保证选定；此处仅为类型收窄，不给缺省值编造接收人。
      if (transferForm.new_assignee_id === undefined) return;
      submitLoading.value = true;
      try {
        await transferBpmEnhancedTask(currentTask.value!.id, {
          new_assignee_id: transferForm.new_assignee_id,
          transfer_reason: transferForm.transfer_reason,
        });
        msg.success('transferSuccess');
        transferDialogVisible.value = false;
        refresh.fetchPendingTasks();
      } catch (e) {
        logger.error(String(e));
      } finally {
        submitLoading.value = false;
      }
    });
  };

  /** 打开审批链对话框并加载数据 */
  const handleViewChain = async (row: ApprovalTask) => {
    currentTask.value = row;
    chainDialogVisible.value = true;
    try {
      const res = await getBpmEnhancedApprovalChain(row.instance_id);
      approvalChain.value = res.data;
    } catch (e) {
      logger.error(String(e));
    }
  };

  // 使用 reactive 包装，访问字段时自动解包 ref
  return reactive({
    // 当前任务
    currentTask,
    // 审批对话框
    approveDialogVisible,
    approveAction,
    submitLoading,
    approveForm,
    handleApprove,
    handleReject,
    confirmApproval,
    // 转交对话框
    transferDialogVisible,
    transferFormRef,
    transferForm,
    transferCandidates,
    transferRules,
    handleTransfer,
    confirmTransfer,
    // 审批链对话框
    chainDialogVisible,
    approvalChain,
    handleViewChain,
  });
}
