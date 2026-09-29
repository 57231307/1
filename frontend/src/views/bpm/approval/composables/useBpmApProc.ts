/**
 * useBpmApProc.ts - BPM 审批流程操作 composable
 * 任务编号: P14 批 2 I-3 第 4 批（拆分原 bpm/approval.vue）
 * 封装审批 / 转交 / 审批链等流程性方法与对话框状态
 *
 * 契约依据（唯一真相 = 后端入参结构体）：
 * - /bpm/approval/execute → handlers/bpm_handler.rs ExecuteApprovalRequest
 *   { task_id, handler_id, handler_name, action, approval_opinion }，
 *   action 取值域 approve | reject（services/bpm_ops/task.rs 状态机同一来源）。
 * - /bpm/tasks/{id}/transfer → handlers/bpm_handler.rs TransferTaskRequest
 *   { new_assignee_id, transfer_reason }。转办不走 execute：execute 的 action
 *   没有 transfer 语义（非 reject 即按 approve 推进流程，会把转办写成审批）。
 * - /bpm/instances/{id}/chain → services/bpm_service_dto.rs ApprovalChainNode
 */
import { ref, reactive } from 'vue';
import { type FormInstance, type FormRules } from 'element-plus';
import { msg } from '@/utils/message';
import { useUserStore } from '@/store/user';
import { i18n } from '@/i18n';
import { getUserList, type User } from '@/api/user';
import {
  executeBpmApproval,
  getBpmEnhancedApprovalChain,
  transferBpmEnhancedTask,
  type ApprovalTask,
  type ApprovalChainNode,
} from '@/api/bpm-enhanced';
import { logger } from '@/utils/logger';

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

  /** 确认审批 */
  const confirmApproval = async () => {
    if (!currentTask.value) return;
    submitLoading.value = true;
    try {
      await executeBpmApproval({
        task_id: currentTask.value.id,
        handler_id: userStore.userInfo?.id ?? 0,
        handler_name: userStore.userInfo?.real_name || userStore.userInfo?.username || '',
        action: approveAction.value,
        approval_opinion: approveForm.comment || undefined,
      });
      msg.success(approveAction.value === 'approve' ? 'approvePassed' : 'approveRejected');
      approveDialogVisible.value = false;
      // 审批后任务从待办迁入已办，两个列表都要刷新，否则已办 Tab 停留在旧数据
      refresh.fetchPendingTasks();
      refresh.fetchCompletedTasks();
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
