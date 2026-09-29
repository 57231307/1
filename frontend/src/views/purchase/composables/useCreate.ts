/**
 * useCreate - 采购单创建表单 composable
 * 任务编号: P13 批 1 B3 I-1（拆分 purchase/index.vue 新建采购单对话框）
 */
import { ref } from 'vue';
import { ElMessage, type FormInstance, type FormRules } from 'element-plus';
import { msg } from '@/utils/message';
import { createPurchaseOrder } from '@/api/purchase';
import { resolveSkuMapping } from '@/api/sku-mapping';
import { i18n } from '@/i18n';
import type { Product } from '@/api/product';

/** SKU 对照解析结果（只读展示用） */
export interface ResolvedMapping {
  supplier_product_code: string;
  supplier_color_no: string;
  supplier_price: string | null;
}

/**
 * 采购明细行数据结构
 */
export interface CreateItem {
  product_id: number | undefined;
  color_id: number | null;
  quantity: number;
  unit_price: number;
  subtotal: number;
  /** 交货容差百分比（undefined=未填，提交时转为 null） */
  quantity_tolerance_pct: number | undefined;
  /** resolveSkuMapping 成功后的只读信息 */
  resolved: ResolvedMapping | null;
}

/**
 * 新建采购单表单数据结构
 */
export interface CreateFormData {
  supplier_id: number | undefined;
  order_date: string;
  required_date: string;
  remark: string;
  /** 收货仓库（后端 validate_order_request 强制非空且真实存在） */
  warehouse_id: number | undefined;
  /** 归属部门（后端 validate_order_request 强制非空且真实存在） */
  department_id: number | undefined;
  items: CreateItem[];
}

/**
 * 新建采购单表单初始默认值
 */
const defaultItem = (): CreateItem => ({
  product_id: undefined,
  color_id: null,
  quantity: 1,
  unit_price: 0,
  subtotal: 0,
  quantity_tolerance_pct: undefined,
  resolved: null,
});

const defaultForm = (): CreateFormData => ({
  supplier_id: undefined,
  order_date: new Date().toISOString().split('T')[0],
  required_date: '',
  remark: '',
  warehouse_id: undefined,
  department_id: undefined,
  items: [defaultItem()],
});

/**
 * 采购单创建表单 composable
 */
