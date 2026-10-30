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
  /**
   * 行级单价，undefined 或 null 表示未填写，0 是人工显式声明的合法值（样布 赠品 寄售行）；
   * 提交前由 items 必填规则拦截未填写行，落库列 purchase_order_item.unit_price（NOT NULL）。
   * 初值留空而不是 0：把未填预置成 0 会让"漏录单价"与"声明 0 价"在进项与成本台账里不可区分。
   */
  unit_price: number | undefined;
  /**
   * 行级税率（百分数口径，13 表示 13%），undefined/null=未填写，0=显式"无进项票"为合法提交值，
   * 提交前由 items 必填规则拦截未填写行，落库列 purchase_order_item.tax_percent（NOT NULL）。
   */
  tax_rate: number | undefined;
  subtotal: number;
  /** 交货容差百分比（undefined=未填，提交时转为 null） */
  quantity_tolerance_pct: number | undefined;
  /** resolveSkuMapping 成功后的只读信息 */
  resolved: ResolvedMapping | null;
  /**
   * resolveSkuMapping 被拒时这一行的可执行原因。优先取后端出参里可外显的 message
   * （该端点的拒绝文案会分开口径说清压根没建对照，还是只有不分色号的通用对照而缺本色号的色号级对照），
   * 后端只回脱敏常量时退回本地文案。解析成功或尚未尝试时为 null。
   * 挂在行上而不是只弹一次提示，因为补对照要离开本对话框去SKU对照表，
   * 提示消失后"供应商标识"这一列还得说清自己是为什么空着。
   */
  resolve_error: string | null;
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
  // 单价不预填 0，新建时留空由人工填写（0 只能由人工显式敲出）
  unit_price: undefined,
  // 税率不预填任何档位，新建时留空由人工填写（无默认 13 之类硬编）
  tax_rate: undefined,
  subtotal: 0,
  quantity_tolerance_pct: undefined,
  resolved: null,
  resolve_error: null,
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
 * 取一行SKU对照解析失败后可显示的拒绝原因。
 * resolve 端点会把"没建对照"和"只有不分色号的通用对照、缺本色号的色号级对照"两种情形
 * 分成两条可外显文案，优先原样用出参 message（拦截器弹出的也是它，两处必须同源），
 * 出参没有可读文案时才退回本地兜底，避免这一列显示一个空字符串冒充原因。
 */
function resolveFailureReason(error: unknown): string {
  const serverMessage = (error as { response?: { data?: { message?: string } } })?.response?.data
    ?.message;
  return serverMessage && serverMessage.trim() !== ''
    ? serverMessage
    : i18n.global.t('message.skuMappingResolveFailed');
}

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
        // 行级单价与税率就地必填校验（不靠后端 400 兜回），口径与 submitCreate 的 validItems 一致，
        // 仅对已选产品且有数量的行要求；0 是合法值，只有 undefined/null 才算未填。
        // 先点名单价再点名税率：漏录单价是本表最容易被当成"已填 0"的一列，提示要指名。
        validator: (_rule: unknown, value: CreateItem[], cb: (error?: Error) => void) => {
          const filled = value.filter(i => i.product_id && i.quantity > 0);
          const missingUnitPrice = filled.find(i => i.unit_price == null);
          if (missingUnitPrice) {
            cb(new Error(i18n.global.t('purchase.createDlg.unitPriceRequired')));
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
   * 选择产品时带出物料参考价填入单价，清空色号相关
   * 物料无参考价就留空（交 items 必填校验拦），不补 0：补 0 等于替用户声明这是一张 0 价行
   */
  const handleProductSelect = (index: number) => {
    const product = products().find(p => p.id === createForm.value.items[index].product_id);
    if (product) {
      // 出参 price 是 DECIMAL 字符串（NULL→null）：有值才 Number 归一，缺值留空交 items 必填校验拦
      const rawPrice = product.price;
      createForm.value.items[index].unit_price =
        rawPrice == null || rawPrice === '' ? undefined : Number(rawPrice);
      calculateSubtotal(createForm.value.items[index]);
    }
    // 换产品时清色号和已解析映射，也清掉上一行遗留的拒绝原因
    createForm.value.items[index].color_id = null;
    createForm.value.items[index].resolved = null;
    createForm.value.items[index].resolve_error = null;
  };

  /**
   * 选择色号后触发 resolveSkuMapping（需 supplier_id 与 product_id 与 color_id 均已填）
   * 解析被拒时把后端点名的原因挂到该行，由"供应商标识"列就地显示并在提交前点名行号，
   * 这里不再另弹一条固定文案（拦截器弹的就是后端原因，再叠一条泛化文案只会与它互相矛盾）
   */
  const handleColorSelect = async (index: number) => {
    const item = createForm.value.items[index];
    const supplierId = createForm.value.supplier_id;
    if (!supplierId || !item.product_id || !item.color_id) {
      item.resolved = null;
      item.resolve_error = null;
      return;
    }
    // 发请求前先清空上一轮的解析结果。换供应商时在途窗口里那一列若仍挂着上一个供应商的对照，
    // 提交门认的就是这个 resolved，等于把别的供应商标识当成当前行的事实
    item.resolved = null;
    item.resolve_error = null;
    try {
      const res = await resolveSkuMapping({
        product_id: item.product_id,
        color_id: item.color_id,
        supplier_id: supplierId,
      });
      const m = res.data;
      if (m) {
        item.resolved = {
          supplier_product_code: m.supplier_product_code,
          supplier_color_no: m.supplier_color_no ?? '',
          supplier_price: m.supplier_price,
        };
        // 默认单价取协议价，允许现谈覆盖；协议价取不到有限数值就保持留空，
        // 由 items 必填校验拦，不静默补 0（显式 0 只能由人工敲）
        if (m.supplier_price != null) {
          const mappedPrice = Number(m.supplier_price);
          if (Number.isFinite(mappedPrice)) {
            item.unit_price = mappedPrice;
            calculateSubtotal(item);
          }
        }
      } else {
        // 200 但 data 为空的异常出参不能停在"正在校验"那一态，按解析失败挂上兜底原因
        item.resolve_error = i18n.global.t('message.skuMappingResolveFailed');
      }
    } catch (error: unknown) {
      item.resolve_error = resolveFailureReason(error);
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
        item.resolve_error = null;
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
    // 无对照即拒绝转采购。要报出第几行和这一行被拒的原因，只说"存在未建立SKU对照的明细行"
    // 时操作员不知道该去SKU对照表补哪一条，尤其是产品级通用对照存在而本色号级对照缺失的情形
    const unresolved = createForm.value.items
      .map((item, idx) => ({ item, row: idx + 1 }))
      .filter(
        ({ item }) => item.product_id && item.quantity > 0 && item.color_id && !item.resolved
      );
    if (unresolved.length > 0) {
      creating.value = false;
      const first = unresolved[0];
      msg.error('skuMappingUnresolvedRows', {
        count: unresolved.length,
        row: first.row,
        reason: first.item.resolve_error ?? i18n.global.t('message.skuMappingResolveNotRun'),
      });
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
          // unit_price 已由上方 items 必填校验保证为数值（0 为人工显式声明的合法值），
          // ! 仅收敛类型不设默认
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
