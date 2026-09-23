/**
 * usePi.ts - 采购验货核心 composable
 * 任务编号: P14 批 2 I-3 第 5 批（拆分原 purchase-inspection/index.vue）
 * 提供检验单列表 / 统计 / 分页 / 过滤 / 表单 / 详情 / 选项加载等核心方法
 * 业务流程（查询 / 重置 / 创建 / 编辑 / 查看 / 提交 / 完成）由 usePiProc 提供
 * 行为完全保持一致（仅结构重构）
 * 批次 286：tableData 接入 useTableApi，移除手写分页逻辑
 *
 * 注意：返回值使用 reactive({...}) 包装，父组件可直接访问字段（自动解包 ref）
 * 子组件通过 :model-value/@update:model-value 模式传入；不会修改 prop
 */
import { ref, reactive, watch } from 'vue';
import { ElMessage } from 'element-plus';
import { msg } from '@/utils/message';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import { type PurchaseInspection, type PurchaseInspectionItem } from '@/api/purchase-inspection';
import {
  getPurchaseReceiptList,
  getReceiptItems,
  type ReceiptItem,
  type PurchaseReceiptEntity,
} from '@/api/purchase-receipt';
import { getSupplierList } from '@/api/supplier';
import { useTableApi } from '@/composables/useTableApi';
import { logger } from '@/utils/logger';
import { i18n } from '@/i18n';

/**
 * 采购验货主业务 composable
 * 集中管理列表、统计、过滤、表单、详情、选项加载
 */
