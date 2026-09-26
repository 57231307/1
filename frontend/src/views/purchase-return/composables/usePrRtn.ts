/**
 * usePrRtn.ts - 采购退货核心 composable
 * 任务编号: P14 批 2 I-3 第 2 批（拆分原 purchase-return/index.vue）
 * 提供采购退货列表查询、表单管理、供应商/采购单/产品加载、CRUD 等核心方法
 * 业务流程（提交审批/审批/拒绝/删除）由 usePrRtnProc 提供
 * 批次 286：tableData 接入 useTableApi，移除手写分页逻辑
 */
import { ref, reactive, watch } from 'vue';
import { msg } from '@/utils/message';
import { getPurchaseOrderList, type PurchaseOrder } from '@/api/purchase';
import { getSupplierList } from '@/api/supplier';
import { getProductList, type Product } from '@/api/product';
import { getWarehouseList, type Warehouse } from '@/api/warehouse';
import {
  getPurchaseReturnById,
  getPurchaseReturnItemList,
  updatePurchaseReturn,
  createPurchaseReturn,
  createPurchaseReturnItem,
  updatePurchaseReturnItem,
  deletePurchaseReturnItem,
  type PurchaseReturn,
  type PurchaseReturnItem,
  type CreatePurchaseReturnPayload,
  type CreatePurchaseReturnItemPayload,
  type UpdatePurchaseReturnItemPayload,
} from '@/api/purchase-return';
import { useTableApi } from '@/composables/useTableApi';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import { logger } from '@/utils/logger';
import { i18n } from '@/i18n';
import { PURCHASE_RETURN_STATUS } from '@/utils/purchase-return-status';

/**
 * 退货明细表单行（编辑态本地结构）：提交时经 buildCreateItemPayload / buildUpdateItemPayload
 * 映射为后端 Create/UpdateReturnItemRequest，读取时经 normalizeItem 从后端 PurchaseReturnItemDto
 * 归一化而来，与响应契约解耦。
 */
export interface ReturnFormItem {
  id?: number;
  productId?: number;
  productName?: string;
  quantity: number;
  unitPrice?: number;
  reason?: string;
}

/**
 * 采购退货 composable
 * 集中管理列表、表单、供应商、采购单、产品、详情对话框的业务状态
 * 对话框可见性由父组件本地 ref 管理
 */
