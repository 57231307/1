/**
 * usePiProc.ts - 采购验货流程操作 composable
 * 任务编号: P14 批 2 I-3 第 5 批（拆分原 purchase-inspection/index.vue）
 * 封装查询 / 重置 / 创建 / 编辑 / 查看 / 提交 / 完成等流程性方法
 * 批次 286：适配 useTableApi（queryParams 放宽为 Record<string, unknown>，page 独立字段）
 *
 * 设计说明：通过 callbacks 接收 usePi 的状态引用（Reactive 包装层）
 */
import { ElMessage, ElMessageBox } from 'element-plus';
import { msg } from '@/utils/message';
import { i18n } from '@/i18n';
import {
  getPurchaseInspectionById,
  getPurchaseInspectionItemList,
  updatePurchaseInspection,
  createPurchaseInspection,
  completePurchaseInspection,
  type PurchaseInspection,
  type PurchaseInspectionItem,
  type CreatePurchaseInspectionPayload,
  type UpdatePurchaseInspectionPayload,
} from '@/api/purchase-inspection';
import { logger } from '@/utils/logger';

/** 取已翻译文案（i18n 全局实例，composable 无组件实例，同域 usePcProc 一致做法）。 */
const t = i18n.global.t.bind(i18n.global);

/**
 * 流程回调（接收 usePi 返回的状态，自动解包后的值类型）
 */
interface PiCallbacks {
  // 列表
  tableData: PurchaseInspection[];
  loading: boolean;
  total: number;
  dateRange: [Date, Date] | null;
  // 查询参数（放宽为 Record 兼容 useTableApi 的 queryParams 类型）
  queryParams: Record<string, unknown>;
  // 分页（useTableApi 独立字段）
  page: number;
  pageSize: number;
  // 选项
  suppliers: { id: number; name: string }[];
  receipts: { id: number; receipt_no: string }[];
  // 表单
  dialogVisible: boolean;
  isEdit: boolean;
  submitLoading: boolean;
  formData: {
    id?: number;
    receipt_id?: number;
    supplier_id?: number;
    inspection_date: string;
    remark: string;
    items: Partial<PurchaseInspectionItem>[];
  };
  // 详情
  detailDialogVisible: boolean;
  detailData: PurchaseInspection;
  detailItems: PurchaseInspectionItem[];
  // 方法
  fetchData: () => Promise<void>;
  handleReceiptChange: (receiptId: number) => Promise<void>;
  syncDateRangeToQuery: () => void;
}

/**
 * 采购验货流程操作方法集合
 */
