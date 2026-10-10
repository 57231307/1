/**
 * usePc.ts - 采购合同核心 composable
 * 任务编号: P14 批 2 I-3 第 3 批（拆分原 purchase-contract/index.vue）
 * 提供采购合同列表查询、表单管理、供应商加载、CRUD 等核心方法
 * 业务流程（提交/审批/执行/删除/导出）由 usePcProc 提供
 * 批次 284：contractList 接入 useTableApi，移除手写分页/加载逻辑
 */
import { ref, reactive } from 'vue';
import { FormInstance } from 'element-plus';
import { msg } from '@/utils/message';
import {
  createPurchaseContract,
  updatePurchaseContract,
  type PurchaseContract,
  type CreatePurchaseContractPayload,
  type UpdatePurchaseContractPayload,
} from '@/api/purchase-contract';
import { getSupplierList, type Supplier } from '@/api/supplier';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import { logger } from '@/utils/logger';
import { useTableApi } from '@/composables/useTableApi';
import { i18n } from '@/i18n';

/**
 * DB 可空列入参归一（三态语义，对齐后端 RFC 7386 显式 null 清空）：
 * ''/null/undefined → null（显式清空该列），非空 → 原值（覆盖）。
 * 禁止塌成 `|| undefined`：省略键在后端语义是「保持原值」，
 * 用户清空交货日期/备注后保存会被静默丢弃——本轮要消灭的形态。
 */
const explicitToNull = (v: string | null | undefined): string | null =>
  v === null || v === undefined || v === '' ? null : v;

/**
 * 采购合同 composable
 * 集中管理列表、表单、供应商、对话框的业务状态
 * 对话框可见性由父组件本地 ref 管理
 */
