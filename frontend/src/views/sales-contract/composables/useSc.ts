/**
 * useSc.ts - 销售合同核心 composable
 * 任务编号: P14 批 2 I-3 第 1 批（拆分原 sales-contract/index.vue）
 * 提供销售合同列表查询、表单管理、客户加载、CRUD 等核心方法
 * 业务流程（提交审批/审批/执行/打印/导出）由 useScProc 提供
 * 批次 284：contractList 接入 useTableApi，移除手写分页/加载逻辑
 *
 * P0 契约修复（本轮）：
 * 1. 明细回源：后端 get_list/get_by_id 从不返回 items（实体无该键），原 prepareEdit 直接读
 *    row.items ⇒ 恒为空数组，保存后再编辑明细全丢。现改为打开编辑前调用既有端点
 *    GET /sales/sales-contracts/:id/items 回填（数量/单价 Decimal→string 需 Number() 归一）。
 * 2. 表头键名三端对齐：create/update payload 补齐 signed_date/effective_date/expiry_date/
 *    payment_method/delivery_location 真实列；update 由「只发 2 个字段」改为全量 PATCH。
 */
import { ref, reactive } from 'vue';
import { ElMessage } from 'element-plus';
import { msg } from '@/utils/message';
import {
  createSalesContract,
  updateSalesContract,
  getSalesContractItems,
  type SalesContract,
  type CreateSalesContractPayload,
  type CreateContractItemInput,
  type UpdateSalesContractPayload,
} from '@/api/sales-contract';
// D14 Batch 5b：原 customerApi 对象已转风格 B 函数
import { getCustomerList, type Customer } from '@/api/customer';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import { logger } from '@/utils/logger';
import { useTableApi } from '@/composables/useTableApi';

/**
 * 销售合同 composable
 * 集中管理列表、表单、客户、对话框的业务状态
 * 对话框可见性由父组件本地 ref 管理
 */
