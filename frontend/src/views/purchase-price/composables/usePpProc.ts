/**
 * usePpProc.ts - 采购价格流程操作 composable
 * 封装采购价格停用/审批（通过/拒绝）/查看/历史/导出等流程性方法
 */
import { ref, reactive } from 'vue';
import { ElMessageBox } from 'element-plus';
import { msg } from '@/utils/message';
import { i18n } from '@/i18n';
import { isDialogDismissal } from '@/utils/monitor';
import { promptApprovalReason, promptRejectReason } from '@/composables/useActionPrompts';
import {
  approvePurchasePrice,
  rejectPurchasePrice,
  updatePurchasePrice,
  getPurchasePriceHistory,
  type PurchasePrice,
} from '@/api/purchase-price';
import { logger } from '@/utils/logger';

/** 刷新回调 */
interface RefreshCallbacks {
  getList: () => Promise<void>;
}

/**
 * 采购价格流程操作方法集合
 */
export function usePpProc(refresh: RefreshCallbacks) {
  // 查看详情对话框状态
  const viewDialogVisible = ref(false);
  const viewData = ref<Partial<PurchasePrice>>({});

  // 历史记录对话框状态
  const historyVisible = ref(false);
  const historyList = ref<PurchasePrice[]>([]);

  /**
   * 停用（记录级停用：status → inactive，落 purchase_prices.status；
   * 与审批拒绝 rejected 是两种语义，不可互替）。
   * 后端更新请求 price 必填 ⇒ 停用时原值回传当前价格，价格不变。
   */
  const handleDisable = async (row: PurchasePrice) => {
    try {
      await ElMessageBox.confirm(
        i18n.global.t('purchasePrice.proc.disableConfirm'),
        i18n.global.t('common.confirmTitle'),
        { type: 'warning' }
      );
      await updatePurchasePrice(row.id, { price: row.price, status: 'inactive' });
      msg.success('disableSuccess');
      await refresh.getList();
    } catch (error: unknown) {
      // 取消/X 关闭是流程中止，不是失败，不能记成错误日志
      if (isDialogDismissal(error)) return;
      logger.error('采购价格停用失败:', error);
      msg.error('disableFailed');
    }
  };

  /**
   * 审批通过（pending → approved）：理由必填，先经 promptApprovalReason(true) 采集再提交，
   * 取消采集框即中止整条链（不是失败，不弹错误）。
   * 端点 POST /purchase/purchase-prices/{id}/approve 只受理批准，理由落
   * purchase_prices.approval_reason 列；拒绝走独立端点（见 handleReject），
   * 禁止用本端点兼职放行拒绝。
   * 成功后 refresh.getList() 回读列表，以列表数据为准。
   */
  const handleApprove = async (row: PurchasePrice) => {
    const approvalReason = await promptApprovalReason(true);
    if (approvalReason === null) return;
    try {
      await approvePurchasePrice(row.id, { approved: true, approval_reason: approvalReason });
      msg.success('approveSuccess');
      await refresh.getList();
    } catch (error: unknown) {
      logger.error('采购价格审批通过失败:', error);
      msg.error('approveFailed');
    }
  };

  /**
   * 审批拒绝（pending → rejected 审批终态，与记录级停用 inactive 语义不可互替）：
   * 理由必填，先经 promptRejectReason() 采集再提交，取消采集框即中止整条链。
   * 端点 POST /purchase/purchase-prices/{id}/reject（与 approve 各自独立），
   * 理由落 purchase_prices.rejected_reason 列。
   * 成功后 refresh.getList() 回读列表。
   */
  const handleReject = async (row: PurchasePrice) => {
    const reason = await promptRejectReason();
    if (reason === null) return;
    try {
      await rejectPurchasePrice(row.id, { reason });
      msg.success('rejectSuccess');
      await refresh.getList();
    } catch (error: unknown) {
      logger.error('采购价格审批拒绝失败:', error);
      msg.error('rejectFailed');
    }
  };

  /** 查看详情（弹出对话框） */
  const handleView = (row: PurchasePrice) => {
    viewData.value = row;
    viewDialogVisible.value = true;
  };

  /** 历史记录 */
  const handleHistory = async (row: PurchasePrice) => {
    try {
      const res = await getPurchasePriceHistory(row.product_id);
      historyList.value = res.data || [];
      historyVisible.value = true;
    } catch (error) {
      logger.error('获取历史记录失败:', error);
    }
  };

  /** 导出（占位） */
  const handleExport = () => {
    msg.success('exportSuccess');
  };

  // 使用 reactive 包装，访问字段时自动解包 ref
  return reactive({
    // 查看
    viewDialogVisible,
    viewData,
    handleView,
    // 历史
    historyVisible,
    historyList,
    handleHistory,
    // 流程
    handleDisable,
    handleApprove,
    handleReject,
    handleExport,
  });
}
