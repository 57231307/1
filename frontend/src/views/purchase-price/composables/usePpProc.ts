/**
 * usePpProc.ts - 采购价格流程操作 composable
 * 任务编号: P14 批 2 I-3 第 3 批（拆分原 purchase-price/index.vue）
 * 封装采购价格停用/查看/历史/导出等流程性方法
 * 行为完全保持一致（仅结构重构）
 */
import { ref, reactive } from 'vue';
import { ElMessageBox } from 'element-plus';
import { msg } from '@/utils/message';
import { i18n } from '@/i18n';
import { isDialogDismissal } from '@/utils/monitor';
import {
  approvePurchasePrice,
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

  /** 停用 */
  const handleDisable = async (row: PurchasePrice) => {
    try {
      await ElMessageBox.confirm('确认停用该价格？', '提示', { type: 'warning' });
      // 后端 purchase_price_handler::UpdatePriceRequest 要求 price 必填，停用时回传当前价格不变。
      // row.price 由 api 层如实声明为后端 Decimal 序列化字符串（UpdatePriceRequest.price: String，
      // 证据 models/purchase_price.rs::Model.price 为 Decimal 且 rust_decimal 仅启用 serde feature），
      // 故直接透传，不再 String() 归一（原写法是接口曾把 price 声明为 number 的谎言时代痕迹）
      await updatePurchasePrice(row.id, { price: row.price, status: 'inactive' });
      msg.success('disableSuccess');
      await refresh.getList();
    } catch (error) {
      logger.error('停用失败:', error);
    }
  };

  /**
   * 审批（为 pending 行补状态迁移出边）。
   * 端点契约：POST /purchase/purchase-prices/{id}/approve，体 purchase_price_handler::ApprovePriceRequest
   * （approved: bool 必填且仅受理 true——false 在 handler::approve_price 直接 400"请使用专用拒绝接口"；
   * 服务层状态门（仅 pending 可批）在同 service::approve_price，契约钉
   * backend/tests/contract_wave8_price_approve_gate_test.rs；
   * 采购侧无 reject 路由 ⇒ 仅批准语义，
   * 不复用销售侧 promptApproval 的"通过/拒绝"采集器，拒绝分支对该端点必然 400）。
   * 成功后 refresh.getList() 回读列表（后端 approve_price 已提交状态+审计），
   * 不以 toast 作为生效证据。
   */
  const handleApprove = async (row: PurchasePrice) => {
    try {
      await ElMessageBox.confirm(
        i18n.global.t('purchasePrice.proc.approveConfirm'),
        i18n.global.t('common.confirmTitle'),
        { type: 'warning' }
      );
      await approvePurchasePrice(row.id, { approved: true });
      msg.success('approveSuccess');
      await refresh.getList();
    } catch (error: unknown) {
      // ElMessageBox 取消/X 关闭以 'cancel'|'close' reject：流程中止，不是错误（全站统一判别）
      if (isDialogDismissal(error)) return;
      // 非取消的 reject 必须留痕并外显（request.ts 拦截器已透出后端信封 message，
      // 此处补记操作上下文 + 固定失败文案，不裸 catch 吞错）
      logger.error('采购价格审批失败:', error);
      msg.error('approveFailed');
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
    handleExport,
  });
}
