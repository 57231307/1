/**
 * useSpProc.ts - 销售价格流程操作 composable
 * 任务编号: P14 批 2 I-3 第 3 批（拆分原 sales-price/index.vue）
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
// 导出改用后端带水印 xlsx 接口
// 后端 GET /sales/sales-prices/export 已就绪（含异步审计日志 + 水印）
import { exportFromBackend } from '@/utils/export';

/**
 * 刷新回调
 *
 * getQueryParams：导出时传递列表筛选条件，与后端 SalesPriceQuery（list/export 共用同一结构体）
 * 同口径 = 透传筛选全集（product_id/customer_id/keyword/status）
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
  // 查看详情对话框状态（详情与列表共用同一行对象：后端 list_prices 出参 SalesPriceView 富化行，
  // 详情不再单独调 get_price ⇒ 名称列在详情同样可见）
  const viewDialogVisible = ref(false);
  // ref<any>({}) 旧写法已收紧；空对象仅初始态占位，经断言 bypass（打开即整行覆盖）
  const viewData = ref<SalesPriceRow>({} as SalesPriceRow);

  // 历史记录对话框状态（get_price_history 后端仍整 Model 序列化，出参无名列 ⇒ 行型保持 SalesPrice）
  const historyVisible = ref(false);
  const historyList = ref<SalesPrice[]>([]);

  /**
   * 审批通过（pending → approved）。
   * 交互：先用 promptApprovalReason(true) 采集通过理由（必填、trim 非空才允许继续），再提交；
   * 取消采集框即中止整条链（不弹错误提示）。
   * 端点：POST /sales/sales-prices/{id}/approve，体 sales_price_handler::ApprovePriceRequest
   * （approved 非 Option ⇒ 恒伴发 true，handler::approve_price 对 approved=false 直接 400 并把拒绝
   * 指向真实存在的 reject 端点；approval_reason 必填语义在同 handler 收口，值由
   * service::approve_price 落 sales_prices.approval_reason 列）。
   * 状态门（仅 pending 可批）在 sales_price_service.rs::approve_price，
   * 契约钉 backend/tests/contract_wave8_price_approve_gate_test.rs。
   * 成功后 refresh.getList() 回读列表（后端已提交状态+审计），不以 toast 作为生效证据。
   */
  const handleApprove = async (row: SalesPrice) => {
    const approvalReason = await promptApprovalReason(true);
    if (approvalReason === null) return;
    try {
      await approveSalesPrice(row.id, { approved: true, approval_reason: approvalReason });
      msg.success('approveSuccess');
      await refresh.getList();
    } catch (error: unknown) {
      // 非取消的失败必须留痕并外显（request.ts 拦截器已透出后端信封 message，
      // 此处补记操作上下文 + 固定失败文案，不裸 catch 吞错）
      logger.error('销售价目审批通过失败:', error);
      msg.error('approveFailed');
    }
  };

  /**
   * 审批拒绝（pending → rejected 终态）。
   * 交互：先用 promptRejectReason() 采集拒绝理由（必填、trim 非空才允许继续），再提交；
   * 取消采集框即中止整条链。
   * 端点：POST /sales/sales-prices/{id}/reject（routes/sales.rs:200-203），
   * 体 sales_price_handler::RejectPriceRequest（reason: String 非 Option ⇒ 必带体），
   * 值由 service::reject_price 落 sales_prices.rejected_reason 列；
   * 状态门：仅 pending 起拒，rejected 与 approved 同为审批结论终态、无回退边。
   * 成功后 refresh.getList() 回读列表，不以 toast 作为生效证据。
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

  /** 查看详情（弹出对话框；行对象即列表富化行，详情与列表共用同一读模型，无需二次取数） */
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
      // v11 批次 174 P2-1 修复：catch (error: any) 改为 unknown + 类型守卫
      const errMsg = error instanceof Error ? error.message : String(error);
      ElMessage.error(errMsg || msg.translate('loadHistoryFailed'));
    }
  };

  /**
   * 导出 Excel
   *
   * 导出统一使用 xlsx 格式（禁止 CSV 作为最终交付格式）；
   * 调用后端 GET /sales/sales-prices/export（后端注入水印 + 异步审计日志），
   * 透传当前列表筛选全集（product_id/customer_id/keyword/status），与列表同口径
   *（后端 list/export 共用同一 SalesPriceQuery 结构体；空串/undefined 由 serializeParams 剔除）
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
