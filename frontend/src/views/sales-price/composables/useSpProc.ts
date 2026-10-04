/**
 * useSpProc.ts - 销售价格流程操作 composable
 * 任务编号: P14 批 2 I-3 第 3 批（拆分原 sales-price/index.vue）
 * 封装销售价格审批/查看/历史/导出等流程性方法
 * 行为完全保持一致（仅结构重构）
 */
import { ref, reactive } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import { msg } from '@/utils/message';
import { i18n } from '@/i18n';
import { isDialogDismissal } from '@/utils/monitor';
import { logger } from '@/utils/logger';
import {
  approveSalesPrice,
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
   * 审批（批准生效）——单向确认，照采购侧 usePpProc 范式。
   * 端点契约：POST /sales/sales-prices/{id}/approve，体 sales_price_handler::ApprovePriceRequest
   * （approved: bool 必填且仅受理 true——false 在 handler::approve_price 直接 400
   * "审批拒绝请使用专用拒绝接口"；销售价目无 reject 路由 ⇒ 该端点仅批准语义，
   * UI 不提供"拒绝"选项；服务层状态门（仅 pending 可批）在同 service::approve_price，
   * 契约钉 backend/tests/contract_wave8_price_approve_gate_test.rs）。
   * 价目"拒绝"业务语义是否存在属挂账的产品口径（决策建议书裁定 B），本前端不隐式实现、不再制造
   * 点得动但必然 400 的入口。
   * remark 不采集不发送：它是请求体真实字段但只进 tracing 日志、不落库
   * （backend/src/models/sales_price.rs 无 remark 列）⇒ 不收集无法持久化的数据。
   * 成功后 refresh.getList() 回读列表（后端已提交状态+审计），不以 toast 作为生效证据。
   */
  const handleApprove = async (row: SalesPrice) => {
    try {
      await ElMessageBox.confirm(
        i18n.global.t('actionForm.approveTitle'),
        i18n.global.t('common.confirmTitle'),
        { type: 'warning' }
      );
      await approveSalesPrice(row.id, { approved: true });
      msg.success('approveSuccess');
      await refresh.getList();
    } catch (error: unknown) {
      // ElMessageBox 取消/X 关闭以 'cancel'|'close' reject：流程中止，不是错误（全站统一判别）
      if (isDialogDismissal(error)) return;
      // 非取消的失败必须留痕并外显（request.ts 拦截器已透出后端信封 message，
      // 此处补记操作上下文 + 固定失败文案，不裸 catch 吞错）
      logger.error('销售价目审批失败:', error);
      msg.error('approveFailed');
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
    handleExport,
  });
}