export function useSc() {
  // 列表 - 接入 useTableApi（批次 284）
  const {
    data: contractList,
    total,
    loading,
    page,
    pageSize,
    queryParams,
    setQueryParam,
    refresh: getList,
  } = useTableApi<SalesContract>({
    url: '/sales/sales-contracts',
    defaultPageSize: 20,
    defaultParams: {
      keyword: '',
      customer_id: undefined as number | undefined,
      status: '',
      signed_date_from: '',
      signed_date_to: '',
    },
    onError: (err: unknown) => {
      // 使用类型守卫安全提取错误信息
      const errMsg = err instanceof Error ? err.message : '';
      ElMessage.error(errMsg || msg.translate('loadSalesContractListFailed'));
    },
  });

  // 日期范围（SalesContractFilter 通过 date-change emit 回传，保留特殊处理）
  const dateRange = ref<[Date, Date] | null>(null);

  // 客户列表
  const customers = ref<Customer[]>([]);

  // 对话框
  const dialogTitle = ref('');

  // 表单数据
  const formData = reactive({
    id: undefined as number | undefined,
    contract_no: '',
    contract_name: '',
    customer_id: undefined as number | undefined,
    contract_type: '',
    total_amount: 0,
    signed_date: '',
    effective_date: '',
    expiry_date: '',
    payment_terms: '',
    payment_method: '',
    delivery_date: '',
    delivery_location: '',
    remarks: '',
    items: [] as Array<{
      product_name: string;
      unit: string;
      quantity: number;
      unit_price: number;
      quantity_tolerance_pct: number | undefined;
      // 实体真实列（表单不采集、不可见，但编辑保存=明细整表 delete+重插，
      // 必须随 items 原样回传，否则每次保存被洗成 NULL）：
      product_id: number | null;
      product_spec: string | null;
      delivery_date: string | null;
      remarks: string | null;
    }>,
  });

  // 懒加载标记
  const hasLoaded = createLazyLoader();

  /** 处理日期范围变化（批次 284：改为 setQueryParam + page=1 + refresh） */
  const handleDateChange = () => {
    if (dateRange.value) {
      setQueryParam('signed_date_from', dateRange.value[0].toISOString().split('T')[0]);
      setQueryParam('signed_date_to', dateRange.value[1].toISOString().split('T')[0]);
    } else {
      setQueryParam('signed_date_from', '');
      setQueryParam('signed_date_to', '');
    }
    page.value = 1;
    getList();
  };

  /** 获取客户列表 */
  const getCustomers = async () => {
    try {
      const res = await getCustomerList();
      customers.value = res.data?.items || [];
    } catch (error) {
      logger.error('获取客户列表失败:', error);
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
      status: '',
      signed_date_from: '',
      signed_date_to: '',
    };
    dateRange.value = null;
    page.value = 1;
    getList();
  };

  /** 准备新建表单（父组件需自行打开对话框） */
  const prepareCreate = () => {
    dialogTitle.value = '新建销售合同';
    Object.assign(formData, {
      id: undefined,
      contract_no: '',
      contract_name: '',
      customer_id: undefined,
      contract_type: '',
      total_amount: 0,
      signed_date: '',
      effective_date: '',
      expiry_date: '',
      payment_terms: '',
      payment_method: '',
      delivery_date: '',
      delivery_location: '',
      remarks: '',
      items: [],
    });
  };

  /**
   * 准备编辑表单（P0 修复：异步回源明细后再返回，父组件 await 后打开对话框）
   * - 表头：列表行 = 后端 sales_contract::Model 全列，直接映射（total_amount Decimal→string 需 Number()）
   * - 明细：后端行对象不含 items，必须调 GET /:id/items 回填；
   *   quantity/unit_price/quantity_tolerance_pct 出参为 string，绑定数值控件前归一为 number
   */
  const prepareEdit = async (row: SalesContract) => {
    dialogTitle.value = '编辑销售合同';
    Object.assign(formData, {
      id: row.id,
      contract_no: row.contract_no,
      contract_name: row.contract_name,
      customer_id: row.customer_id,
      contract_type: row.contract_type ?? '',
      total_amount: Number(row.total_amount ?? 0),
      signed_date: row.signed_date ?? '',
      effective_date: row.effective_date ?? '',
      expiry_date: row.expiry_date ?? '',
      payment_terms: row.payment_terms ?? '',
      payment_method: row.payment_method ?? '',
      delivery_date: row.delivery_date ?? '',
      delivery_location: row.delivery_location ?? '',
      // 后端无 remarks 列（remark 迁移落库前该输入不持久化，见汇报「迁移需求」）
      remarks: '',
    });
    try {
      const res = await getSalesContractItems(row.id);
      formData.items = (res.data ?? []).map(it => ({
        product_name: it.product_name,
        unit: it.unit,
        quantity: Number(it.quantity),
        unit_price: Number(it.unit_price),
        // 出参 quantity_tolerance_pct 为 Decimal 字符串或 null：真实空值保持 undefined
        quantity_tolerance_pct:
          it.quantity_tolerance_pct != null ? Number(it.quantity_tolerance_pct) : undefined,
        // 实体真实列原样保留（表单不可见但保存时须回传，整表重插不洗列）：
        product_id: it.product_id,
        product_spec: it.product_spec,
        delivery_date: it.delivery_date,
        remarks: it.remarks,
      }));
    } catch (error) {
      logger.error('获取销售合同明细失败:', error);
      formData.items = [];
      ElMessage.error(
        (error instanceof Error ? error.message : '') ||
          msg.translate('loadSalesContractListFailed')
      );
    }
  };

  /**
   * 表单明细行 → 后端 CreateContractItemDto。
   * 后端 update 对明细走「delete_many + 整表重插」（services/sales_contract_service.rs:316-321），
   * 回显保留的真实列 product_id/product_spec/delivery_date/remarks 必须原样回传
   * （null=真实空值，禁止塞默认值），否则每次编辑保存把这几列洗成 NULL。
   */
  const buildItemsPayload = (): CreateContractItemInput[] =>
    formData.items
      .filter(i => i.product_name && i.quantity > 0)
      .map(i => ({
        product_id: i.product_id,
        product_name: i.product_name,
        product_spec: i.product_spec,
        unit: i.unit,
        quantity: i.quantity,
        // el-input-number 清空为 undefined ⇒ JSON 序列化后为缺省，后端 Option=None（走默认允差解析）
        quantity_tolerance_pct: i.quantity_tolerance_pct,
        unit_price: i.unit_price,
        delivery_date: i.delivery_date,
        remarks: i.remarks,
      }));

  /** 提交表单 */
  const handleSubmitForm = async () => {
    try {
      if (formData.id) {
        // P0 修复：update 由「只发 contract_name/payment_terms」改为表头全量 PATCH + 明细整表替换
        const updatePayload: UpdateSalesContractPayload = {
          contract_no: formData.contract_no,
          contract_name: formData.contract_name,
          customer_id: formData.customer_id,
          total_amount: formData.total_amount,
          contract_type: formData.contract_type || undefined,
          payment_terms: formData.payment_terms || undefined,
          delivery_date: formData.delivery_date || undefined,
          signed_date: formData.signed_date || undefined,
          effective_date: formData.effective_date || undefined,
          expiry_date: formData.expiry_date || undefined,
          payment_method: formData.payment_method || undefined,
          delivery_location: formData.delivery_location || undefined,
          remark: formData.remarks || undefined,
          items: buildItemsPayload(),
        };
        await updateSalesContract(formData.id, updatePayload);
      } else {
        const payload: CreateSalesContractPayload = {
          contract_no: formData.contract_no,
          contract_name: formData.contract_name,
          customer_id: formData.customer_id!,
          total_amount: formData.total_amount,
          contract_type: formData.contract_type || undefined,
          payment_terms: formData.payment_terms || undefined,
          // 后端 delivery_date 已对齐真实列可空性（Option<NaiveDate>），未填不再 400
          delivery_date: formData.delivery_date || undefined,
          signed_date: formData.signed_date || undefined,
          effective_date: formData.effective_date || undefined,
          expiry_date: formData.expiry_date || undefined,
          payment_method: formData.payment_method || undefined,
          delivery_location: formData.delivery_location || undefined,
          remark: formData.remarks || undefined,
          items: buildItemsPayload(),
        };
        await createSalesContract(payload);
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

  /** 初始化加载辅助数据（懒加载客户，列表由 useTableApi setup 自动加载） */
  const initLoad = () => {
    loadIfNot('customers', getCustomers, hasLoaded);
  };

  // 使用 reactive 包装，访问字段时自动解包 ref
  return reactive({
    // 查询与列表（useTableApi 管理）
    queryParams,
    page,
    pageSize,
    loading,
    contractList,
    total,
    getList,
    handleQuery,
    handleReset,
    // 日期范围
    dateRange,
    handleDateChange,
    // 客户
    customers,
    getCustomers,
    // 对话框与表单
    dialogTitle,
    formData,
    prepareCreate,
    prepareEdit,
    handleSubmitForm,
    // 初始化
    initLoad,
  });
}

export type ScLazyLoader = Record<string, boolean>;