export function usePi() {
  // 统计数据（依据列表数据动态计算）
  const stats = reactive({
    total: 0,
    pending: 0,
    passed: 0,
    failed: 0,
  });

  // 日期范围（独立 ref，便于 PurchaseInspectionFilter 双向绑定；fetch 前注入 queryParams.inspection_date_from/to）
  const dateRange = ref<[Date, Date] | null>(null);

  // 列表数据接入 useTableApi
  // 采购验货 API 使用 snake_case 分页参数（page/page_size），匹配 useTableApi 默认配置
  const {
    data: tableData,
    total,
    loading,
    page,
    pageSize,
    queryParams,
    refresh: fetchData,
  } = useTableApi<PurchaseInspection>({
    url: '/purchase/inspections',
    defaultPageSize: 20,
    defaultParams: {
      keyword: '',
      supplier_id: undefined as number | undefined,
      status: '',
      result: '',
      inspection_date_from: '',
      inspection_date_to: '',
    },
    onError: (err: unknown) => {
      logger.error('获取数据失败:', err);
    },
  });

  // 监听列表/总数变化，同步统计字段（保持原 fetchData 中 stats 更新行为）
  watch(
    [tableData, total],
    () => {
      stats.total = total.value;
      stats.pending = tableData.value.filter(i => i.inspection_status === 'pending').length;
      stats.passed = tableData.value.filter(i => i.inspection_result === 'pass').length;
      stats.failed = tableData.value.filter(i => i.inspection_result === 'fail').length;
    },
    { deep: false }
  );

  // 选项
  const suppliers = ref<{ id: number; name: string }[]>([]);
  // receipts 用于创建质检单时选入库单；supplier_id 由入库单携带（CreatePurchaseInspectionRequest 实际必填 supplier_id）
  const receipts = ref<{ id: number; receipt_no: string; supplier_id: number }[]>([]);

  // 对话框
  const dialogVisible = ref(false);
  const isEdit = ref(false);
  const submitLoading = ref(false);
  const formData = reactive<{
    id?: number;
    receipt_id?: number;
    supplier_id?: number;
    inspection_date: string;
    remark: string;
    items: Partial<PurchaseInspectionItem>[];
  }>({
    id: undefined,
    receipt_id: undefined,
    supplier_id: undefined,
    inspection_date: '',
    remark: '',
    items: [],
  });

  const formRules = {
    receipt_id: [
      {
        required: true,
        message: i18n.global.t('purchaseInspection.validation.receiptRequired'),
        trigger: 'change',
      },
    ],
    inspection_date: [
      {
        required: true,
        message: i18n.global.t('purchaseInspection.validation.inspectionDateRequired'),
        trigger: 'change',
      },
    ],
  };

  // 详情对话框
  const detailDialogVisible = ref(false);
  const detailData = ref<PurchaseInspection>({} as PurchaseInspection);
  // 表头响应不含 items；handleView 异步加载明细后写入此 ref，PurchaseInspectionDetail.vue 读取此数组
  const detailItems = ref<PurchaseInspectionItem[]>([]);

  // 入库单明细加载状态
  const receiptItemsLoading = ref(false);

  /** 同步 dateRange 到 queryParams.inspection_date_from/to */
  const syncDateRangeToQuery = () => {
    if (dateRange.value) {
      queryParams.value = {
        ...queryParams.value,
        inspection_date_from: dateRange.value[0].toISOString(),
        inspection_date_to: dateRange.value[1].toISOString(),
      };
    } else {
      queryParams.value = {
        ...queryParams.value,
        inspection_date_from: '',
        inspection_date_to: '',
      };
    }
  };

  /**
   * 加载供应商列表（真实 API，替换原硬编码 mock 数组）
   * suppliers 用于 CreatePurchaseInspectionRequest.supplier_id 选择（后端实际必填，缺失则校验失败）
   */
  const fetchSuppliers = async () => {
    try {
      const res = await getSupplierList({ page: 1, page_size: 1000 });
      suppliers.value = res.data.items.map((s: { id: number; supplier_name: string }) => ({
        id: s.id,
        name: s.supplier_name,
      }));
    } catch (error) {
      logger.error('[purchase-inspection] 加载供应商列表失败', error);
    }
  };

  /**
   * 加载入库单列表（真实 API，替换原硬编码 mock 数组）
   */
  const fetchReceipts = async () => {
    try {
      const res = await getPurchaseReceiptList({ page: 1, page_size: 500 });
      receipts.value = res.data.items
        .filter((r): r is PurchaseReceiptEntity & { id: number } => r.id !== undefined)
        .map((r: PurchaseReceiptEntity & { id: number }) => ({
          id: r.id,
          receipt_no: r.receipt_no,
          supplier_id: r.supplier_id,
        }));
    } catch (error) {
      logger.error('[purchase-inspection] 加载入库单列表失败', error);
    }
  };

  /**
   * 入库单变化时：
   * 1. 从入库单派生 supplier_id（CreatePurchaseInspectionRequest 实际必填 supplier_id，缺失时后端拒绝）
   * 2. 加载入库单明细作为质检明细初始值
   */
  const handleReceiptChange = async (receiptId: number) => {
    if (!receiptId) {
      formData.items = [];
      formData.supplier_id = undefined;
      return;
    }
    // 从已加载的 receipts 列表中找到所选入库单的 supplier_id
    const receipt = receipts.value.find(r => r.id === receiptId);
    if (receipt) {
      formData.supplier_id = receipt.supplier_id;
    }
    receiptItemsLoading.value = true;
    try {
      const res = await getReceiptItems(receiptId);
      const items: ReceiptItem[] = res.data;
      if (items.length === 0) {
        msg.info('noReceiptDetails');
        formData.items = [];
        return;
      }
      // 将入库单明细映射为检验单明细，初始化各数量字段
      formData.items = items.map(item => ({
        product_id: item.product_id,
        // 入库明细行按后端契约是 material_code/material_name
        product_name: item.material_name,
        product_code: item.material_code,
        expected_quantity: item.quantity,
        inspected_quantity: 0,
        passed_quantity: 0,
        failed_quantity: 0,
        defect_reason: '',
      }));
    } catch (error) {
      const errMsg = error instanceof Error ? error.message : '获取入库单明细失败，请稍后重试';
      ElMessage.error(errMsg);
      logger.error('获取入库单明细失败:', error);
      formData.items = [];
    } finally {
      receiptItemsLoading.value = false;
    }
  };

  // 懒加载标记
  const hasLoaded = createLazyLoader();

  // 使用 reactive 包装，父组件可直接访问字段
  return reactive({
    // 统计
    stats,
    // 列表
    tableData,
    loading,
    total,
    dateRange,
    page,
    pageSize,
    queryParams,
    // 选项
    suppliers,
    receipts,
    // 表单对话框
    dialogVisible,
    isEdit,
    submitLoading,
    formData,
    formRules,
    // 详情对话框
    detailDialogVisible,
    detailData,
    detailItems,
    // 入库单明细加载
    receiptItemsLoading,
    // 加载方法
    fetchData,
    fetchSuppliers,
    fetchReceipts,
    handleReceiptChange,
    syncDateRangeToQuery,
    // 懒加载标记
    hasLoaded,
    // 兼容旧名（loadIfNot 接受字符串 key）
    loadIfNot,
  });
}
