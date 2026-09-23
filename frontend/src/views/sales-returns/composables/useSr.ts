/**
 * useSr.ts - 销售退货核心 composable
 * 任务编号: P14 批 2 I-3 第 7 批（拆分原 sales-returns/index.vue）
 * 提供退货单列表查询、表单管理、销售订单/客户/产品加载、CRUD 等核心方法
 * 审批流程由 useSrProc 提供
 * 行为完全保持一致（仅结构重构）
 */
import { ref, reactive } from 'vue';
import type { FormInstance } from 'element-plus';
import { ElMessage } from 'element-plus';
import { msg } from '@/utils/message';
import {
  getSalesReturnList,
  getSalesReturnItemList,
  createSalesReturn,
  updateSalesReturn,
  createSalesReturnItem,
  updateSalesReturnItem,
  deleteSalesReturnItem,
  type SalesReturn,
  type SalesReturnQueryParams,
  type CreateSalesReturnRequest,
  type CreateSalesReturnItemRequest,
} from '@/api/sales-return';
import { getSalesOrderList } from '@/api/sales';
import { getCustomerList } from '@/api/customer';
import { getProductList } from '@/api/product';
import logger from '@/utils/logger';

/**
 * 退货原因分类候选。value 为落库的业务数据（后端 reason_type 字段），故使用字面量；
 * labelKey 指向 i18n（salesReturns.editDialog.<labelKey>），展示文案走 i18n。
 */
export const RETURN_REASON_OPTIONS: { value: string; labelKey: string }[] = [
  { value: '色差', labelKey: 'reasonTypeColorDifference' },
  { value: '缸差', labelKey: 'reasonTypeDyeLotDifference' },
  { value: '克重不符', labelKey: 'reasonTypeGramWeightMismatch' },
  { value: '幅宽不符', labelKey: 'reasonTypeWidthMismatch' },
  { value: '品质瑕疵', labelKey: 'reasonTypeQualityDefect' },
  { value: '数量不符', labelKey: 'reasonTypeQuantityMismatch' },
  { value: '发错货', labelKey: 'reasonTypeWrongShipment' },
  { value: '客户取消订单', labelKey: 'reasonTypeCustomerCancel' },
];

// v11 批次 163 P2-1 修复：定义具体类型替代 any
// v11 批次 174 P2-1 修复：导出接口供 ReturnEditDialog 使用
export interface SalesOrderOption {
  id: number;
  order_no: string;
  customer_id: number;
  customer_name: string;
  items?: Array<{
    product_id: number;
    product_name: string;
    product_code: string;
    unit_price: number;
  }>;
}

export interface CustomerOption {
  id: number;
  name: string;
  [key: string]: unknown;
}

export interface ProductOption {
  id: number;
  name: string;
  [key: string]: unknown;
}

export interface ReturnFormItem {
  id: number | null;
  productId: number | null;
  productName: string | null;
  productCode: string | null;
  quantity: number;
  unitPrice: number;
  taxPercent?: number;
  discountPercent?: number;
  amount: number;
  reason: string;
}

export interface ReturnForm {
  id: number | null;
  salesOrderId: number | null;
  salesOrderNo: string;
  customerId: number | null;
  customerName: string;
  returnDate: string;
  warehouseId: number | null;
  reasonType: string;
  reasonDetail: string;
  remarks: string;
  items: ReturnFormItem[];
  totalAmount: number;
  status: string;
}

/**
 * 销售退货 composable
 * 集中管理退货列表、表单、销售订单/客户/产品、对话框的业务状态
 */
