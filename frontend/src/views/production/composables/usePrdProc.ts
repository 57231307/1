/**
 * usePrdProc.ts - 生产管理流程操作 composable
 * 任务编号: P14 批 2 I-3 第 4 批（拆分原 production/index.vue）
 * 封装状态变更 / 导出 CSV / 打印等流程性方法
 * 行为完全保持一致（仅结构重构）
 *
 * 设计说明：通过 callbacks 接收 usePrd 的状态引用（Reactive 包装层）；
 * 由于 usePrd 返回 reactive({...})，父组件传入 prd.data 等会自动解包为值
 */
import { reactive } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import { msg } from '@/utils/message';
import { i18n } from '@/i18n';
import {
  deleteProductionOrder,
  updateProductionOrderStatus,
  submitProductionOrder,
  approveProductionOrder,
  reportProductionProgress,
  getProductionOrderLogs,
  type ProductionOrder,
} from '@/api/production';
import { getStatusLabel } from './prdFmts';
import { escapeHtml } from '@/utils/print';
// V15 P0-S12 修复（Batch 475c）：导出改用后端带水印 xlsx 接口
// 后端 GET /production-orders/orders/export 已就绪（含行级数据权限 + 异步审计日志 + 水印）
import { exportFromBackend } from '@/utils/export';

/**
 * 流程回调（接收 usePrd 返回的状态，自动解包后的值类型）
 *
 * V15 P0-S12 修复（Batch 475c）：新增 getQueryParams，用于导出时传递列表筛选条件
 * 保证导出数据与当前列表筛选一致（status/product_id）
 */
interface PrdCallbacks {
  // 列表数据
  data: ProductionOrder[];
  // 刷新列表
  refresh: () => Promise<void>;
  // V15 P0-S12 修复（Batch 475c）：获取当前筛选条件（status/product_id），用于导出
  getQueryParams?: () => { status?: string; product_id?: number };
}

/**
 * 生产管理流程操作方法集合
 */
