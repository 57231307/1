/**
 * useVchrLstProc.ts - 凭证列表流程操作 composable
 * 任务编号: P14 批 2 I-3 第 1 批（拆分原 VoucherListTab.vue）
 * 封装凭证打印、导出、审核、记账、反记账、删除等流程性方法
 * 行为完全保持一致（仅结构重构）
 */
import { ElMessageBox } from 'element-plus';
import { msg } from '@/utils/message';
import printJS from 'print-js';
import {
  deleteVoucher,
  approveVoucher,
  postVoucher,
  unpostVoucher,
  type VoucherEntity,
} from '@/api/voucher';
import { getStatusLabel, getTypeLabel } from './vchrLstFmts';
import { exportFromBackend } from '@/utils/export';

/** 接收的列表数据（支持 ref 和 plain value） */
type ContractListLike = { value: VoucherEntity[] } | VoucherEntity[];

/**
 * 创建凭证流程操作方法集合
 * @param tableData 列表 ref 或 plain value
 * @param loadData 重新拉取列表方法
 */
export function useVchrLstProc(tableData: ContractListLike, loadData: () => Promise<void>) {
  /** 取出底层数组（兼容 ref 和 plain value） */
  const getList = (): VoucherEntity[] => {
    return Array.isArray(tableData) ? tableData : tableData.value;
  };

  /** 打印当前列表 */
  const handlePrint = () => {
    const list = getList();
    const printData = list.map((item, index) => ({
      序号: index + 1,
      凭证号: item.voucher_no,
      日期: item.voucher_date,
      类型: getTypeLabel(item.type),
      摘要: item.description || '-',
      借方金额: `¥${item.total_debit}`,
      贷方金额: `¥${item.total_credit}`,
      状态: getStatusLabel(item.status),
    }));
    printJS({
      printable: printData,
      properties: Object.keys(printData[0] || {}) as string[],
      type: 'json',
      header: '会计凭证列表',
      style: 'padding: 20px; font-size: 14px;',
      headerStyle: 'font-size: 18px; font-weight: bold; margin-bottom: 20px;',
      gridHeaderStyle: 'font-weight: bold; background-color: #f5f7fa;',
      gridStyle: 'border-collapse: collapse; width: 100%;',
    } as never);
  };

  /** 导出 Excel（规则 3：禁止 CSV 作为最终交付格式） */
  const handleExport = () => {
    exportFromBackend('/gl/vouchers/export', {}, '会计凭证');
  };

  /** 删除凭证 */
  const handleDelete = async (row: VoucherEntity) => {
    if (row.status === 'posted') {
      msg.warning('postedVoucherCannotDelete');
      return;
    }
    try {
      await ElMessageBox.confirm('确定要删除这个凭证吗？', '提示', {
        type: 'warning',
      });
      await deleteVoucher(row.id!);
      msg.success('deleteSuccess');
      await loadData();
    } catch (error) {
      msg.info('deleteCancelled');
    }
  };

  /** 审核凭证 */
  const handleApprove = async (row: VoucherEntity) => {
    try {
      await ElMessageBox.confirm('确定要审核这个凭证吗？', '提示', {
        type: 'warning',
      });
      await approveVoucher(row.id!);
      msg.success('auditSuccess');
      await loadData();
    } catch (error) {
      msg.info('operationCancelled');
    }
  };

  /** 记账凭证 */
  const handlePost = async (row: VoucherEntity) => {
    try {
      await ElMessageBox.confirm('确定要记账这个凭证吗？', '提示', {
        type: 'warning',
      });
      await postVoucher(row.id!);
      msg.success('bookSuccess');
      await loadData();
    } catch (error) {
      msg.info('operationCancelled');
    }
  };

  /** 反记账 */
  const handleUnpost = async (row: VoucherEntity) => {
    try {
      await ElMessageBox.confirm('确定要反记账这个凭证吗？', '提示', {
        type: 'warning',
      });
      await unpostVoucher(row.id!);
      msg.success('unbookSuccess');
      await loadData();
    } catch (error) {
      msg.info('operationCancelled');
    }
  };

  return {
    handlePrint,
    handleExport,
    handleDelete,
    handleApprove,
    handlePost,
    handleUnpost,
  };
}
