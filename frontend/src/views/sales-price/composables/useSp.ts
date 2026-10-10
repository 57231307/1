/**
 * useSp.ts - 销售价格核心 composable
 * 任务编号: P14 批 2 I-3 第 3 批（拆分原 sales-price/index.vue）
 * 提供销售价格列表查询、表单管理、客户/产品加载、CRUD 等核心方法
 * 业务流程（审批/导出/查看）由 useSpProc 提供
 * 批次 284：priceList 接入 useTableApi，移除手写分页/加载逻辑
 */
import { ref, reactive } from 'vue';
import { ElMessage } from 'element-plus';
import { FormInstance } from 'element-plus';
import { msg } from '@/utils/message';
import {
  createSalesPrice,
  updateSalesPrice,
  type SalesPrice,
  type SalesPriceCreateInput,
  type SalesPriceUpdateInput,
} from '@/api/sales-price';
import { getCustomerList, type Customer } from '@/api/customer';
import { getProductList, type Product } from '@/api/product';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import { logger } from '@/utils/logger';
import { useTableApi } from '@/composables/useTableApi';

/**
 * 编辑态（el-input-number 的 number）→ 写线格式（Decimal 十进制字符串）。
 * undefined = 缺值 ⇒ 返回 undefined，由载荷构造处省略键（严禁伪造 0/''，教训：提交 bd6c407c；
 * 0 是合法业务值，会被如实转成 "0" 提交，与"缺值"可区分）。
 * String(number) 输出十进制或科学计数法（如 1e+21），两者均被后端 visit_str 的
 * from_str().or_else(from_scientific) 精确接收（rust_decimal-1.42.1/src/serde.rs:362-368）。
 */
const toDecimalWire = (value: number | undefined): string | undefined =>
  value === undefined ? undefined : String(value);

/** 表单可选 Decimal 字段 → 线格式对象（缺值省略键；两字段在 wire 上均可为 string） */
const decimalFieldsToWire = (form: {
  price?: number;
  min_order_qty?: number;
}): { price?: string; min_order_qty?: string } => {
  const price = toDecimalWire(form.price);
  const minOrderQty = toDecimalWire(form.min_order_qty);
  return {
    ...(price !== undefined ? { price } : {}),
    ...(minOrderQty !== undefined ? { min_order_qty: minOrderQty } : {}),
  };
};

/**
 * 销售价格 composable
 * 集中管理列表、表单、客户、产品、对话框的业务状态
 * 对话框可见性由父组件本地 ref 管理
 */