export function usePrRtn() {
  // 统计数据（依据列表数据动态计算）
  const stats = reactive({
    total: 0,
    pending: 0,
    approved: 0,
    amount: 0,
  });

  // 日期范围（独立 ref，便于 PurchaseReturnFilter 双向绑定；fetch 前注入 queryParams.startDate/endDate）
  const dateRange = ref<[Date, Date] | null>(null);

  // 列表数据接入 useTableApi
  // 分页/筛选参数键与后端 ReturnQueryParams 同名：page/page_size/status/supplier_id
  // keyword/start_date/end_date 为 schema gap（后端 ReturnQueryParams 不接受，Axum 静默丢弃）
  const {
    data: tableData,
    total,
    loading,
    page,
    pageSize,
    queryParams,
    refresh: fetchData,
  } = useTableApi<PurchaseReturn>({
    url: '/purchase/returns',
    listKey: 'items',
    defaultPageSize: 20,
    pageKey: 'page',
    defaultParams: {
      keyword: '',
      supplier_id: undefined as number | undefined,
      status: '',
      start_date: '',
      end_date: '',
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
      stats.pending = tableData.value.filter(
        i => i.return_status === PURCHASE_RETURN_STATUS.SUBMITTED
      ).length;
      stats.approved = tableData.value.filter(
        i => i.return_status === PURCHASE_RETURN_STATUS.APPROVED
      ).length;
      stats.amount = tableData.value.reduce((sum, i) => sum + (i.total_amount || 0), 0);
    },
    { deep: false }
  );

  // 供应商列表
  const suppliers = ref<{ id: number; name: string }[]>([]);

  // 采购订单列表
  const purchaseOrders = ref<{ id: number; order_no: string }[]>([]);

  // 产品列表
  const products = ref<{ id: number; name: string; price: number }[]>([]);

  // 仓库列表（用于 warehouse_id 选择，approval 时扣减库存需要该字段）
  const warehouses = ref<Warehouse[]>([]);

  // 表单数据
  const formData = reactive({
    id: undefined as number | undefined,
    purchaseOrderId: undefined as number | undefined,
    supplierId: undefined as number | undefined,
    returnDate: '',
    warehouseId: undefined as number | undefined,
    reasonType: '',
    reason: '',
    remarks: '',
    items: [] as ReturnFormItem[],
  });

  // 表单校验规则：supplier_id 与 reason_type 是后端 CreatePurchaseReturnRequest 的非 Option 字段，
  // 缺失会被 422 拒绝，故前端必须挡住；文案走 i18n（此前为裸中文字面量）。
  const formRules = {
    purchaseOrderId: [
      {
        required: true,
        message: i18n.global.t('purchaseReturn.validation.purchaseOrderRequired'),
        trigger: 'change',
      },
    ],
    returnDate: [
      {
        required: true,
        message: i18n.global.t('purchaseReturn.validation.returnDateRequired'),
        trigger: 'change',
      },
    ],
    supplierId: [
      {
        required: true,
        message: i18n.global.t('purchaseReturn.validation.supplierRequired'),
        trigger: 'change',
      },
    ],
    reasonType: [
      {
        required: true,
        message: i18n.global.t('purchaseReturn.validation.reasonTypeRequired'),
        trigger: 'change',
      },
    ],
    reason: [
      {
        required: true,
        message: i18n.global.t('purchaseReturn.validation.reasonRequired'),
        trigger: 'blur',
      },
    ],
  };

  // 详情数据（表头 + 明细，由 fetchDetail 回源填充）
  const detailData = ref<PurchaseReturn>({} as PurchaseReturn);
  const detailItems = ref<PurchaseReturnItem[]>([]);
  const detailItemsLoading = ref(false);

  // 懒加载标记
  const hasLoaded = createLazyLoader();

  /** 同步 dateRange 到 queryParams.start_date/end_date */
  const syncDateRangeToQuery = () => {
    if (dateRange.value) {
      queryParams.value = {
        ...queryParams.value,
        start_date: dateRange.value[0].toISOString(),
        end_date: dateRange.value[1].toISOString(),
      };
    } else {
      queryParams.value = {
        ...queryParams.value,
        start_date: '',
        end_date: '',
      };
    }
  };

  /** 加载供应商列表 */
  const fetchSuppliers = async () => {
    const res = await getSupplierList({ page: 1, page_size: 1000 });
    suppliers.value = (res.data?.items || []).map((s: { id: number; supplier_name: string }) => ({
      id: s.id,
      name: s.supplier_name,
    }));
  };

  /** 加载采购订单列表 */
  const fetchPurchaseOrders = async () => {
    const res = await getPurchaseOrderList({ page: 1, page_size: 500 });
    purchaseOrders.value = (res.data?.items || []) as unknown as PurchaseOrder[];
  };

  /** 加载产品列表 */
  const fetchProducts = async () => {
    const res = await getProductList({ page: 1, page_size: 1000 });
    const items = (res.data?.items ||
      (res.data as unknown as { list?: Product[] })?.list ||
      []) as Product[];
    products.value = items.map(p => ({
      id: p.id,
      name: p.product_name,
      price: 0,
    }));
  };

  /** 加载仓库列表（供 warehouse_id 选择，后端 CreatePurchaseReturnRequest 接受该字段） */
  const fetchWarehouses = async () => {
    const res = await getWarehouseList({ page: 1, page_size: 1000 });
    warehouses.value = res.data?.items || [];
  };

  /** 查询：先同步日期范围，重置页码，触发加载 */
  const handleQuery = () => {
    syncDateRangeToQuery();
    page.value = 1;
    fetchData();
  };

  /** 重置：清空筛选条件 + 日期 + 重置页码，触发加载 */
  const handleReset = () => {
    queryParams.value = {
      ...queryParams.value,
      keyword: '',
      supplier_id: undefined,
      status: '',
      start_date: '',
      end_date: '',
    };
    dateRange.value = null;
    page.value = 1;
    fetchData();
  };

  /** 日期范围变化：同步 dateRange 后立即查询 */
  const handleDateChange = (v: [Date, Date] | null) => {
    dateRange.value = v;
    handleQuery();
  };

  /** 准备新建表单（父组件需自行打开对话框） */
  const prepareCreate = () => {
    Object.assign(formData, {
      id: undefined,
      purchaseOrderId: undefined,
      supplierId: undefined,
      returnDate: '',
      warehouseId: undefined,
      reasonType: '',
      reason: '',
      remarks: '',
      items: [],
    });
  };

  /** 从后端 PurchaseReturnItemDto 归一化为表单本地结构 */
  const normalizeItem = (raw: PurchaseReturnItem): ReturnFormItem => ({
    id: raw.id,
    productId: raw.material_id,
    productName: raw.material_name ?? undefined,
    quantity: raw.quantity_returned,
    unitPrice: raw.unit_price,
    reason: raw.notes ?? undefined,
  });

  /**
   * 准备编辑表单（父组件需自行打开对话框）
   * 表头由调用方传入（来自 fetchDetail），明细由 loadFormItems 异步加载（独立端点，表头响应不含 items）
   */
  const prepareEdit = (row: PurchaseReturn) => {
    Object.assign(formData, {
      id: row.id,
      purchaseOrderId: row.order_id ?? undefined,
      supplierId: row.supplier_id,
      returnDate: row.return_date,
      warehouseId: row.warehouse_id ?? undefined,
      reasonType: row.reason_type ?? '',
      reason: row.reason_detail ?? '',
      remarks: row.notes ?? '',
      items: [],
    });
    removedItemIds.value = [];
    if (row.id) {
      loadFormItems(row.id).catch((error: unknown) => {
        logger.error('[purchase-return] 明细回填失败', error);
        msg.error('loadFailed');
      });
    }
  };

  /**
   * 异步回源编辑明细（表头响应不含 items，明细是独立端点）
   * 按 returnId 校验，避免快速切换行时把上一张单的明细写进当前表单
   */
  const loadFormItems = async (returnId: number) => {
    const res = await getPurchaseReturnItemList(returnId);
    if (formData.id !== returnId) return;
    formData.items = res.data.map(item => normalizeItem(item));
  };

  const fetchDetail = async (id: number) => {
    try {
      const res = await getPurchaseReturnById(id);
      detailData.value = res.data;
      detailItemsLoading.value = true;
      try {
        const itemsRes = await getPurchaseReturnItemList(id);
        detailItems.value = itemsRes.data;
      } catch (error) {
        logger.error('获取退货明细失败:', error);
        detailItems.value = [];
      } finally {
        detailItemsLoading.value = false;
      }
    } catch (error) {
      logger.error('获取详情失败:', error);
    }
  };

  /** 采购订单变化：派生供应商 */
  const handleOrderChange = (orderId: number) => {
    const order = purchaseOrders.value.find(o => o.id === orderId);
    if (order) {
      formData.supplierId = (order as unknown as { supplier_id?: number }).supplier_id;
      formData.items = [{ productId: undefined, productName: '', quantity: 1, unitPrice: 0 }];
    }
  };

  /** 添加明细 */
  const handleAddItem = () => {
    formData.items.push({
      productId: undefined,
      productName: '',
      quantity: 1,
      unitPrice: 0,
      reason: '',
    });
  };

  /** 删除明细（编辑态记录被删明细 ID，提交时走 deletePurchaseReturnItem） */
  const removedItemIds = ref<number[]>([]);
  const handleRemoveItem = (index: number) => {
    const removed = formData.items[index];
    if (formData.id && removed?.id) removedItemIds.value.push(removed.id);
    formData.items.splice(index, 1);
  };

  /** 产品变化（联动单价/名称） */
  const handleProductChange = (row: ReturnFormItem, productId: number) => {
    const product = products.value.find(p => p.id === productId);
    if (product) {
      row.productName = product.name;
      row.unitPrice = product.price;
    }
  };

  /** 构造新建明细请求体（对齐 CreatePurchaseReturnItemPayload） */
  const buildCreateItemPayload = (
    it: ReturnFormItem,
    idx: number
  ): CreatePurchaseReturnItemPayload => ({
    line_no: idx + 1,
    material_id: it.productId as number,
    quantity_returned: it.quantity ?? 0,
    unit_price: it.unitPrice ?? 0,
    notes: it.reason || undefined,
  });

  /** 构造更新明细请求体（对齐 UpdatePurchaseReturnItemPayload，仅发可变字段） */
  const buildUpdateItemPayload = (it: ReturnFormItem): UpdatePurchaseReturnItemPayload => ({
    quantity_returned: it.quantity ?? 0,
    unit_price: it.unitPrice ?? 0,
    notes: it.reason || undefined,
  });

  /**
   * 提交表单（新建/编辑）
   * 新建：显式构造 CreatePurchaseReturnPayload（不含 items），先建单再逐条写明细
   * 编辑：updatePurchaseReturn 仅接受 reason_type/reason_detail/notes（UpdatePurchaseReturnRequest），
   *       明细通过 item 级端点增量同步
   */
  const handleFormSubmit = async (isEdit: boolean): Promise<boolean> => {
    try {
      const validItems = formData.items.filter(e => (e.productId as number) > 0);
      if (validItems.length === 0) {
        msg.warning('pleaseAddReturnDetail');
        return false;
      }
      if (isEdit && formData.id) {
        await updatePurchaseReturn(formData.id, {
          reason_type: formData.reasonType,
          reason_detail: formData.reason,
          notes: formData.remarks || undefined,
        });
        let idx = 0;
        for (const it of validItems) {
          if (it.id) {
            await updatePurchaseReturnItem(formData.id, it.id, buildUpdateItemPayload(it));
          } else {
            await createPurchaseReturnItem(formData.id, buildCreateItemPayload(it, idx));
          }
          idx += 1;
        }
        for (const itemId of removedItemIds.value) {
          await deletePurchaseReturnItem(formData.id, itemId);
        }
        removedItemIds.value = [];
        msg.success('updateSuccess');
      } else {
        const headBody: CreatePurchaseReturnPayload = {
          order_id: formData.purchaseOrderId,
          supplier_id: formData.supplierId as number,
          return_date: formData.returnDate,
          warehouse_id: formData.warehouseId,
          reason_type: formData.reasonType,
          reason_detail: formData.reason || undefined,
          notes: formData.remarks || undefined,
        };
        const created = await createPurchaseReturn(headBody);
        const returnId = created.data?.id;
        if (returnId) {
          let idx = 0;
          for (const it of validItems) {
            await createPurchaseReturnItem(returnId, buildCreateItemPayload(it, idx));
            idx += 1;
          }
        }
        msg.success('createSuccess');
      }
      fetchData();
      return true;
    } catch (error) {
      logger.error('提交失败:', error);
      return false;
    }
  };

  /** 初始化加载（仅加载辅助数据，列表由 useTableApi setup 自动加载） */
  const initLoad = () => {
    loadIfNot('suppliers', fetchSuppliers, hasLoaded);
    loadIfNot('purchaseOrders', fetchPurchaseOrders, hasLoaded);
    loadIfNot('products', fetchProducts, hasLoaded);
    loadIfNot('warehouses', fetchWarehouses, hasLoaded);
  };

  // 使用 reactive 包装，访问字段时自动解包 ref
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
    handleQuery,
    handleReset,
    handleDateChange,
    fetchData,
    // 供应商/采购单/产品/仓库
    suppliers,
    purchaseOrders,
    products,
    warehouses,
    // 表单
    formData,
    formRules,
    prepareCreate,
    prepareEdit,
    handleOrderChange,
    handleAddItem,
    handleRemoveItem,
    handleProductChange,
    handleFormSubmit,
    // 详情
    detailData,
    detailItems,
    detailItemsLoading,
    fetchDetail,
    // 初始化
    initLoad,
  });
}
