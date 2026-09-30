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
  const formData = reactive({
    id: undefined as number | undefined,
    contract_no: '',
    contract_name: '',
    supplier_id: undefined as number | undefined,
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
    formData.total_amount = Number(row.total_amount ?? 0);
  };

  /**
   * 提交表单
   * P0 契约修复（本轮）：
   * - 新建：CreatePurchaseContractPayload 补齐真实列表头字段
   *   contract_type/signed_date/effective_date/expiry_date/payment_method/delivery_location
   *   （原「DTO 不收、service 不写、表单有输入框」⇒ 创建即丢数据；现后端三端已对齐）。
   *   remark 键名单数（非 remarks）；该字段后端暂无对应列（迁移中），先按 DTO 原样提交。
   * - 编辑：UpdatePurchaseContractPayload 由「只发 contract_name/payment_terms」改为
   *   表头全量 PATCH（后端 UpdateContractDto 已扩展），修复「保存后再编辑内容不完整」。
   *   delivery_date 表单仍按产品口径必填（后端已对齐真实列可空性为 Option）。
   */
  const handleSubmitForm = async (): Promise<boolean> => {
    try {
      await formRef.value?.validate();
      if (formData.id) {
        const updatePayload: UpdatePurchaseContractPayload = {
          contract_no: formData.contract_no,
          contract_name: formData.contract_name,
          supplier_id: formData.supplier_id,
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
          delivery_date: formData.delivery_date,
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