export function useSp() {
  // 列表 - 接入 useTableApi（批次 284）
  const {
    data: priceList,
    total,
    loading,
    page,
    pageSize,
    queryParams,
    refresh: getList,
  } = useTableApi<SalesPrice>({
    url: '/sales/sales-prices',
    defaultPageSize: 20,
    defaultParams: {
      keyword: '',
      customer_id: undefined as number | undefined,
      product_id: undefined as number | undefined,
      status: '',
    },
    onError: (err: unknown) => {
      // 使用类型守卫安全提取错误信息
      const errMsg = err instanceof Error ? err.message : '';
      ElMessage.error(errMsg || msg.translate('loadSalesPriceListFailed'));
    },
  });

  // 客户和产品列表
  const customers = ref<Customer[]>([]);
  const products = ref<Product[]>([]);

  // 对话框
  const dialogTitle = ref('');
  const formRef = ref<FormInstance>();

  // 表单数据
  // price/min_order_qty 为「编辑态」number（el-input-number 数值控件），初值不得伪造 0：
  // undefined=用户未填 ⇒ 提交时省略键（缺值写 0 覆盖旧值的事故教训：提交 bd6c407c）。
  // 写线格式 string 的换算只发生在 handleSubmitForm 边界（toDecimalWire）。
  const formData = reactive({
    id: undefined as number | undefined,
    product_id: undefined as number | undefined,
    customer_id: undefined as number | undefined,
    price: undefined as number | undefined,
    currency: 'CNY',
    unit: 'meter',
    min_order_qty: undefined as number | undefined,
    price_type: 'STANDARD',
    price_level: '',
    effective_date: '',
    expiry_date: '',
  });

  // 表单验证规则
  const formRules = {
    product_id: [{ required: true, message: '请选择产品', trigger: 'change' }],
    price: [{ required: true, message: '请输入销售价格', trigger: 'blur' }],
    currency: [{ required: true, message: '请选择币种', trigger: 'change' }],
    unit: [{ required: true, message: '请选择单位', trigger: 'change' }],
    effective_date: [{ required: true, message: '请选择生效日期', trigger: 'change' }],
    price_type: [{ required: true, message: '请选择价格类型', trigger: 'change' }],
  };

  // 懒加载标记
  const hasLoaded = createLazyLoader();

  /** 获取客户列表 */
  const getCustomers = async () => {
    try {
      const res = await getCustomerList({ page: 1, page_size: 1000 });
      customers.value = res.data?.items || [];
    } catch (error) {
      logger.error('获取客户列表失败:', error);
    }
  };

  /** 获取产品列表 */
  const getProducts = async () => {
    try {
      const res = await getProductList({ page: 1, page_size: 1000 });
      products.value = res.data?.items || [];
    } catch (error) {
      logger.error('获取产品列表失败:', error);
    }
  };

  /** 查询 */
  const handleQuery = () => {
    page.value = 1;
    getList();
  };

  /** 重置 */
  const handleReset = () => {
    queryParams.value = {
      keyword: '',
      customer_id: undefined,
      product_id: undefined,
      status: '',
    };
    page.value = 1;
    getList();
  };

  /** 准备新建表单（父组件需自行打开对话框） */
  const prepareCreate = () => {
    dialogTitle.value = '新建销售价格';
    Object.assign(formData, {
      id: undefined,
      product_id: undefined,
      customer_id: undefined,
      price: undefined,
      currency: 'CNY',
      unit: 'meter',
      min_order_qty: undefined,
      price_type: 'STANDARD',
      price_level: '',
      effective_date: '',
      expiry_date: '',
    });
  };

  /** 准备编辑表单（父组件需自行打开对话框） */
  const prepareEdit = (row: SalesPrice) => {
    dialogTitle.value = '编辑销售价格';
    // 显式逐字段映射（替代整体 Object.assign(formData, row)）：
    // 1) 读接口 price/min_order_qty 是 Decimal 字符串（后端 models/sales_price.rs::Model 两字段
    //    Decimal、rust_decimal 仅启用 serde feature ⇒ JSON 线格式为字符串），
    //    编辑态控件 el-input-number 需要 number ⇒ 此处做有意的线格式→编辑态换算；
    // 2) 不再把行对象上的非表单键（status/created_at 等）连带塞进 formData。
    Object.assign(formData, {
      id: row.id,
      product_id: row.product_id,
      customer_id: row.customer_id ?? undefined,
      price: Number(row.price),
      currency: row.currency,
      unit: row.unit,
      min_order_qty: Number(row.min_order_qty),
      price_type: row.price_type ?? 'STANDARD',
      price_level: row.price_level ?? '',
      effective_date: row.effective_date ?? '',
      expiry_date: row.expiry_date ?? '',
    });
  };

  /** 提交表单 */
  const handleSubmitForm = async (): Promise<boolean> => {
    try {
      await formRef.value?.validate();
      // 载荷按后端 DTO 键集构造（sales_price_service.rs::CreateSalesPriceInput/::UpdateSalesPriceInput）：
      // 只发两 DTO 认识的键；
      // 空串/undefined 一律省略键（'' 对 date 字段会让 update 侧 parse 失败裸 400，
      // 对 Decimal 字段则是脏值——省略=不变/走默认，才是"未填"的忠实编码）。
      const optionals = {
        ...(formData.customer_id !== undefined ? { customer_id: formData.customer_id } : {}),
        ...(formData.effective_date ? { effective_date: formData.effective_date } : {}),
        ...(formData.expiry_date ? { expiry_date: formData.expiry_date } : {}),
        ...(formData.currency ? { currency: formData.currency } : {}),
      };
      const decimals = decimalFieldsToWire({
        price: formData.price,
        min_order_qty: formData.min_order_qty,
      });
      if (formData.id) {
        const payload: SalesPriceUpdateInput = {
          ...(formData.product_id !== undefined ? { product_id: formData.product_id } : {}),
          ...optionals,
          ...decimals,
        };
        await updateSalesPrice(formData.id, payload);
      } else {
        // product_id/price/unit/price_type 为后端必填（CreateSalesPriceInput 非 Option），
        // 由 formRules required 保证提交时已填；as 仅把该校验承诺传达给类型系统
        // （与 usePp.ts createPayload 既有范式一致）。
        const payload: SalesPriceCreateInput = {
          product_id: formData.product_id as number,
          price: decimals.price as string,
          unit: formData.unit,
          price_type: formData.price_type,
          ...optionals,
          ...(decimals.min_order_qty !== undefined
            ? { min_order_qty: decimals.min_order_qty }
            : {}),
        };
        await createSalesPrice(payload);
      }
      msg.success('saveSuccess');
      await getList();
      return true;
    } catch (error: unknown) {
      // 使用类型守卫安全提取错误信息
      if (error instanceof Error && error.message) {
        ElMessage.error(error.message || msg.translate('operationFailed'));
      }
      return false;
    }
  };

  /** 初始化加载辅助数据（懒加载客户/产品，列表由 useTableApi setup 自动加载） */
  const initLoad = () => {
    loadIfNot('customers', getCustomers, hasLoaded);
    loadIfNot('products', getProducts, hasLoaded);
  };

  // 使用 reactive 包装，访问字段时自动解包 ref
  return reactive({
    // 查询与列表（useTableApi 管理）
    queryParams,
    page,
    pageSize,
    loading,
    priceList,
    total,
    getList,
    handleQuery,
    handleReset,
    // 客户与产品
    customers,
    products,
    getCustomers,
    getProducts,
    // 对话框与表单
    dialogTitle,
    formRef,
    formData,
    formRules,
    prepareCreate,
    prepareEdit,
    handleSubmitForm,
    // 初始化
    initLoad,
  });
}
