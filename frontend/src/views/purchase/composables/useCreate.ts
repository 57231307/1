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
import {
  inclusiveAmountFromLine,
  netFromInclusive,
  parseDecimalInput,
  taxFromInclusive,
} from '@/utils/money';
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
  /**
   * 行级**含税单价**（价税分离的录入事实，本表唯一价格主输入）：保存输入框原文，
   * 空串表示未填写，'0' 是人工显式声明的合法值（样布 赠品 寄售行）；
   * 提交前由 items 必填规则经 parseDecimalInput 拦截未填写与非法文本，
   * 非 0 税率行缺本键后端点名拒绝建单（utils/tax.rs 的 require_inclusive_unit_price，
   * 系统不乘 (1 + 税率) 代造含税价）。
   * 初值留空而不是 0：把未填预置成 0 会让"漏录单价"与"声明 0 价"在进项与成本台账里不可区分。
   */
  unit_price_with_tax: string;
  /**
   * 行级**不含税单价**（派生只读）：由含税单价按本行税率除法反算
   * （money.ts 的 netFromInclusive，与后端 utils/tax.rs 同源），不可手输。
   * 提交时随请求一起上送，后端 ensure_unit_price_reconciles 逐分复核，分位不一致即拒绝。
   */
  unit_price: number | undefined;
  /**
   * 行级**税额预览**（派生只读）：含税金额减净额取差额（money.ts 的 taxFromInclusive），
   * 只做编辑态展示；落库税额由后端按真实税率反算，前端不参与生成。
   * 未填含税单价或未录税率时为 undefined（无值），不预置 0 冒充算得的税额。
   */
  tax_amount: number | undefined;
  /**
   * 行级税率（百分数口径，13 表示 13%），undefined/null=未填写，0=显式"无进项票"为合法提交值，
   * 提交前由 items 必填规则拦截未填写行，落库列 purchase_order_item.tax_percent（NOT NULL）。
   */
  tax_rate: number | undefined;
  /** 行级净额小计（派生只读）：含税金额聚合后按税率除法反算，与后端 subtotal 列同口径；
   *  未填含税单价或未录税率时为 undefined（无值），不预置 0 冒充算得的净额 */
  subtotal: number | undefined;
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
  // 含税单价不预填 0，新建时留空由人工填写（0 只能由人工显式敲出）
  unit_price_with_tax: '',
  // 不含税单价、税额与小计是派生列，未填含税单价或未录税率时保持"无值"，不预置 0 冒充算得
  unit_price: undefined,
  tax_amount: undefined,
  // 税率不预填任何档位，新建时留空由人工填写（无默认 13 之类硬编）
  tax_rate: undefined,
  subtotal: undefined,
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
  // 建单在途：提交前要先异步校验表单，若等校验返回才置位，连点两下会各建一张采购单
  const creating = ref(false);
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
    items: [
      {
        // 行级含税单价与税率就地必填校验（不靠后端 400 兜回），口径与 submitCreate 的 validItems 一致，
        // 仅对已选产品且有数量的行要求；0 是合法值，只有未填写才算缺。
        // 先点名单价再点名税率：漏录含税单价是本表最容易被当成"已填 0"的一列，提示要指名。
        validator: (_rule: unknown, value: CreateItem[], cb: (error?: Error) => void) => {
          const filled = value.filter(i => i.product_id && i.quantity > 0);
          const missingUnitPrice = filled.find(
            i => parseDecimalInput(i.unit_price_with_tax) == null
          );
          if (missingUnitPrice) {
            cb(new Error(i18n.global.t('purchase.createDlg.unitPriceRequired')));
            return;
          }
          const invalidUnitPrice = filled.find(i => {
            const parsed = parseDecimalInput(i.unit_price_with_tax);
            return parsed !== undefined && (!Number.isFinite(parsed) || parsed < 0);
          });
          if (invalidUnitPrice) {
            cb(new Error(i18n.global.t('purchase.createDlg.unitPriceInvalid')));
            return;
          }
          const missingTaxRate = filled.find(i => i.tax_rate == null);
          if (missingTaxRate) {
            cb(new Error(i18n.global.t('purchase.createDlg.taxRateRequired')));
            return;
          }
          cb();
        },
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
   * 选择产品时带出物料参考价填入**含税单价**（本表唯一价格主输入），清空色号相关
   * 物料无参考价就留空（交 items 必填校验拦），不补 0：补 0 等于替用户声明这是一张 0 价行
   */
  const handleProductSelect = (index: number) => {
    const product = products().find(p => p.id === createForm.value.items[index].product_id);
    if (product) {
      // 出参 price 是 DECIMAL 字符串（NULL→null）：原文带进含税单价主输入列，缺值留空交 items 必填校验拦
      const rawPrice = product.price;
      createForm.value.items[index].unit_price_with_tax =
        rawPrice == null || rawPrice === '' ? '' : String(rawPrice);
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
        // 默认含税单价取协议价（Decimal 十进制字符串原样带出，不做 float 往返），
        // 允许现谈覆盖；协议价取不到可解析的数值就保持留空，
        // 由 items 必填校验拦，不静默补 0（显式 0 只能由人工敲）
        if (m.supplier_price != null && Number.isFinite(Number(m.supplier_price))) {
          item.unit_price_with_tax = m.supplier_price;
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
   * 重算派生列（含税单价是录入事实，净额与税额一律除法反算取差额）：
   * - 含税金额 = 含税单价 × 数量（inclusiveAmountFromLine，税率不进算式，不构成造含税）
   * - 不含税单价与净额小计 = 含税值 ÷ (1 + 税率% ÷ 100) 四舍五入到分（netFromInclusive）
   * - 税额 = 含税金额 − 净额小计（taxFromInclusive 取差额，与后端 utils tax 同一口径，
   *   不做第二次独立舍入，恒等式 净额 + 税额 == 含税 逐分成立）
   * 未填含税单价或未录税率时派生列回到无值：不猜档位、不补 0，缺值由 items 必填校验点名。
   */
  const calculateSubtotal = (item: CreateItem) => {
    const inclusiveUnit = parseDecimalInput(item.unit_price_with_tax);
    if (inclusiveUnit === undefined || !Number.isFinite(inclusiveUnit) || inclusiveUnit < 0) {
      item.unit_price = undefined;
      item.tax_amount = undefined;
      item.subtotal = undefined;
      return;
    }
    if (item.tax_rate === undefined || item.tax_rate === null) {
      item.unit_price = undefined;
      item.tax_amount = undefined;
      item.subtotal = undefined;
      return;
    }
    const inclusiveLine = inclusiveAmountFromLine(inclusiveUnit, item.quantity);
    item.unit_price = netFromInclusive(inclusiveUnit, item.tax_rate);
    item.subtotal = netFromInclusive(inclusiveLine, item.tax_rate);
    item.tax_amount = taxFromInclusive(inclusiveLine, item.tax_rate);
  };

  /**
   * 计算总金额（净额口径合计，派生列缺值按 0 计入合计，不改动行上的无值语义）
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
    if (creating.value) return;
    creating.value = true;
    try {
      await createFormRef.value?.validate();
    } catch {
      creating.value = false;
      return;
    }
    const validItems = createForm.value.items.filter(item => item.product_id && item.quantity > 0);
    if (validItems.length === 0) {
      creating.value = false;
      msg.warning('pleaseAddPurchaseDetail');
      return;
    }
    // 无对照即拒绝转采购
    const unresolved = validItems.filter(
      item => item.product_id && item.color_id && !item.resolved
    );
    if (unresolved.length > 0) {
      creating.value = false;
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
      creating.value = false;
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
          // 含税单价是录入事实（主输入），按十进制字符串走线原样上送；
          // items 必填校验已保证它是可解析的非负数值，缺值与非法文本走不到这里
          unit_price_with_tax: item.unit_price_with_tax,
          // unit_price 是上面 calculateSubtotal 由含税单价除法反算出的派生只读值，
          // 后端 ensure_unit_price_reconciles 会逐分复核，与含税事实不同源即拒绝建单
          unit_price: item.unit_price!,
          // tax_rate 已由上方 items 必填校验保证为数值（0 为合法提交值），! 仅收敛类型不设默认
          tax_rate: item.tax_rate!,
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
    } finally {
      creating.value = false;
    }
  };

  return {
    createDialogVisible,
    createFormRef,
    createFormRules,
    creating,
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