export function useSr() {
  // 列表 loading
  const loading = ref(false);

  // v11 批次 163 P2-1 修复：any[] 改为具体类型 SalesReturn[]
  const returnList = ref<SalesReturn[]>([]);

  // 分页总数（来自后端 PaginatedResponse.total）
  const total = ref(0);

  // 列表查询状态：字段名与后端 SalesReturnQueryParams 逐一对应（snake_case）。
  // 空串/未选的筛选项由 request.ts serializeParams 双侧剔除，无需手工剥离。
  const queryParams = reactive({
    return_no: '',
    status: '',
    customer_id: undefined as number | undefined,
    page: 1,
    page_size: 20,
  });

  // 详情弹窗当前记录
  const currentReturn = ref<SalesReturn | null>(null);

  // 销售订单/客户/产品下拉数据
  const salesOrderList = ref<SalesOrderOption[]>([]);
  const customerList = ref<CustomerOption[]>([]);
  const productList = ref<ProductOption[]>([]);

  // 表单数据
  const formData = reactive<ReturnForm>({
    id: null,
    salesOrderId: null,
    salesOrderNo: '',
    customerId: null,
    customerName: '',
    returnDate: '',
    warehouseId: null,
    reasonType: '',
    reasonDetail: '',
    remarks: '',
    items: [],
    totalAmount: 0,
    status: 'DRAFT',
  });

  // 表单引用
  const formRef = ref<FormInstance>();

  // 加载退货列表：把真实查询/分页状态透传给后端，并直读 PaginatedResponse 的 items/total
  const loadReturns = async () => {
    loading.value = true;
    const params: SalesReturnQueryParams = {
      return_no: queryParams.return_no,
      status: queryParams.status,
      customer_id: queryParams.customer_id,
      page: queryParams.page,
      page_size: queryParams.page_size,
    };
    try {
      const res = await getSalesReturnList(params);
      returnList.value = res.data.items;
      total.value = res.data.total;
      logger.info(
        `[sales-return] 列表加载成功 page=${queryParams.page} 返回 ${res.data.items.length} 行 / 共 ${res.data.total} 行`
      );
    } catch (error: unknown) {
      // v11 批次 163 P2-1 修复：catch (error: any) 改为 unknown + 类型守卫
      const errMsg = error instanceof Error ? error.message : String(error);
      logger.error('[sales-return] 列表加载失败', errMsg);
      ElMessage.error(errMsg || msg.translate('loadReturnListFailed'));
    } finally {
      loading.value = false;
    }
  };

  // 查询：条件变更后回到首页并重新加载
  const handleSearch = () => {
    queryParams.page = 1;
    loadReturns();
  };

  // 重置：清空筛选条件并回到首页重新加载（保留每页条数选择）
  const handleReset = () => {
    queryParams.return_no = '';
    queryParams.status = '';
    queryParams.customer_id = undefined;
    queryParams.page = 1;
    loadReturns();
  };

  // 翻页：页码变化即重新加载
  const handlePageChange = () => {
    loadReturns();
  };

  // 每页条数变化：回到首页重新加载
  const handleSizeChange = () => {
    queryParams.page = 1;
    loadReturns();
  };

  // 加载销售订单下拉
  const loadSalesOrders = async () => {
    try {
      const res = await getSalesOrderList({ status: 'completed' });
      salesOrderList.value = (res.data?.items || []) as unknown as SalesOrderOption[];
    } catch (error: unknown) {
      logger.error('加载销售订单失败', error instanceof Error ? error.message : String(error));
    }
  };

  // 加载客户下拉
  const loadCustomers = async () => {
    try {
      const res = await getCustomerList();
      customerList.value = (res.data?.items || []) as unknown as CustomerOption[];
    } catch (error: unknown) {
      logger.error('加载客户列表失败', error instanceof Error ? error.message : String(error));
    }
  };

  // 加载产品下拉
  const loadProducts = async () => {
    try {
      const res = await getProductList();
      productList.value = (res.data?.items || []) as unknown as ProductOption[];
    } catch (error: unknown) {
      logger.error('加载产品列表失败', error instanceof Error ? error.message : String(error));
    }
  };

  // 重置表单为新建态
  const resetFormForCreate = () => {
    removedItemIds.value = [];
    Object.assign(formData, {
      id: null,
      salesOrderId: null,
      salesOrderNo: '',
      customerId: null,
      customerName: '',
      returnDate: new Date().toISOString().split('T')[0],
      warehouseId: null,
      reasonType: '',
      reasonDetail: '',
      remarks: '',
      items: [
        {
          id: null,
          productId: null,
          productName: '',
          productCode: '',
          quantity: 1,
          unitPrice: 0,
          taxPercent: undefined,
          discountPercent: undefined,
          amount: 0,
          reason: '',
        },
      ],
      totalAmount: 0,
      status: 'DRAFT',
    });
  };

  // 编辑态回填明细行：明细是独立端点，表头响应不含 items
  // 异步返回后按 returnId 校验，避免快速切换行时把上一张单的明细写进当前表单
  const loadFormItems = async (returnId: number) => {
    const res = await getSalesReturnItemList(returnId);
    if (formData.id !== returnId) return;
    formData.items = res.data.map(item => ({
      id: item.id,
      productId: item.product_id,
      productName: item.product_name,
      productCode: item.product_code,
      quantity: Number(item.quantity),
      unitPrice: Number(item.unit_price),
      taxPercent: Number(item.tax_percent),
      discountPercent: Number(item.discount_percent),
      amount: Number(item.total_amount),
      reason: item.notes ?? '',
    }));
    calculateTotal();
  };

  // 用行数据填充表单为编辑态
  // 表头 reason 由后端以 "{reason_type}: {reason_detail}" 组合存储，编辑回填时按首个 ": " 拆分回两字段
  const fillFormForEdit = (row: SalesReturn) => {
    const rawReason = row.reason ?? '';
    const sep = rawReason.indexOf(': ');
    Object.assign(formData, {
      id: row.id ?? null,
      salesOrderId: row.sales_order_id ?? null,
      salesOrderNo: row.sales_order_no ?? '',
      customerId: row.customer_id ?? null,
      customerName: row.customer_name ?? '',
      returnDate: row.return_date ?? '',
      warehouseId: row.warehouse_id ?? null,
      reasonType: sep >= 0 ? rawReason.slice(0, sep) : rawReason,
      reasonDetail: sep >= 0 ? rawReason.slice(sep + 2) : '',
      remarks: row.remarks ?? '',
      items: [],
      totalAmount: row.total_amount ?? 0,
      status: row.status ?? 'DRAFT',
    });
    removedItemIds.value = [];
    if (row.id !== undefined) {
      loadFormItems(row.id).catch((error: unknown) => {
        logger.error('[sales-return] 退货明细回填失败', error);
        msg.error('loadFailed');
      });
    }
  };

  // 销售订单变更时联动客户与明细
  const onSalesOrderChange = (orderId: number) => {
    const order = salesOrderList.value.find(o => o.id === orderId);
    if (order) {
      formData.salesOrderNo = order.order_no;
      formData.customerId = order.customer_id;
      formData.customerName = order.customer_name;
      if (order.items) {
        formData.items = order.items.map(item => ({
          id: null,
          productId: item.product_id,
          productName: item.product_name,
          productCode: item.product_code,
          quantity: 0,
          unitPrice: item.unit_price,
          taxPercent: undefined,
          discountPercent: undefined,
          amount: 0,
          reason: '',
        }));
      }
      calculateTotal();
    }
  };

  // 添加空明细行
  const addItem = () => {
    formData.items.push({
      id: null,
      productId: null,
      productName: '',
      productCode: '',
      quantity: 1,
      unitPrice: 0,
      amount: 0,
      reason: '',
    });
  };

  // 删除明细行：已落库的行记下 id，提交时按 id DELETE（否则编辑态删行只在前端生效）
  const removedItemIds = ref<number[]>([]);
  const removeItem = (index: number) => {
    const [removed] = formData.items.splice(index, 1);
    if (removed?.id !== null && removed?.id !== undefined) {
      removedItemIds.value.push(removed.id);
    }
    calculateTotal();
  };

  // 重算总金额（仅用于表单展示，服务端独立维护表头 total_amount，不随请求体提交）
  const calculateTotal = () => {
    formData.totalAmount = formData.items.reduce((sum, item) => {
      return sum + item.quantity * item.unitPrice;
    }, 0);
  };

  // 表头显式构造为后端 CreateSalesReturnRequest 字段集；明细由独立端点逐行写入，
  // 因此表头体不得带 items/status/total_amount（后端会忽略，表头金额由服务端汇总）
  const buildHeadBody = (): CreateSalesReturnRequest => ({
    order_id: formData.salesOrderId ?? undefined,
    customer_id: formData.customerId as number,
    return_date: formData.returnDate,
    warehouse_id: formData.warehouseId as number,
    reason_type: formData.reasonType,
    reason_detail: formData.reasonDetail || undefined,
    notes: formData.remarks || undefined,
  });

  // 明细同步：无 id 的新增行 POST，已存在的行 PUT（后端 PUT 仅接受数量/单价/原因），
  // 用户在表单里删掉的已有行按 id 先 DELETE
  const syncItems = async (returnId: number) => {
    for (const itemId of removedItemIds.value) {
      await deleteSalesReturnItem(returnId, itemId);
    }
    removedItemIds.value = [];
    let idx = 0;
    for (const it of formData.items) {
      idx += 1;
      if (it.productId === null) continue;
      if (it.id === null) {
        const body: CreateSalesReturnItemRequest = {
          line_no: idx,
          product_id: it.productId,
          quantity: it.quantity,
          unit_price: it.unitPrice,
          tax_percent: it.taxPercent,
          discount_percent: it.discountPercent,
          reason: it.reason || undefined,
        };
        await createSalesReturnItem(returnId, body);
      } else {
        await updateSalesReturnItem(returnId, it.id, {
          quantity: it.quantity,
          unit_price: it.unitPrice,
          reason: it.reason || undefined,
        });
      }
    }
  };

  // 表单校验 + 提交：先建/改表头，再同步明细；任一明细写入失败即如实报错，不宣称成功
  const submitForm = async (dialogMode: 'create' | 'edit') => {
    if (!formRef.value) return false;

    let valid = false;
    await formRef.value.validate(v => {
      valid = v;
    });
    if (!valid) return false;

    if (formData.items.every(it => it.productId === null)) {
      msg.warning('pleaseAddReturnDetail');
      return false;
    }

    const headBody = buildHeadBody();

    try {
      if (dialogMode === 'create') {
        const res = await createSalesReturn(headBody);
        await syncItems(res.data.id);
        msg.success('createSuccess');
      } else {
        const returnId = formData.id as number;
        await updateSalesReturn(returnId, headBody);
        await syncItems(returnId);
        msg.success('updateSuccess');
      }
      return true;
    } catch (error: unknown) {
      // 表头已写入而明细失败是可能发生的中间态：如实上报真实错误，不假装回滚
      ElMessage.error(
        (error instanceof Error ? error.message : String(error)) ||
          (dialogMode === 'create' ? msg.translate('createFailed') : msg.translate('updateFailed'))
      );
      return false;
    }
  };

  // 表单弹窗关闭
  const onEditDialogClose = () => {
    formRef.value?.resetFields();
  };

  // 一次性初始化加载
  const initLoad = async () => {
    await Promise.all([loadReturns(), loadSalesOrders(), loadCustomers(), loadProducts()]);
  };

  return {
    // 状态
    loading,
    returnList,
    total,
    queryParams,
    currentReturn,
    salesOrderList,
    customerList,
    productList,
    formData,
    formRef,
    // 方法
    loadReturns,
    handleSearch,
    handleReset,
    handlePageChange,
    handleSizeChange,
    loadSalesOrders,
    loadCustomers,
    loadProducts,
    resetFormForCreate,
    fillFormForEdit,
    onSalesOrderChange,
    addItem,
    removeItem,
    calculateTotal,
    submitForm,
    onEditDialogClose,
    initLoad,
  };
}