export function usePrdProc(cb: PrdCallbacks) {
  /** 状态变更 */
  const handleStatusChange = async (row: ProductionOrder, status: string) => {
    try {
      await ElMessageBox.confirm(
        `确认将订单 ${row.order_no} 状态更改为 ${getStatusLabel(status)} 吗？`,
        '确认',
        { type: 'warning' }
      );
      await updateProductionOrderStatus(row.id, status);
      msg.success('statusUpdateSuccess');
      await cb.refresh();
    } catch (e: unknown) {
      if (e !== 'cancel') {
        const err = e as { message?: string };
        ElMessage.error(err.message || msg.translate('statusUpdateFailed'));
      }
    }
  };

  /** 删除订单 */
  const handleDelete = async (row: ProductionOrder) => {
    try {
      await ElMessageBox.confirm(`确认删除订单 ${row.order_no} 吗？此操作不可恢复。`, '删除确认', {
        type: 'warning',
        confirmButtonText: '确定',
        cancelButtonText: '取消',
      });
      await deleteProductionOrder(row.id);
      msg.success('deleteSuccess');
      await cb.refresh();
    } catch (e: unknown) {
      if (e !== 'cancel') {
        const err = e as { message?: string };
        ElMessage.error(err.message || msg.translate('deleteFailed'));
      }
    }
  };

  /**
   * 导出 Excel（V15 P0-S12 修复 Batch 475c）
   *
   * 规则 3：导出统一使用 xlsx 格式（禁止 CSV 作为最终交付格式）
   * 改为调用后端 GET /production/production-orders/orders/export，后端注入水印 + 行级数据权限 + 异步审计日志
   * 传入当前列表筛选条件（status/product_id），保证导出与列表一致
   */
  const handleExport = async () => {
    if (cb.data.length === 0) {
      msg.warning('noDataToExport');
      return;
    }
    const filters = cb.getQueryParams?.() ?? {};
    const params: Record<string, unknown> = {
      status: filters.status || undefined,
      product_id: filters.product_id,
    };
    await exportFromBackend(
      '/production/production-orders/orders/export',
      params,
      'production_orders_export'
    );
  };

  /** 打印 */
  const handlePrint = () => {
    const printWindow = window.open('', '_blank');
    if (!printWindow) {
      msg.error('printWindowBlocked');
      return;
    }
    const rows = cb.data
      .map(
        (item: ProductionOrder) => `
    <tr>
      <td>${escapeHtml(item.order_no)}</td><td>${escapeHtml(item.product_name || '-')}</td>
      <td style="text-align:right">${escapeHtml(item.planned_quantity)}</td>
      <td style="text-align:right">${escapeHtml(item.actual_quantity || '-')}</td>
      <td>${escapeHtml(item.planned_start_date?.substring(0, 10) || '-')}</td>
      <td>${escapeHtml(item.planned_end_date?.substring(0, 10) || '-')}</td>
      <td>${escapeHtml(getStatusLabel(item.status))}</td><td>${escapeHtml(item.priority)}</td>
    </tr>
  `
      )
      .join('');
    printWindow.document.write(`<html><head><meta charset="utf-8"><title>生产订单</title>
    <style>@media print{@page{size:landscape;}}body{font-family:"Microsoft YaHei",sans-serif;font-size:12px;}h1{text-align:center;}table{width:100%;border-collapse:collapse;margin-top:12px;}th,td{border:1px solid #333;padding:6px 8px;}th{background:#f5f5f5;}.meta{text-align:center;color:#666;font-size:11px;}</style></head><body>
    <h1>生产订单列表</h1><div class="meta">打印日期: ${new Date().toISOString().split('T')[0]} | 共 ${cb.data.length} 条</div>
    <table><thead><tr><th>订单编号</th><th>产品名称</th><th>计划数量</th><th>实际数量</th><th>计划开始</th><th>计划结束</th><th>状态</th><th>优先级</th></tr></thead><tbody>${rows}</tbody></table></body></html>`);
    printWindow.document.close();
    printWindow.onload = () => printWindow.print();
  };

  // ===== 提交审批（DRAFT → PENDING_APPROVAL） =====
  const handleSubmitForApproval = async (row: ProductionOrder) => {
    try {
      await ElMessageBox.confirm(`确定提交生产订单 ${row.order_no} 进入审批吗？`, '提交审批确认', {
        type: 'warning',
      });
      await submitProductionOrder(row.id);
      ElMessage.success('已提交审批');
      await cb.refresh();
    } catch (error) {
      if (error !== 'cancel') {
        ElMessage.error((error as Error).message || '提交审批失败');
      }
    }
  };

  // ===== 审批（PENDING_APPROVAL → SCHEDULED / REJECTED） =====
  const handleApproveOrder = async (row: ProductionOrder, approved: boolean) => {
    let opinion: string | undefined;
    try {
      if (approved) {
        await ElMessageBox.confirm(`确定通过生产订单 ${row.order_no} 的审批吗？`, '审批确认', {
          type: 'warning',
        });
      } else {
        const { value } = await ElMessageBox.prompt('请输入驳回意见', `驳回 ${row.order_no}`, {
          type: 'warning',
          inputPattern: /\S+/,
          inputErrorMessage: '驳回意见不能为空',
        });
        opinion = value;
      }
      await approveProductionOrder(row.id, { approved, opinion });
      ElMessage.success(approved ? '审批通过，已排产' : '已驳回');
      await cb.refresh();
    } catch (error) {
      if (error !== 'cancel') {
        ElMessage.error((error as Error).message || '审批失败');
      }
    }
  };

  // ===== 汇报生产进度（IN_PROGRESS 态） =====
  const handleProgressReport = async (row: ProductionOrder) => {
    let completed = '';
    try {
      const r1 = await ElMessageBox.prompt('请输入本次完成数量', `汇报进度 ${row.order_no}`, {
        inputPattern: /^\d+(\.\d+)?$/,
        inputErrorMessage: '请输入有效数量',
      });
      completed = r1.value;
      const r2 = await ElMessageBox.prompt(
        '请输入次品数量（可留空为 0）',
        `汇报进度 ${row.order_no}`,
        {
          inputPattern: /^\d*$/,
          inputErrorMessage: '请输入有效数量',
        }
      );
      const defect = r2.value || '0';
      const r3 = await ElMessageBox.prompt('备注（可留空）', `汇报进度 ${row.order_no}`, {
        inputValidator: () => true,
      });
      await reportProductionProgress(row.id, {
        completed_quantity: Number(completed),
        defect_quantity: Number(defect),
        remark: r3.value || undefined,
      });
      ElMessage.success('进度已汇报');
      await cb.refresh();
    } catch (error) {
      if (error !== 'cancel') {
        ElMessage.error((error as Error).message || '进度汇报失败');
      }
    }
  };

  // ===== 查看操作日志（getProductionOrderLogs） =====
  const handleViewLogs = async (row: ProductionOrder) => {
    try {
      const res = await getProductionOrderLogs(row.id);
      const logs = res.data.logs;
      const text =
        logs.length === 0
          ? '暂无操作日志'
          : logs
              .map(
                l =>
                  `[${l.created_at ?? '-'}] ${l.operation_type ?? l.action} - ` +
                  `${l.username ?? i18n.global.t('production.audit.unknownOperator')}${l.description ? `：${l.description}` : ''}`
              )
              .join('\n');
      ElMessageBox.alert(text, `生产订单 ${row.order_no} 操作日志`, {
        customStyle: { whiteSpace: 'pre-wrap' },
      });
    } catch (error) {
      ElMessage.error((error as Error).message || '获取日志失败');
    }
  };

  // 使用 reactive 包装，访问字段时自动解包 ref
  return reactive({
    handleStatusChange,
    handleDelete,
    handleExport,
    handlePrint,
    handleSubmitForApproval,
    handleApproveOrder,
    handleProgressReport,
    handleViewLogs,
  });
}