export function usePc() {
  // 列表 - 接入 useTableApi（批次 284）
  const {
    data: contractList,
    total,
    loading,
    page,
    pageSize,
    queryParams,
    refresh: getList,
  } = useTableApi<PurchaseContract>({
    url: '/purchase/purchase-contracts',
    defaultPageSize: 20,
    defaultParams: {
      keyword: '',
      supplier_id: undefined as number | undefined,
      status: '',
      date_range: [] as string[],
    },
    onError: (err: unknown) => {
      logger.error('获取采购合同列表失败:', err);
    },
  });

  // 供应商列表
  const suppliers = ref<Supplier[]>([]);

  // 对话框
  const dialogTitle = ref('');
  const formRef = ref<FormInstance>();

  // 表单数据
  // 可空列字段如实声明 string | null：编辑回显直接承接后端真实 NULL（出参键值），
  // 提交时经 explicitToNull 显式回传 null（清空语义），不再用 '' 掩盖空值状态。
  const formData = reactive({
    id: undefined as number | undefined,
    contract_no: '',
    contract_name: '',
    supplier_id: undefined as number | undefined,
    contract_type: '' as string | null,
    total_amount: 0,
    signed_date: '' as string | null,
    effective_date: '' as string | null,
    expiry_date: '' as string | null,
    payment_terms: '' as string | null,
    payment_method: '' as string | null,
    delivery_date: '' as string | null,
    delivery_location: '' as string | null,
    remarks: '' as string | null,
  });

  // 表单验证规则
  // delivery_date: 后端 DTO 已对齐真实列可空性（purchase_contracts.delivery_date 可空 → Option），
  // 但采购录入产品口径要求必填交货日期 ⇒ 前端保留 required 规则（严于 DB 可空性允许）
  const formRules = {
    contract_no: [
      {
        required: true,
        message: i18n.global.t('purchaseContract.validation.contractNoRequired'),
        trigger: 'blur',
      },
    ],
    contract_name: [
      {
        required: true,
        message: i18n.global.t('purchaseContract.validation.contractNameRequired'),
        trigger: 'blur',
      },
    ],
    supplier_id: [
      {
        required: true,
        message: i18n.global.t('purchaseContract.validation.supplierRequired'),
        trigger: 'change',
      },
    ],
    delivery_date: [
      {
        required: true,
        message: i18n.global.t('purchaseContract.validation.deliveryDateRequired'),
        trigger: 'change',
      },
    ],
  };

  // 懒加载标记
  const hasLoaded = createLazyLoader();

  /** 获取供应商列表 */
  const getSuppliers = async () => {
    try {
      const res = await getSupplierList();
      suppliers.value = res.data?.items || [];
    } catch (error) {
      logger.error('获取供应商列表失败:', error);
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
      supplier_id: undefined,
      status: '',
      date_range: [],
    };
    page.value = 1;
    getList();
  };

  /** 准备新建表单（父组件需自行打开对话框） */
  const prepareCreate = () => {
    dialogTitle.value = i18n.global.t('purchaseContract.form.dialogCreateTitle');
    Object.assign(formData, {
      id: undefined,
      contract_no: '',
      contract_name: '',
      supplier_id: undefined,
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
    });
  };

  /** 准备编辑表单（父组件需自行打开对话框） */
  const prepareEdit = (row: PurchaseContract) => {
    dialogTitle.value = i18n.global.t('purchaseContract.form.dialogEditTitle');
    Object.assign(formData, row);
    // 后端真实列/出参键为 remark（单数），表单键为 remarks：回显必须显式映射。
    // 不回显则 update payload 会以「表单为空」把已存在的 remark 送 null 洗掉。
    formData.remarks = row.remark;
    formData.total_amount = Number(row.total_amount ?? 0);
  };

  /**
   * 提交表单
   * 三态清空语义（对齐后端 RFC 7386 显式 null）：
   * - 编辑：可空列空值经 explicitToNull 显式送 null（清空该列）；NOT NULL 列
   *   （contract_name/supplier_id）空则省略键（=保持原值，送显式 null 会被后端业务错误拒绝）。
   * - 新建：Create DTO 的 Option 字段未填时省略键即可（创建无"保持原值"问题）。
   * delivery_date 表单仍按产品口径必填（DB 列可空，后端已开放三态）。
   */
  const handleSubmitForm = async (): Promise<boolean> => {
    try {
      await formRef.value?.validate();
      if (formData.id) {
        const updatePayload: UpdatePurchaseContractPayload = {
          // 不发送 contract_no：后端 UpdateContractDto 无该字段（编号系统生成，编辑链路忽略）
          // NOT NULL 列：空则省略键（保持原值），禁止塌成显式 null
          contract_name: formData.contract_name || undefined,
          supplier_id: formData.supplier_id,
          // DB 可空列：有值=覆盖、空=显式 null 清空（省略键=保持原值不是本表单意图，
          // 编辑表单已回显真实值，未动的值原样覆盖回去）
          total_amount: formData.total_amount,
          contract_type: explicitToNull(formData.contract_type),
          payment_terms: explicitToNull(formData.payment_terms),
          delivery_date: explicitToNull(formData.delivery_date),
          signed_date: explicitToNull(formData.signed_date),
          effective_date: explicitToNull(formData.effective_date),
          expiry_date: explicitToNull(formData.expiry_date),
          payment_method: explicitToNull(formData.payment_method),
          delivery_location: explicitToNull(formData.delivery_location),
          remark: explicitToNull(formData.remarks),
        };
        await updatePurchaseContract(formData.id, updatePayload);
      } else {
        const createPayload: CreatePurchaseContractPayload = {
          contract_no: formData.contract_no,
          contract_name: formData.contract_name,
          supplier_id: formData.supplier_id as number,
          total_amount: formData.total_amount,
          contract_type: formData.contract_type || undefined,
          payment_terms: formData.payment_terms || undefined,
          // 新建无"保持原值"问题：未填省略键（Create DTO Option → NULL）
          delivery_date: formData.delivery_date || undefined,
          signed_date: formData.signed_date || undefined,
          effective_date: formData.effective_date || undefined,
          expiry_date: formData.expiry_date || undefined,
          payment_method: formData.payment_method || undefined,
          delivery_location: formData.delivery_location || undefined,
          remark: formData.remarks || undefined,
        };
        await createPurchaseContract(createPayload);
      }
      msg.success('saveSuccess');
      await getList();
      return true;
    } catch (error) {
      logger.error('表单验证失败:', error);
      return false;
    }
  };

  /** 初始化加载辅助数据（懒加载供应商，列表由 useTableApi setup 自动加载） */
  const initLoad = () => {
    loadIfNot('suppliers', getSuppliers, hasLoaded);
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
    // 供应商
    suppliers,
    getSuppliers,
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
