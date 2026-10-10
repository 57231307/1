/**
 * useSpProc.ts - 销售价格流程操作 composable
 * 封装销售价格审批（通过/拒绝）/查看/历史/导出等流程性方法
 */
import { ref, reactive } from 'vue';
import { ElMessage } from 'element-plus';
import { msg } from '@/utils/message';
import { logger } from '@/utils/logger';
import { promptApprovalReason, promptRejectReason } from '@/composables/useActionPrompts';
import {
  approveSalesPrice,
  rejectSalesPrice,
  getPriceHistory,
  type SalesPrice,
  type SalesPriceRow,
} from '@/api/sales-price';
import { exportFromBackend } from '@/utils/export';

/**
 * 刷新回调
 * getQueryParams：导出时透传列表筛选全集（product_id/customer_id/keyword/status），与列表同口径
 */
interface RefreshCallbacks {
  getList: () => Promise<void>;
  getQueryParams?: () => {
    product_id?: number;
    customer_id?: number;
    keyword?: string;
    status?: string;
  };
}

/**
 * 销售价格流程操作方法集合
 */
export function useSpProc(refresh: RefreshCallbacks) {
  // 查看详情对话框状态（详情复用列表富化行，不再二次取数）
  const viewDialogVisible = ref(false);
  const viewData = ref<SalesPriceRow>({} as SalesPriceRow);

  // 历史记录对话框状态
  const historyVisible = ref(false);
  const historyList = ref<SalesPrice[]>([]);

  /**
   * 审批通过（pending → approved）：理由必填，先经 promptApprovalReason(true) 采集再提交，
   * 取消采集框即中止整条链（不是失败，不弹错误）。
   * 端点 POST /sales/sales-prices/{id}/approve 只受理批准（approved=false 被后端直接 400），
   * 理由落 sales_prices.approval_reason 列；拒绝走独立端点（见 handleReject），
   * 禁止用本端点兼职放行拒绝。
   * 成功后 refresh.getList() 回读列表，以列表数据为准。
   */
  const handleApprove = async (row: SalesPrice) => {
    const approvalReason = await promptApprovalReason(true);
    if (approvalReason === null) return;
    try {
      await approveSalesPrice(row.id, { approved: true, approval_reason: approvalReason });
      msg.success('approveSuccess');
      await refresh.getList();
    } catch (error: unknown) {
      logger.error('销售价目审批通过失败:', error);
      msg.error('approveFailed');
    }
  };

  /**
   * 审批拒绝（pending → rejected 审批终态，与 approved 同为不可回退的结论态）：
   * 理由必填，先经 promptRejectReason() 采集再提交，取消采集框即中止整条链。
   * 端点 POST /sales/sales-prices/{id}/reject（与 approve 各自独立），
   * 理由落 sales_prices.rejected_reason 列。
   * 成功后 refresh.getList() 回读列表。
   */
  const handleReject = async (row: SalesPrice) => {
    const reason = await promptRejectReason();
    if (reason === null) return;
    try {
      await rejectSalesPrice(row.id, { reason });
      msg.success('rejectSuccess');
      await refresh.getList();
    } catch (error: unknown) {
      logger.error('销售价目审批拒绝失败:', error);
      msg.error('rejectFailed');
    }
  };

  /** 查看详情（直接复用列表富化行，无需二次取数） */
  const handleView = (row: SalesPriceRow) => {
    viewData.value = row;
    viewDialogVisible.value = true;
  };

  /** 历史记录 */
  const handleHistory = async (row: SalesPrice) => {
    try {
      const res = await getPriceHistory(row.product_id);
      historyList.value = res.data || [];
      historyVisible.value = true;
    } catch (error: unknown) {
      const errMsg = error instanceof Error ? error.message : String(error);
      ElMessage.error(errMsg || msg.translate('loadHistoryFailed'));
    }
  };

  /**
   * 导出 Excel（统一 xlsx，禁止 CSV 交付）：
   * 调用后端 GET /sales/sales-prices/export（后端注入水印 + 异步审计日志），
   * 透传当前列表筛选全集，与列表同口径。
   */
  const handleExport = async () => {
    const filters = refresh.getQueryParams?.() ?? {};
    const params: Record<string, unknown> = {
      product_id: filters.product_id,
      customer_id: filters.customer_id,
      keyword: filters.keyword || undefined,
      status: filters.status || undefined,
    };
    await exportFromBackend('/sales/sales-prices/export', params, 'sales_prices_export');
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
    handleApprove,
    handleReject,
    handleExport,
  });
}
