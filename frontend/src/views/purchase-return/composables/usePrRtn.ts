/**
 * usePrRtn.ts - 采购退货核心 composable
 * 任务编号: P14 批 2 I-3 第 2 批（拆分原 purchase-return/index.vue）
 * 提供采购退货列表查询、表单管理、供应商/采购单/产品加载、CRUD 等核心方法
 * 业务流程（提交审批/审批/拒绝/删除）由 usePrRtnProc 提供
 * 行为完全保持一致（仅结构重构）
 * 批次 286：tableData 接入 useTableApi，移除手写分页逻辑
 */
import { ref, reactive, watch } from 'vue';
import { msg } from '@/utils/message';
import { getPurchaseOrderList, type PurchaseOrder } from '@/api/purchase';
import { getSupplierList } from '@/api/supplier';
import { getProductList, type Product } from '@/api/product';
import {
  getPurchaseReturnById,
  updatePurchaseReturn,
  createPurchaseReturn,
  createPurchaseReturnItem,
  updatePurchaseReturnItem,
  deletePurchaseReturnItem,
  type PurchaseReturn,
  type PurchaseReturnItem,
} from '@/api/purchase-return';
import { useTableApi } from '@/composables/useTableApi';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import { logger } from '@/utils/logger';
import { PURCHASE_RETURN_STATUS } from '@/utils/purchase-return-status';

/**
 * 退货明细表单行（编辑态本地结构）：提交时经 mapItemPayload 映射为后端 Create/UpdateReturnItemRequest，
 * 读取时经 normalizeItem 从后端 PurchaseReturnItemDto 归一化而来，与响应契约解耦。
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

  // 表单数据
  const formData = reactive({
    id: undefined as number | undefined,
    purchaseOrderId: undefined as number | undefined,
    supplierId: undefined as number | undefined,
    returnDate: '',
    reasonType: 'quality',
    reason: '',
    remarks: '',
    items: [] as ReturnFormItem[],
  });

  // 表单校验规则
  const formRules = {
    purchaseOrderId: [{ required: true, message: '请选择采购订单', trigger: 'change' }],
    returnDate: [{ required: true, message: '请选择退货日期', trigger: 'change' }],
    reason: [{ required: true, message: '请输入退货原因', trigger: 'blur' }],
  };

  // 详情数据
  const detailData = ref<PurchaseReturn>({} as PurchaseReturn);

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

  /** 加载供应商列表（修复假数据：改走真实供应商列表） */
  const fetchSuppliers = async () => {
    const res = await getSupplierList({ page: 1, page_size: 1000 });
    suppliers.value = (res.data?.items || []).map((s: { id: number; supplier_name: string }) => ({
      id: s.id,
      name: s.supplier_name,
    }));
  };

  /** 加载采购订单列表 */
  const fetchPurchaseOrders = async () => {
    // 修复假数据：原为硬编码数组，改走真实 /purchase/orders 列表
    const res = await getPurchaseOrderList({ page: 1, page_size: 500 });
    purchaseOrders.value = (res.data?.items || []) as unknown as PurchaseOrder[];
  };

  /** 加载产品列表（修复假数据：原为硬编码数组，改走真实产品列表） */
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
      returnDate: '',
      reason: '',
      remarks: '',
      items: [],
    });
  };

  /** 准备编辑表单（父组件需自行打开对话框） */
  const prepareEdit = (row: PurchaseReturn) => {
    Object.assign(formData, {
      id: row.id,
      purchaseOrderId: row.order_id ?? undefined,
      supplierId: row.supplier_id,
      returnDate: row.return_date,
      reasonType: row.reason_type || 'quality',
      reason: row.reason_detail ?? '',
      items: (row.items || []).map(it => normalizeItem(it)),
    });
  };

  /** 获取详情（后端 PurchaseReturnItemDto snake_case 归一化为表单本地结构，供编辑直接使用） */
  const normalizeItem = (raw: PurchaseReturnItem): ReturnFormItem => ({
    id: raw.id,
    productId: raw.material_id,
    productName: raw.material_name ?? undefined,
    quantity: raw.quantity_returned,
    unitPrice: raw.unit_price,
    reason: raw.notes ?? undefined,
  });

  const fetchDetail = async (id: number) => {
    try {
      const res = await getPurchaseReturnById(id);
      detailData.value = res.data;
    } catch (error) {
      logger.error('获取详情失败:', error);
    }
  };

  /** 采购订单变化（模拟加载明细） */
  const handleOrderChange = (orderId: number) => {
    // 修复假明细：原硬编码“产品A x10”，改为按所选采购单派生供应商，明细由用户自行添加
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

  /** 提交表单（新建/编辑；对齐后端契约：Create 含 order_id/supplier_id/reason_type，
   *  items 走 item 级端点；Update 仅 reason_type/reason_detail/notes，PUT 不含 items） */
  const mapItemPayload = (it: ReturnFormItem, idx: number) => ({
    line_no: idx + 1,
    material_id: it.productId as number,
    quantity_returned: it.quantity ?? 0,
    unit_price: it.unitPrice ?? 0,
    notes: it.reason || undefined,
  });

  const handleFormSubmit = async (isEdit: boolean): Promise<boolean> => {
    try {
      const validItems = formData.items.filter(e => (e.productId as number) > 0);
      if (validItems.length === 0) {
        msg.warning('pleaseAddReturnDetail');
        return false;
      }
      if (isEdit && formData.id) {
        // 编辑：表头 PUT 仅接受三个字段（原发驼峰全被后端忽略，假保存）；明细走 item 级端点
        await updatePurchaseReturn(formData.id, {
          reason_type: formData.reasonType,
          reason_detail: formData.reason,
          notes: formData.remarks || undefined,
        });
        let idx = 0;
        for (const it of validItems) {
          if (it.id) {
            await updatePurchaseReturnItem(formData.id, it.id, mapItemPayload(it, idx));
          } else {
            await createPurchaseReturnItem(formData.id, mapItemPayload(it, idx));
          }
          idx += 1;
        }
        for (const itemId of removedItemIds.value) {
          await deletePurchaseReturnItem(formData.id, itemId);
        }
        removedItemIds.value = [];
        msg.success('updateSuccess');
      } else {
        // 新建：CreatePurchaseReturnRequest 不含 items，先建单再逐条加明细
        const created = await createPurchaseReturn({
          order_id: formData.purchaseOrderId,
          supplier_id: formData.supplierId as number,
          return_date: formData.returnDate,
          reason_type: formData.reasonType,
          reason_detail: formData.reason || undefined,
          notes: formData.remarks || undefined,
        } as never);
        const returnId = (created.data as unknown as { id?: number })?.id;
        if (returnId) {
          let idx = 0;
          for (const it of validItems) {
            await createPurchaseReturnItem(returnId, mapItemPayload(it, idx));
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
    // 供应商/采购单/产品
    suppliers,
    purchaseOrders,
    products,
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
    fetchDetail,
    // 初始化
    initLoad,
  });
}
