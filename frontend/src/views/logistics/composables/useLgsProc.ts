/**
 * useLgsProc.ts - 物流管理流程操作 composable
 * 任务编号: P14 批 2 I-3 第 4 批（拆分原 logistics/index.vue）
 * 封装创建 / 编辑 / 查看 / 发货 / 更新状态 / 删除等流程性方法
 * 行为完全保持一致（仅结构重构）
 *
 * 设计说明：通过 callbacks 接收 useLgs 的状态引用（Reactive 包装层）；
 * 内部访问 cb.isEdit.value 等即可修改实际 ref 的 value
 */
import { reactive, computed } from 'vue';
import { ElMessageBox } from 'element-plus';
import { msg } from '@/utils/message';
import { i18n } from '@/i18n';
import {
  getLogisticsById,
  signWaybill,
  updateLogistics,
  createLogistics,
  deleteLogistics,
  type LogisticsWaybill,
  type UpdateWaybillPayload,
} from '@/api/logistics';
import { WAYBILL_STATUS, type WaybillStatus } from '@/constants/waybill-status';
import { logger } from '@/utils/logger';

/**
 * 订单表单字段类型
 */
interface LgsFormData {
  id?: number | undefined;
  order_id?: number | undefined;
  logistics_company?: string;
  tracking_number?: string;
  driver_name?: string;
  driver_phone?: string;
  freight_fee?: number;
  expected_arrival?: string;
  notes?: string;
}

/**
 * 状态表单字段类型
 */
export interface LgsStatusForm {
  id: number;
  currentStatus: WaybillStatus | '';
  newStatus: WaybillStatus | '';
}

/**
 * 流程回调（接收 useLgs 返回的状态）
 * 由于 useLgs 返回 reactive({...})，父组件传入 lgs.dialogVisible 等会自动解包为值；
 * 因此回调使用 plain 类型，useLgsProc 内部通过 cb.isEdit = newValue 修改（reactive 会更新底层 ref）
 */
interface LgsCallbacks {
  // 详情对话框
  detailDialogVisible: boolean;
  // 详情数据
  detailData: LogisticsWaybill;
  // 表单
  isEdit: boolean;
  formData: LgsFormData;
  submitLoading: boolean;
  dialogVisible: boolean;
  // 状态表单
  statusForm: LgsStatusForm;
  statusDialogVisible: boolean;
  // 列表刷新
  fetchData: () => Promise<void>;
}

/**
 * 物流管理流程操作方法集合
 */
export function useLgsProc(cb: LgsCallbacks) {
  /** 打开新建对话框 */
  const handleCreate = () => {
    cb.isEdit = false;
    Object.assign(cb.formData, {
      id: undefined,
      order_id: undefined,
      logistics_company: '',
      tracking_number: '',
      driver_name: '',
      driver_phone: '',
      freight_fee: 0,
      expected_arrival: '',
      notes: '',
    });
    cb.dialogVisible = true;
  };

  /** 打开编辑对话框 */
  const handleEdit = (row: LogisticsWaybill) => {
    cb.isEdit = true;
    Object.assign(cb.formData, {
      id: row.id,
      order_id: row.order_id,
      logistics_company: row.logistics_company,
      tracking_number: row.tracking_number,
      driver_name: row.driver_name,
      driver_phone: row.driver_phone,
      freight_fee: row.freight_fee,
      expected_arrival: row.expected_arrival,
      notes: row.notes,
    });
    cb.dialogVisible = true;
  };

  /** 查看详情 */
  const handleView = async (row: LogisticsWaybill) => {
    try {
      const res = await getLogisticsById(row.id!);
      cb.detailData = res.data;
      cb.detailDialogVisible = true;
    } catch (error) {
      logger.error('获取详情失败:', error);
    }
  };

  /**
   * 提交表单（仅 API 调用，校验已由 LogisticsForm 内部完成）
   */
  const handleSubmit = async () => {
    cb.submitLoading = true;
    try {
      if (cb.isEdit && cb.formData.id) {
        // 关联订单不随编辑变更：运单与订单的归属在建单时确定，改派需重建运单
        const payload: UpdateWaybillPayload = {
          logistics_company: cb.formData.logistics_company,
          tracking_number: cb.formData.tracking_number,
          driver_name: cb.formData.driver_name,
          driver_phone: cb.formData.driver_phone,
          freight_fee: cb.formData.freight_fee,
          expected_arrival: cb.formData.expected_arrival,
          notes: cb.formData.notes,
        };
        await updateLogistics(cb.formData.id, payload);
        msg.success('updateSuccess');
      } else {
        await createLogistics(cb.formData);
        msg.success('createSuccess');
      }
      cb.dialogVisible = false;
      await cb.fetchData();
    } catch (error) {
      logger.error('提交失败:', error);
    } finally {
      cb.submitLoading = false;
    }
  };

  /** 打开更新状态对话框 */
  const handleUpdateStatus = (row: LogisticsWaybill) => {
    cb.statusForm.id = row.id!;
    cb.statusForm.currentStatus = row.status;
    cb.statusForm.newStatus = '';
    cb.statusDialogVisible = true;
  };

  /** 提交状态更新 */
  const handleStatusSubmit = async () => {
    try {
      await updateLogistics(cb.statusForm.id, { status: cb.statusForm.newStatus as WaybillStatus });
      msg.success('statusUpdateSuccess');
      cb.statusDialogVisible = false;
      await cb.fetchData();
    } catch (error) {
      logger.error('状态更新失败:', error);
    }
  };

  /**
   * 电子签收：DELIVERED → SIGNED。
   * 后端在同一事务内写入签收人/签收时间，并把关联销售订单的应收发票推进为已确认。
   */
  const handleSign = async (row: LogisticsWaybill) => {
    try {
      await ElMessageBox.confirm(
        i18n.global.t('logistics.confirm.sign'),
        i18n.global.t('logistics.confirm.title'),
        { type: 'warning' }
      );
      await signWaybill(row.id!);
      msg.success('signSuccess');
      await cb.fetchData();
    } catch (error) {
      if (error !== 'cancel') {
        logger.error('签收失败:', error);
      }
    }
  };

  /** 删除 */
  const handleDelete = async (row: LogisticsWaybill) => {
    try {
      await ElMessageBox.confirm(
        i18n.global.t('logistics.confirm.delete'),
        i18n.global.t('logistics.confirm.title'),
        { type: 'warning' }
      );
      await deleteLogistics(row.id!);
      msg.success('deleteSuccess');
      await cb.fetchData();
    } catch (error) {
      if (error !== 'cancel') {
        logger.error('删除失败:', error);
      }
    }
  };

  /** 可选新状态映射（根据当前状态）：状态机内运输中只允许推进到已送达 */
  const availableStatuses = computed(() => {
    const map: Record<string, { label: string; value: WaybillStatus }[]> = {
      [WAYBILL_STATUS.inTransit]: [
        {
          label: i18n.global.t('logistics.common.status.delivered'),
          value: WAYBILL_STATUS.delivered,
        },
      ],
    };
    const options = map[cb.statusForm.currentStatus];
    if (!options) {
      logger.warn(`[useLgsProc] 状态「${cb.statusForm.currentStatus}」没有可选的推进目标`);
      return [];
    }
    return options;
  });

  // 使用 reactive 包装，访问字段时自动解包 ref
  return reactive({
    handleCreate,
    handleEdit,
    handleView,
    handleSubmit,
    handleUpdateStatus,
    handleStatusSubmit,
    handleSign,
    handleDelete,
    availableStatuses,
  });
}