export function useCreate(products: () => Product[], onSuccess: () => void) {
  const createDialogVisible = ref(false);
  const createFormRef = ref<FormInstance>();
  const createFormRules: FormRules = {
    supplier_id: [{ required: true, message: '请选择供应商', trigger: 'change' }],
    order_date: [{ required: true, message: '请选择订单日期', trigger: 'change' }],
    warehouse_id: [
      {
        required: true,
        message: i18n.global.t('purchase.createDlg.warehouseRequired'),
        trigger: 'change',
      },
    ],
    department_id: [
      {
        required: true,
        message: i18n.global.t('purchase.createDlg.departmentRequired'),
        trigger: 'change',
      },
    ],
  };
  const createForm = ref<CreateFormData>(defaultForm());

  /**
   * 打开创建采购单对话框
   */
  const handleCreate = () => {
    createForm.value = defaultForm();
    createDialogVisible.value = true;
  };

  /**
   * 添加采购明细行
   */
  const addItem = () => {
    createForm.value.items.push(defaultItem());
  };

  /**
   * 移除采购明细行
   */
  const removeItem = (index: number) => {
    if (createForm.value.items.length > 1) {
      createForm.value.items.splice(index, 1);
    }
  };

  /**
   * 选择产品时自动填入单价，清空色号相关
   */
  const handleProductSelect = (index: number) => {
    const product = products().find(p => p.id === createForm.value.items[index].product_id);
    if (product) {
      createForm.value.items[index].unit_price = product.price || 0;
      calculateSubtotal(createForm.value.items[index]);
    }
    // 换产品时清色号和已解析映射
    createForm.value.items[index].color_id = null;
    createForm.value.items[index].resolved = null;
  };

  /**
   * 选择色号后触发 resolveSkuMapping（需 supplier_id + product_id + color_id 均已填）
   * 无对照 → message.error 拒绝转采购
   */
  const handleColorSelect = async (index: number) => {
    const item = createForm.value.items[index];
    const supplierId = createForm.value.supplier_id;
    if (!supplierId || !item.product_id || !item.color_id) {
      item.resolved = null;
      return;
    }
    try {
      const res = await resolveSkuMapping({
        product_id: item.product_id,
        color_id: item.color_id,
        supplier_id: supplierId,
      });
      if (res.data) {
        const m = res.data;
        item.resolved = {
          supplier_product_code: m.supplier_product_code,
          supplier_color_no: m.supplier_color_no ?? '',
          supplier_price: m.supplier_price,
        };
        // 默认单价取协议价，允许现谈覆盖
        if (m.supplier_price) {
          item.unit_price = Number(m.supplier_price) || 0;
          calculateSubtotal(item);
        }
      }
    } catch (error: unknown) {
      item.resolved = null;
      const errObj = error as { response?: { status?: number; data?: { code?: string } } };
      const status = errObj?.response?.status;
      const code = errObj?.response?.data?.code;
      if (status === 404 || code === 'NOT_FOUND' || code === 'BUSINESS_ERROR') {
        msg.error('skuMappingNotFound');
      } else {
        msg.error('skuMappingResolveFailed');
      }
      console.error('[purchase] resolveSkuMapping error:', error);
    }
  };

  /**
   * 供应商变更时，若已选产品+色号则重新解析
   */
  const handleSupplierChange = () => {
    for (let i = 0; i < createForm.value.items.length; i++) {
      const item = createForm.value.items[i];
      if (item.product_id && item.color_id && createForm.value.supplier_id) {
        handleColorSelect(i);
      } else {
        item.resolved = null;
      }
    }
  };

  /**
   * 重算小计金额
   */
  const calculateSubtotal = (item: CreateItem) => {
    item.subtotal = (item.quantity || 0) * (item.unit_price || 0);
  };

  /**
   * 计算总金额
   */
  const calculateTotal = () => {
    return createForm.value.items.reduce(
      (sum: number, item: CreateItem) => sum + (item.subtotal || 0),
      0
    );
  };

  /**
   * 提交新建采购单
   */
  const submitCreate = async () => {
    try {
      await createFormRef.value?.validate();
    } catch {
      return;
    }
    const validItems = createForm.value.items.filter(item => item.product_id && item.quantity > 0);
    if (validItems.length === 0) {
      msg.warning('pleaseAddPurchaseDetail');
      return;
    }
    // 无对照即拒绝转采购
    const unresolved = validItems.filter(
      item => item.product_id && item.color_id && !item.resolved
    );
    if (unresolved.length > 0) {
      msg.error('skuMappingRequired');
      return;
    }
    // supplier_id/order_date/warehouse_id/department_id 均由表单必填校验保证非空，
    // 缺失时上方 validate() 已中断，此处收敛类型供契约负载使用。
    const supplierId = createForm.value.supplier_id;
    const orderDate = createForm.value.order_date;
    const warehouseId = createForm.value.warehouse_id;
    const departmentId = createForm.value.department_id;
    if (supplierId == null || !orderDate || warehouseId == null || departmentId == null) {
      // validate() 理论上已拦截；显式兜底返回而非把 undefined 发往后端
      return;
    }
    try {
      await createPurchaseOrder({
        supplier_id: supplierId,
        order_date: orderDate,
        warehouse_id: warehouseId,
        department_id: departmentId,
        expected_delivery_date: createForm.value.required_date || undefined,
        notes: createForm.value.remark || undefined,
        items: validItems.map((item, idx) => ({
          line_no: idx + 1,
          material_id: item.product_id!,
          quantity_ordered: item.quantity,
          unit_price: item.unit_price,
          ...(item.quantity_tolerance_pct != null
            ? { quantity_tolerance_pct: item.quantity_tolerance_pct }
            : {}),
        })),
      });
      msg.success('purchaseOrderCreated');
      createDialogVisible.value = false;
      onSuccess();
    } catch (error: unknown) {
      ElMessage.error(
        (error instanceof Error ? error.message : '') || msg.translate('createFailed')
      );
    }
  };

  return {
    createDialogVisible,
    createFormRef,
    createFormRules,
    createForm,
    handleCreate,
    addItem,
    removeItem,
    handleProductSelect,
    handleColorSelect,
    handleSupplierChange,
    calculateSubtotal,
    calculateTotal,
    submitCreate,
  };
}
