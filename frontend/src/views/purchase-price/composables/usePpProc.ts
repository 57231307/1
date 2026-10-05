/**
 * usePpProc.ts - 采购价格流程操作 composable
 * 任务编号: P14 批 2 I-3 第 3 批（拆分原 purchase-price/index.vue）
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
   * 停用（记录级停用：status → inactive，由 purchase_price_service.rs::update_price 按
   * price_approval::ALL 白名单校验后落库；与审批拒绝 rejected 是两种语义，不可互替）。
   * 后端 UpdatePriceRequest.price 必填 ⇒ 停用时回传当前价格原值不变。
   * row.price 由 api 层如实声明为后端 Decimal 序列化字符串（models/purchase_price.rs::Model.price
   * 为 Decimal 且 rust_decimal 仅启用 serde feature），故直接透传，不再 String() 归一。
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
   * 审批通过（pending → approved）。
   * 交互：先用 promptApprovalReason(true) 采集通过理由（必填、trim 非空才允许继续），再提交；
   * 取消采集框即中止整条链（不弹错误提示）。
   * 端点：POST /purchase/purchase-prices/{id}/approve，体 purchase_price_handler::ApprovePriceRequest
   * （approved 非 Option ⇒ 恒伴发 true，handler::approve_price 只受理批准；approval_reason 必填语义
   * 在同 handler 收口，值由 service::approve_price 落 purchase_prices.approval_reason 列）；
   * 状态门（仅 pending 可批）在 purchase_price_service.rs::approve_price，
   * 契约钉 backend/tests/contract_wave8_price_approve_gate_test.rs。
   * 销售价目侧同款双动作语义见 frontend/src/views/sales-price/composables/useSpProc.ts。
   * 成功后 refresh.getList() 回读列表（后端 approve_price 已提交状态+审计），
   * 不以 toast 作为生效证据。
   */
  const handleApprove = async (row: PurchasePrice) => {
    const approvalReason = await promptApprovalReason(true);
    if (approvalReason === null) return;
    try {
      await approvePurchasePrice(row.id, { approved: true, approval_reason: approvalReason });
      msg.success('approveSuccess');
      await refresh.getList();
    } catch (error: unknown) {
      // 非取消的 reject 必须留痕并外显（request.ts 拦截器已透出后端信封 message，
      // 此处补记操作上下文 + 固定失败文案，不裸 catch 吞错）
      logger.error('采购价格审批通过失败:', error);
      msg.error('approveFailed');
    }
  };

  /**
   * 审批拒绝（pending → rejected 终态，与记录级停用 inactive 语义不同）。
   * 交互：先用 promptRejectReason() 采集拒绝理由（必填），再提交；取消即中止整条链。
   * 端点：POST /purchase/purchase-prices/{id}/reject（routes/purchase.rs:302-305），
   * 体 purchase_price_handler::RejectPriceRequest（reason: String 非 Option ⇒ 必带体），
   * 值由 service::reject_price 落 purchase_prices.rejected_reason 列；
   * 状态门：仅 pending 起拒，rejected 与 approved 同为审批结论终态。
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
