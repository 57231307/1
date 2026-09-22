/**
 * usePurchList - 采购单列表与查询 composable
 * 任务编号: P13 批 1 B3 I-1（拆分 purchase/index.vue）
 */
import { ref, reactive } from 'vue';
import { ElMessage } from 'element-plus';
import { msg } from '@/utils/message';
import { getPurchaseOrderList, type PurchaseOrder } from '@/api/purchase';
import type { Supplier } from '@/api/supplier';
import type { Product } from '@/api/product';
import type { Warehouse } from '@/api/warehouse';
import { getSupplierList } from '@/api/supplier';
import { getProductList } from '@/api/product';
import { getWarehouseList } from '@/api/warehouse';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import { logger } from '@/utils/logger';
import { i18n } from '@/i18n';
import {
  PURCHASE_ORDER_STATUS,
  purchaseStatusLabelKey,
  purchaseStatusTagType,
  type PurchaseTagType,
} from '@/utils/purchase-status';

/**
 * 付款状态对应的 el-tag 类型与文本
 * 订单状态的词表与配色出自 utils/purchase-status（与后端 purchase_order.order_status 原值一致）
 */
const paymentTypeMap: Record<string, PurchaseTagType> = {
  unpaid: 'danger',
  partial: 'warning',
  paid: 'success',
};
const paymentTextMap: Record<string, string> = {
  unpaid: '未付款',
  partial: '部分付款',
  paid: '已付款',
};

/**
 * 采购单列表与查询 composable
 */
export function usePurchList() {
  const hasLoaded = createLazyLoader();

  const loading = ref(false);
  const orders = ref<PurchaseOrder[]>([]);
  const suppliers = ref<Supplier[]>([]);
  const products = ref<Product[]>([]);
  const warehouses = ref<Warehouse[]>([]);
  const total = ref(0);

  const stats = ref({
    monthOrders: 0,
    monthAmount: 0,
    pendingReceipt: 0,
    supplierCount: 0,
  });

  const queryParams = reactive({
    page: 1,
    page_size: 20,
    keyword: '',
    supplier_id: undefined as number | undefined,
    status: '',
  });

  /**
   * 格式化货币为人民币字符串
   */
  const formatCurrency = (amount: number) => {
    return new Intl.NumberFormat('zh-CN', {
      style: 'currency',
      currency: 'CNY',
      minimumFractionDigits: 0,
    }).format(amount);
  };

  /**
   * 订单状态对应的 el-tag 类型
   * 词表外的状态由 utils/purchase-status 抛错并记日志，不再回退配色掩盖数据异常
   */
  const getStatusType = (status: string): PurchaseTagType => purchaseStatusTagType(status);

  /**
   * 订单状态显示文案（i18n，键名即后端枚举原值）
   */
  const getStatusText = (status: string): string => i18n.global.t(purchaseStatusLabelKey(status));

  /**
   * 付款状态对应的 el-tag 类型
   * 注：采购订单列表/详情 DTO（后端 PurchaseOrderDto）不含 payment_status 字段，
   * 本列的取值来源尚未确定，词表暂维持现状
   */
  const getPaymentStatusType = (status: string): PurchaseTagType =>
    paymentTypeMap[status] ?? 'info';

  /**
   * 付款状态显示文本
   */
  const getPaymentStatusText = (status: string) => paymentTextMap[status] || status;

  /**
   * 获取采购单列表
   */
  const fetchData = async () => {
    loading.value = true;
    try {
      // 未选状态时必须省略该参数：后端 list_orders 用 OrderStatus.eq(status) 精确匹配
      // 且不做取值校验，提交 status=（空串）会筛出恒零结果
      const res = await getPurchaseOrderList({
        ...queryParams,
        status: queryParams.status || undefined,
      });
      // 兼容两种后端响应：data 为数组（ApiResponse<Vec<T>>）或
      // PaginatedResponse { items/list, total }（历史格式）
      const payload = res.data as unknown;
      const list = Array.isArray(payload)
        ? (payload as PurchaseOrder[])
        : ((payload as { items?: PurchaseOrder[] })?.items ??
          (payload as { list?: PurchaseOrder[] })?.list ??
          []);
      orders.value = list;
      total.value = Array.isArray(payload)
        ? list.length
        : (payload as { total?: number })?.total || list.length;

      // 计算统计数据
      stats.value.monthOrders = total.value;
      stats.value.monthAmount = orders.value.reduce((sum, o) => sum + (o.total_amount || 0), 0);
      // 待收货：后端语义为「已审批且尚未收货」的订单（收货后转 PARTIAL_RECEIVED/COMPLETED）
      stats.value.pendingReceipt = orders.value.filter(
        o => o.status === PURCHASE_ORDER_STATUS.APPROVED
      ).length;
    } catch (error: unknown) {
      ElMessage.error(
        (error instanceof Error ? error.message : '') ||
          msg.translate('loadPurchaseOrderListFailed')
      );
      orders.value = [];
      total.value = 0;
    } finally {
      loading.value = false;
    }
  };

  /**
   * 从列表响应提取数组：兼容 PaginatedResponse { items } 与历史 { list } 格式
   */
  const extractList = <T>(payload: unknown): T[] =>
    Array.isArray(payload)
      ? (payload as T[])
      : ((payload as { items?: T[] })?.items ?? (payload as { list?: T[] })?.list ?? []);

  /**
   * 获取供应商列表
   */
  const fetchSuppliers = async () => {
    try {
      const res = await getSupplierList({ page_size: 1000 });
      // 后端返回 PaginatedResponse { items }，兼容历史 list 格式
      suppliers.value = extractList<Supplier>(res.data);
      stats.value.supplierCount = suppliers.value.length;
    } catch (error) {
      logger.error('获取供应商列表失败:', error);
    }
  };

  /**
   * 获取产品列表
   */
  const fetchProducts = async () => {
    try {
      const res = await getProductList({ page_size: 1000 });
      products.value = extractList<Product>(res.data);
    } catch (error) {
      logger.error('获取产品列表失败:', error);
    }
  };

  /**
   * 获取仓库列表
   */
  const fetchWarehouses = async () => {
    try {
      const res = await getWarehouseList({ page_size: 1000 });
      warehouses.value = extractList<Warehouse>(res.data);
    } catch (error) {
      logger.error('获取仓库列表失败:', error);
    }
  };

  const handleQuery = () => {
    queryParams.page = 1;
    fetchData();
  };

  const handleReset = () => {
    queryParams.keyword = '';
    queryParams.supplier_id = undefined;
    queryParams.status = '';
    handleQuery();
  };

  /**
   * 初始化页面（按需懒加载数据）
   */
  const initPage = () => {
    loadIfNot('fetchData', fetchData, hasLoaded);
    loadIfNot('fetchSuppliers', fetchSuppliers, hasLoaded);
    loadIfNot('fetchProducts', fetchProducts, hasLoaded);
    loadIfNot('fetchWarehouses', fetchWarehouses, hasLoaded);
  };

  return {
    loading,
    orders,
    suppliers,
    products,
    warehouses,
    total,
    stats,
    queryParams,
    formatCurrency,
    getStatusType,
    getStatusText,
    getPaymentStatusType,
    getPaymentStatusText,
    fetchData,
    handleQuery,
    handleReset,
    initPage,
  };
}