export function usePiProc(cb: PiCallbacks) {
  /** 查询：先同步日期范围，重置页码，触发加载 */
  const handleQuery = () => {
    cb.syncDateRangeToQuery();
    cb.page = 1;
    cb.fetchData();
  };

  /** 重置：清空筛选条件 + 日期 + 重置页码，触发加载 */
  const handleReset = () => {
    cb.queryParams = {
      ...cb.queryParams,
      keyword: '',
      supplier_id: undefined,
      status: '',
      result: '',
      inspection_date_from: '',
      inspection_date_to: '',
    };
    cb.dateRange = null;
    cb.page = 1;
    cb.fetchData();
  };

  /** 创建检验单 */
  const handleCreate = () => {
    cb.isEdit = false;
    Object.assign(cb.formData, {
      id: undefined,
      receipt_id: undefined,
      supplier_id: undefined,
      inspection_date: '',
      remark: '',
      items: [],
    });
    cb.dialogVisible = true;
  };

  /**
   * 编辑检验单
   * 表头来自行数据（useTableApi 列表返回真实 Model），明细从 /inspections/{id}/items 异步回源
   */
  const handleEdit = async (row: PurchaseInspection) => {
    cb.isEdit = true;
    Object.assign(cb.formData, {
      id: row.id,
      receipt_id: row.receipt_id ?? undefined,
      supplier_id: row.supplier_id,
      inspection_date: row.inspection_date,
      remark: row.notes ?? '',
      items: [],
    });
    cb.dialogVisible = true;
    if (row.id) {
      try {
        const itemsRes = await getPurchaseInspectionItemList(row.id);
        if (cb.formData.id === row.id) {
          cb.formData.items = itemsRes.data.items;
        }
      } catch (error) {
        logger.error('[purchase-inspection] 明细回填失败', error);
      }
    }
  };

  /**
   * 查看详情
   * 表头由 get_inspection 获取；明细由 /inspections/{id}/items 异步回源（表头响应不含 items）
   */
  const handleView = async (row: PurchaseInspection) => {
    try {
      const res = await getPurchaseInspectionById(row.id!);
      cb.detailData = res.data;
      cb.detailItems = [];
      cb.detailDialogVisible = true;
      try {
        const itemsRes = await getPurchaseInspectionItemList(row.id!);
        cb.detailItems = itemsRes.data.items;
      } catch (error) {
        logger.error('[purchase-inspection] 详情明细加载失败', error);
      }
    } catch (error) {
      logger.error('获取详情失败:', error);
    }
  };

  /**
   * 提交表单（创建/更新）
   * 新建：构造 CreatePurchaseInspectionPayload（对齐后端 CreatePurchaseInspectionRequest），
   *       remark 键映射为后端的 notes；不含 items（明细走 item 级端点）
   * 编辑：构造 UpdatePurchaseInspectionPayload（对齐后端 UpdatePurchaseInspectionRequest），
   *       仅可更新 sample_size/defect_description/notes
   */
  const handleSubmit = async () => {
    try {
      if (cb.isEdit && cb.formData.id) {
        const updatePayload: UpdatePurchaseInspectionPayload = {
          notes: cb.formData.remark || undefined,
        };
        await updatePurchaseInspection(cb.formData.id, updatePayload);
        msg.success('updateSuccess');
      } else {
        // supplier_id 由所选入库单派生，而后端 create_inspection 在其缺失时直接 validation 拒绝。
        // 取不到就是入库单数据不完整，显式报错并中止提交，不能静默发出一个必然失败的请求。
        if (cb.formData.supplier_id === undefined) {
          ElMessage.error(t('purchaseInspection.message.supplierNotDerived'));
          logger.error('质检单创建中止：所选入库单未带出 supplier_id', {
            receipt_id: cb.formData.receipt_id,
          });
          return;
        }
        const createPayload: CreatePurchaseInspectionPayload = {
          receipt_id: cb.formData.receipt_id,
          supplier_id: cb.formData.supplier_id,
          inspection_date: cb.formData.inspection_date || undefined,
          notes: cb.formData.remark || undefined,
        };
        await createPurchaseInspection(createPayload);
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

  /** 提交前的加载状态开启（父组件调用） */
  const handleBeforeSubmit = () => {
    cb.submitLoading = true;
  };

  /** 完成检验 */
  const handleComplete = async (row: PurchaseInspection) => {
    try {
      const { value: passInput } = await ElMessageBox.prompt(
        t('purchaseInspection.complete.passQuantityTip'),
        t('purchaseInspection.complete.title'),
        {
          confirmButtonText: t('common.confirm'),
          cancelButtonText: t('common.cancel'),
          inputPlaceholder: t('purchaseInspection.complete.passQuantityPlaceholder'),
          inputPattern: /^\d+(\.\d+)?$/,
          inputErrorMessage: t('purchaseInspection.complete.numberError'),
        }
      );
      const { value: rejectInput } = await ElMessageBox.prompt(
        t('purchaseInspection.complete.rejectQuantityTip'),
        t('purchaseInspection.complete.title'),
        {
          confirmButtonText: t('common.confirm'),
          cancelButtonText: t('common.cancel'),
          inputPlaceholder: t('purchaseInspection.complete.rejectQuantityPlaceholder'),
          inputPattern: /^\d+(\.\d+)?$/,
          inputErrorMessage: t('purchaseInspection.complete.numberError'),
        }
      );
      const { value: resultInput } = await ElMessageBox.prompt(
        t('purchaseInspection.complete.resultTip'),
        t('purchaseInspection.complete.title'),
        {
          confirmButtonText: t('common.confirm'),
          cancelButtonText: t('common.cancel'),
          inputPlaceholder: t('purchaseInspection.complete.resultPlaceholder'),
          inputPattern: /^(pass|fail|partial)$/,
          inputErrorMessage: t('purchaseInspection.complete.resultError'),
        }
      );
      await completePurchaseInspection(row.id!, {
        pass_quantity: Number(passInput),
        reject_quantity: Number(rejectInput),
        inspection_result: resultInput,
      });
      msg.success('operationSuccess');
      await cb.fetchData();
    } catch (error) {
      if (error !== 'cancel') {
        logger.error('操作失败:', error);
      }
    }
  };

  // 使用 reactive 包装，访问字段时自动解包 ref
  return {
    handleQuery,
    handleReset,
    handleCreate,
    handleEdit,
    handleView,
    handleSubmit,
    handleBeforeSubmit,
    handleComplete,
  };
}
