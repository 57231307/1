/**
 * usePrRtn.ts - 采购退货核心 composable
 * 任务编号: P14 批 2 I-3 第 2 批（拆分原 purchase-return/index.vue）
 * 提供采购退货列表查询、表单管理、供应商/采购单/产品加载、CRUD 等核心方法
 * 业务流程（提交审批/审批/拒绝/删除）由 usePrRtnProc 提供
 * 批次 286：tableData 接入 useTableApi，移除手写分页逻辑
 */
import { ref, reactive, watch } from 'vue';
import { msg } from '@/utils/message';
import { getPurchaseOrderList, type PurchaseOrder } from '@/api/purchase';
import { getSupplierList } from '@/api/supplier';
import { getProductList, type Product } from '@/api/product';
import { getWarehouseList, type Warehouse } from '@/api/warehouse';
import {
  getPurchaseReturnById,
  getPurchaseReturnItemList,
  updatePurchaseReturn,
  createPurchaseReturn,
  createPurchaseReturnItem,
  updatePurchaseReturnItem,
  deletePurchaseReturnItem,
  type PurchaseReturn,
  type PurchaseReturnItem,
  type CreatePurchaseReturnPayload,
  type CreatePurchaseReturnItemPayload,
  type UpdatePurchaseReturnItemPayload,
} from '@/api/purchase-return';
import {
  getPurchaseInspectionById,
  getPurchaseInspectionItemList,
} from '@/api/purchase-inspection';
import { useTableApi } from '@/composables/useTableApi';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import { logger, logAuxLoadFailure } from '@/utils/logger';
import { i18n } from '@/i18n';
import { PURCHASE_RETURN_STATUS } from '@/utils/purchase-return-status';
import { RETURN_REASON_OPTIONS } from '@/constants/return-reason';
import { netFromInclusive, parseDecimalInput, roundToCent } from '@/utils/money';

/**
 * 退货明细表单行（编辑态本地结构）：提交时经 buildCreateItemPayload / buildUpdateItemPayload
 * 映射为后端 Create/UpdateReturnItemRequest，读取时经 normalizeItem 从后端 PurchaseReturnItemDto
 * 归一化而来，与响应契约解耦。
 */
export interface ReturnFormItem {
  id?: number;
  productId?: number;
  productName?: string;
  quantity: number;
  /**
   * 不含税单价（派生只读值）：由含税单价按本行税率除法反算，落 purchase_return_item.unit_price 列。
   * 提交时仍带该键，后端按它做单价钩稽门（与反算值分位一致才放行），前端不乘 (1 + 税率) 造含税。
   */
  unitPrice?: number;
  /**
   * 含税单价（录入事实，主输入列）：输入框原文，空串=未填，'0'=人工显式声明的 0 价行。
   * 税率非 0 时后端要求本值，缺值由服务层按必填族点名拒绝，系统不按税率倒造含税价。
   */
  unitPriceWithTax?: string;
  /** 税率（百分数口径，13 表示 13%）：undefined=未填，0 只能由退货员显式填写（免税或无票红冲） */
  taxPercent?: number;
  reason?: string;
  /**
   * 面料追溯维度（审批按 产品+色号+缸号+批次 精确扣库存）。undefined=未设定：
   * 新建时落库为空串（白坯/单库存行语义），编辑时不下发（不覆盖后端原值）。
   */
  colorNo?: string;
  dyeLotNo?: string;
  batchNo?: string;
}

/**
 * 质检不合格一键生成退货时的原因分类落库值。
 * `purchase_return.reason_type` 是自由文本列，其取值词表唯一来源是共用常量
 * `constants/return-reason`（value 即写入方原值）；本域不再抄一份中文常量，
 * 避免同一含义在库里分裂成两种写法。词表被改到取不到该档时显式抛错（由预填入口
 * 记录并中止），禁止用裸字面量兜底掩盖。
 */
function qualityDefectReasonType(): string {
  const option = RETURN_REASON_OPTIONS.find(
    opt => opt.labelKey === 'common.returnReason.qualityDefect'
  );
  if (!option) {
    throw new Error('[purchase-return] 共用退货原因词表缺少 qualityDefect 档，无法预填退货原因');
  }
  return option.value;
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
  // keyword/start_date/end_date 为 schema gap（后端 ReturnQueryParams 不接受，Axum 静默丢弃）
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
      logAuxLoadFailure('获取数据失败:', err);
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

  // 仓库列表（用于 warehouse_id 选择，approval 时扣减库存需要该字段）
  const warehouses = ref<Warehouse[]>([]);

  // 表单数据
  const formData = reactive({
    id: undefined as number | undefined,
    purchaseOrderId: undefined as number | undefined,
    receiptId: undefined as number | undefined,
    supplierId: undefined as number | undefined,
    returnDate: '',
    warehouseId: undefined as number | undefined,
    reasonType: '',
    reason: '',
    remarks: '',
    items: [] as ReturnFormItem[],
  });

  // 表单校验规则：supplier_id 与 reason_type 是后端 CreatePurchaseReturnRequest 的非 Option 字段，
  // 缺失会被 422 拒绝，故前端必须挡住；文案走 i18n（此前为裸中文字面量）。
  const formRules = {
    purchaseOrderId: [
      {
        required: true,
        message: i18n.global.t('purchaseReturn.validation.purchaseOrderRequired'),
        trigger: 'change',
      },
    ],
    returnDate: [
      {
        required: true,
        message: i18n.global.t('purchaseReturn.validation.returnDateRequired'),
        trigger: 'change',
      },
    ],
    supplierId: [
      {
        required: true,
        message: i18n.global.t('purchaseReturn.validation.supplierRequired'),
        trigger: 'change',
      },
    ],
    reasonType: [
      {
        required: true,
        message: i18n.global.t('purchaseReturn.validation.reasonTypeRequired'),
        trigger: 'change',
      },
    ],
    reason: [
      {
        required: true,
        message: i18n.global.t('purchaseReturn.validation.reasonRequired'),
        trigger: 'blur',
      },
    ],
  };

  // 详情数据（表头 + 明细，由 fetchDetail 回源填充）
  const detailData = ref<PurchaseReturn>({} as PurchaseReturn);
  const detailItems = ref<PurchaseReturnItem[]>([]);
  const detailItemsLoading = ref(false);

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

  /** 加载供应商列表 */
  const fetchSuppliers = async () => {
    try {
      const res = await getSupplierList({ page: 1, page_size: 1000 });
      suppliers.value = (res.data?.items || []).map((s: { id: number; supplier_name: string }) => ({
        id: s.id,
        name: s.supplier_name,
      }));
    } catch (error) {
      logAuxLoadFailure('获取供应商列表失败', error);
    }
  };

  /** 加载采购订单列表 */
  const fetchPurchaseOrders = async () => {
    try {
      const res = await getPurchaseOrderList({ page: 1, page_size: 500 });
      purchaseOrders.value = (res.data?.items || []) as unknown as PurchaseOrder[];
    } catch (error) {
      logAuxLoadFailure('获取采购订单列表失败', error);
    }
  };

  /** 加载产品列表 */
  const fetchProducts = async () => {
    try {
      const res = await getProductList({ page: 1, page_size: 1000 });
      const items = (res.data?.items ||
        (res.data as unknown as { list?: Product[] })?.list ||
        []) as Product[];
      products.value = items.map(p => ({
        id: p.id,
        name: p.product_name,
        price: 0,
      }));
    } catch (error) {
      logAuxLoadFailure('获取产品列表失败', error);
    }
  };

  /** 加载仓库列表（供 warehouse_id 选择，后端 CreatePurchaseReturnRequest 接受该字段） */
  const fetchWarehouses = async () => {
    try {
      const res = await getWarehouseList({ page: 1, page_size: 1000 });
      warehouses.value = res.data?.items || [];
    } catch (error) {
      logAuxLoadFailure('获取仓库列表失败', error);
    }
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
      receiptId: undefined,
      supplierId: undefined,
      returnDate: '',
      warehouseId: undefined,
      reasonType: '',
      reason: '',
      remarks: '',
      items: [],
    });
  };

  /**
 * 编辑回显的含税单价原文（价税分离的录入事实，主输入列）。
 * 入参，raw 是后端退货明细行（subtotal 与 tax_amount 与 total_amount 三列已落库）。
 * 出参是含税单价输入框原文，空串表示还原不出、须人工重录。
 * 取法只有除法还原这一条腿，与后端 utils/tax.rs 的 recover_inclusive_unit_price 同源：
 * 本行有净额口径折扣或数量为 0 时标价含税额还原不出来，一律留空要求录入，
 * 绝不用「不含税单价 乘 1 加税率」倒造含税价。
 * 传给谁，normalizeItem 的行映射，写入 ReturnFormItem.unitPriceWithTax。
 */
const recoverInclusiveUnitPriceText = (raw: PurchaseReturnItem): string => {
  if (raw.discount_amount !== 0 || raw.quantity_returned <= 0) return '';
  return String(roundToCent(raw.total_amount / raw.quantity_returned));
};

/**
 * 逐行派生列重算（方向口径与后端 utils/tax.rs 与前端 money.ts 单源一致）。
 * 入参，row 是本表一行；读含税单价原文与税率，写派生的不含税单价。
 * 含税成交额 = 含税单价 乘 数量（事实聚合，税率不进乘法），不含税单价 = 含税单价 ÷ (1 + 税率% ÷ 100)
 * 先舍到分；税额与含税额小计由 money.ts 取差额，本函数不存在任何乘税率造税额或造含税的写法。
 * 出参 undefined（缺含税单价或税率非法）时派生单价回到 0 无值状态，由提交前的必填校验点名拦下，
 * 不猜档位、不补 0 冒充算得。
 * 传给谁，组件的含税单价与税率输入框 change 与产品带出腿。
 */
export const applyDerivedPrices = (row: ReturnFormItem): void => {
  const inclusiveUnit = parseDecimalInput(row.unitPriceWithTax);
  const rate = row.taxPercent;
  if (
    inclusiveUnit === undefined ||
    !Number.isFinite(inclusiveUnit) ||
    inclusiveUnit < 0 ||
    rate === undefined ||
    !Number.isFinite(rate) ||
    rate < 0
  ) {
    row.unitPrice = 0;
    return;
  }
  row.unitPrice = netFromInclusive(inclusiveUnit, rate);
};

/** 从后端 PurchaseReturnItemDto 归一化为表单本地结构 */
  const normalizeItem = (raw: PurchaseReturnItem): ReturnFormItem => ({
    id: raw.id,
    productId: raw.material_id,
    productName: raw.material_name ?? undefined,
    quantity: raw.quantity_returned,
    unitPrice: raw.unit_price,
    // 含税单价回显只准由已落库成交含税额除数量还原（与后端 recover_inclusive_unit_price 同口径）；
    // 本行有折扣或数量为 0 时还原不出标价含税单价，留空由退货员重新录入，绝不按税率倒造一个。
    unitPriceWithTax: recoverInclusiveUnitPriceText(raw),
    taxPercent: raw.tax_rate,
    reason: raw.notes ?? undefined,
    // 维度回读：后端有值才回填；缺省保持 undefined（编辑提交时不下发，避免误覆盖原维度）。
    // 依赖后端 PurchaseReturnItemDto 补 SELECT 这三列（见 api 注释），否则编辑态维度为空态需退货员重选。
    colorNo: raw.color_no ?? undefined,
    dyeLotNo: raw.dye_lot_no ?? undefined,
    batchNo: raw.batch_no ?? undefined,
  });

  /**
   * 从质检单派生退货预填数据（新建态，不携原 id）
   * 与 quotation copy 同构：加载源单数据 → 以新建态预填 → 保存走 POST 建新单
   *
   * 明细口径（契约以出参为准）：GET /purchase/inspections/{id}/items 直接序列化
   * `purchase_inspection_item::Model`（backend/src/services/purchase_inspection_service
   * 的 list_inspection_items 直接序列化实体），
   * 业务列只有 product_id / item_name / qualified_quantity / unqualified_quantity / remark，
   * 没有 failed_quantity / passed_quantity / product_name / defect_reason 这些键
   * （历史前端按它们取值，恒 undefined → `undefined > 0` 恒 false → 明细永远预填不出行，
   *  一键生成退货名存实亡）。不合格数量取 `unqualified_quantity`，DECIMAL 经 JSON 是字符串，
   * 需 Number() 归一。
   */
  const prepareFromInspection = async (inspectionId: number) => {
    try {
      const reasonType = qualityDefectReasonType();
      const [inspRes, itemsRes] = await Promise.all([
        getPurchaseInspectionById(inspectionId),
        getPurchaseInspectionItemList(inspectionId),
      ]);
      const insp = inspRes.data;
      if (!insp) {
        logger.error('[purchase-return] 质检单数据为空', { inspectionId });
        return false;
      }
      // 表头预填：采购订单/入库单/供应商/退货日期
      Object.assign(formData, {
        id: undefined,
        purchaseOrderId: insp.order_id ?? undefined,
        receiptId: insp.receipt_id ?? undefined,
        supplierId: insp.supplier_id,
        returnDate: new Date().toISOString().slice(0, 10),
        warehouseId: undefined,
        reasonType,
        reason: insp.defect_description || '',
        remarks: insp.notes || '',
        items: [],
      });
      // 明细预填：只取不合格数量 > 0 的检验明细行，退货数量 = 不合格数量
      formData.items = itemsRes.data.items
        .filter(item => Number(item.unqualified_quantity) > 0)
        .map(item => ({
          productId: item.product_id,
          quantity: Number(item.unqualified_quantity),
          // 单价与「添加明细」生成的空行同初值 0：purchase_inspection_item 不落单价，无来源可带，
          // 由退货员按实填写（不是用主数据/采购价凑一个假单价）
          unitPrice: 0,
          // 含税单价与税率同为录入事实，质检明细两列都无来源可带，留空由退货员按实填写，
          // 不预填任何税率档位
          unitPriceWithTax: '',
          taxPercent: undefined,
          reason: item.remark ?? undefined,
          // 质检明细 purchase_inspection_item 不落 色号/缸号/批次（后端无该三列，无来源可带），
          // 故维度不预填、不写死、不从空兜底：留 undefined 由退货员按实际退的库存行选定。
        }));
      if (formData.items.length === 0) {
        logger.error('[purchase-return] 质检单无不合格明细，退货单无可派生行', { inspectionId });
      }
      return true;
    } catch (error) {
      logger.error('[purchase-return] 从质检单预填退货表单失败', error);
      return false;
    }
  };

  /**
   * 准备编辑表单（父组件需自行打开对话框）
   * 表头由调用方传入（来自 fetchDetail），明细由 loadFormItems 异步加载（独立端点，表头响应不含 items）
   */
  const prepareEdit = (row: PurchaseReturn) => {
    Object.assign(formData, {
      id: row.id,
      purchaseOrderId: row.order_id ?? undefined,
      receiptId: row.receipt_id ?? undefined,
      supplierId: row.supplier_id,
      returnDate: row.return_date,
      warehouseId: row.warehouse_id ?? undefined,
      reasonType: row.reason_type ?? '',
      reason: row.reason_detail ?? '',
      remarks: row.notes ?? '',
      items: [],
    });
    removedItemIds.value = [];
    if (row.id) {
      loadFormItems(row.id).catch((error: unknown) => {
        logger.error('[purchase-return] 明细回填失败', error);
        msg.error('loadFailed');
      });
    }
  };

  /**
   * 异步回源编辑明细（表头响应不含 items，明细是独立端点）
   * 按 returnId 校验，避免快速切换行时把上一张单的明细写进当前表单
   */
  const loadFormItems = async (returnId: number) => {
    const res = await getPurchaseReturnItemList(returnId);
    if (formData.id !== returnId) return;
    formData.items = res.data.map(item => normalizeItem(item));
  };

  const fetchDetail = async (id: number) => {
    try {
      const res = await getPurchaseReturnById(id);
      detailData.value = res.data;
      detailItemsLoading.value = true;
      try {
        const itemsRes = await getPurchaseReturnItemList(id);
        detailItems.value = itemsRes.data;
      } catch (error) {
        logger.error('获取退货明细失败:', error);
        detailItems.value = [];
      } finally {
        detailItemsLoading.value = false;
      }
    } catch (error) {
      logger.error('获取详情失败:', error);
    }
  };

  /** 采购订单变化：派生供应商 */
  const handleOrderChange = (orderId: number) => {
    const order = purchaseOrders.value.find(o => o.id === orderId);
    if (order) {
      formData.supplierId = (order as unknown as { supplier_id?: number }).supplier_id;
      formData.items = [
        { productId: undefined, productName: '', quantity: 1, unitPrice: 0, unitPriceWithTax: '' },
      ];
    }
  };

  /** 添加明细 */
  const handleAddItem = () => {
    formData.items.push({
      productId: undefined,
      productName: '',
      quantity: 1,
      unitPrice: 0,
      // 含税单价与税率留空由退货员填写（0 只能由人工显式敲出），税率不预填任何档位
      unitPriceWithTax: '',
      taxPercent: undefined,
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

  /** 产品变化（联动名称与参考价；参考价带进**含税单价**主输入列，可现谈覆盖） */
  const handleProductChange = (row: ReturnFormItem, productId: number) => {
    const product = products.value.find(p => p.id === productId);
    if (product) {
      row.productName = product.name;
      // 带出只是建议值：产品参考价按含税口径进主输入列，不含税单价随后由税率除法反算，
      // 无参考价就留空，不补 0、不乘税率
      row.unitPriceWithTax = product.price == null ? '' : String(product.price);
      applyDerivedPrices(row);
    }
  };

  /** 构造新建明细请求体（对齐 CreatePurchaseReturnItemPayload）
   *  含税单价与税率同为必填录入项：后端缺任一键按必填族点名拒绝（VALIDATION_ERROR），
   *  前端在 handleFormSubmit 里先拦下，unit_price 是含税单价按税率除法反算的派生值。 */
  const buildCreateItemPayload = (
    it: ReturnFormItem,
    idx: number
  ): CreatePurchaseReturnItemPayload => ({
    line_no: idx + 1,
    material_id: it.productId as number,
    quantity_returned: it.quantity ?? 0,
    unit_price: it.unitPrice ?? 0,
    // rust_decimal 走线是字符串：含税单价按原文下发，不做二次浮点运算
    unit_price_with_tax: it.unitPriceWithTax,
    tax_rate: it.taxPercent,
    notes: it.reason || undefined,
    // 面料追溯维度：有值随 POST 下发；未填（undefined）时 JSON 省略该键，后端 unwrap_or_default 落空串（白坯）。
    color_no: it.colorNo,
    dye_lot_no: it.dyeLotNo,
    batch_no: it.batchNo,
  });

  /** 构造更新明细请求体（对齐 UpdateReturnItemRequest 三态契约，RFC 7386）：
   *  表单已回显原值、恒送当前值=覆盖；NOT NULL 列（含追溯三列 NOT NULL DEFAULT ''）
   *  禁止送 null——"改回白坯/空值"送空串；notes 为 DB 可空列，UI 清空 ⇒ 送显式 null
   * （=清空为 NULL），塌成 `|| undefined` 省略会"改了不生效"。 */
  const buildUpdateItemPayload = (it: ReturnFormItem): UpdatePurchaseReturnItemPayload => ({
    quantity_returned: it.quantity ?? 0,
    unit_price: it.unitPrice ?? 0,
    // 含税单价与税率恒送当前值（覆盖语义）：二者已由 handleFormSubmit 校验为已填且合法，
    // 折扣行的标价含税单价还原不出来（后端 recover_inclusive_unit_price 会点名拒绝），
    // 故必须由退货员按实录入，前端不带任何按税率倒造的兜底
    unit_price_with_tax: it.unitPriceWithTax,
    tax_rate: it.taxPercent,
    notes: it.reason || null,
    // 追溯维度为 NOT NULL DEFAULT '' 列：恒送当前值（空串=白坯合法值），不得送 null
    color_no: it.colorNo,
    dye_lot_no: it.dyeLotNo,
    batch_no: it.batchNo,
  });

  /**
   * 提交表单（新建/编辑）
   * 新建：显式构造 CreatePurchaseReturnPayload（不含 items），先建单再逐条写明细
   * 编辑：updatePurchaseReturn 仅接受 reason_type/reason_detail/notes（UpdatePurchaseReturnRequest），
   *       明细通过 item 级端点增量同步
   */
  const handleFormSubmit = async (isEdit: boolean): Promise<boolean> => {
    try {
      const validItems = formData.items.filter(e => (e.productId as number) > 0);
      if (validItems.length === 0) {
        msg.warning('pleaseAddReturnDetail');
        return false;
      }
      // 含税单价与税率是退货行的必填录入项（后端按必填族点名拒绝，缺键不回落任何档位）：
      // 界面必须先点名拦下，不能让退货员以为已保存而少一行明细。
      const missingInclusive = validItems.find(
        e => parseDecimalInput(e.unitPriceWithTax) === undefined
      );
      if (missingInclusive) {
        msg.warning('pleaseFillReturnInclusivePrice');
        return false;
      }
      const invalidInclusive = validItems.find(e => {
        const parsed = parseDecimalInput(e.unitPriceWithTax);
        return parsed !== undefined && (!Number.isFinite(parsed) || parsed < 0);
      });
      if (invalidInclusive) {
        msg.warning('invalidReturnInclusivePrice');
        return false;
      }
      const missingRate = validItems.find(e => e.taxPercent === undefined);
      if (missingRate) {
        msg.warning('pleaseFillReturnTaxRate');
        return false;
      }
      const invalidRate = validItems.find(
        e => e.taxPercent !== undefined && (!Number.isFinite(e.taxPercent) || e.taxPercent < 0)
      );
      if (invalidRate) {
        msg.warning('invalidReturnTaxRate');
        return false;
      }
      if (isEdit && formData.id) {
        // 三态语义（后端 UpdatePurchaseReturnRequest DoubleOption，RFC 7386）：
        // reason_type/reason_detail/notes 均为 DB 可空列，UI 清空 ⇒ 送显式 null（=清空为 NULL）；
        // reason_type/reason_detail 表单恒回显选择值，未清空时按当前值覆盖。
        await updatePurchaseReturn(formData.id, {
          reason_type: formData.reasonType,
          reason_detail: formData.reason || null,
          notes: formData.remarks || null,
        });
        let idx = 0;
        for (const it of validItems) {
          if (it.id) {
            await updatePurchaseReturnItem(formData.id, it.id, buildUpdateItemPayload(it));
          } else {
            await createPurchaseReturnItem(formData.id, buildCreateItemPayload(it, idx));
          }
          idx += 1;
        }
        for (const itemId of removedItemIds.value) {
          await deletePurchaseReturnItem(formData.id, itemId);
        }
        removedItemIds.value = [];
        msg.success('updateSuccess');
      } else {
        const headBody: CreatePurchaseReturnPayload = {
          order_id: formData.purchaseOrderId,
          receipt_id: formData.receiptId,
          supplier_id: formData.supplierId as number,
          return_date: formData.returnDate,
          warehouse_id: formData.warehouseId,
          reason_type: formData.reasonType,
          reason_detail: formData.reason || undefined,
          notes: formData.remarks || undefined,
        };
        const created = await createPurchaseReturn(headBody);
        const returnId = created.data?.id;
        if (returnId) {
          let idx = 0;
          for (const it of validItems) {
            await createPurchaseReturnItem(returnId, buildCreateItemPayload(it, idx));
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
    loadIfNot('warehouses', fetchWarehouses, hasLoaded);
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
    // 供应商/采购单/产品/仓库
    suppliers,
    purchaseOrders,
    products,
    warehouses,
    // 表单
    formData,
    formRules,
    prepareCreate,
    prepareFromInspection,
    prepareEdit,
    handleOrderChange,
    handleAddItem,
    handleRemoveItem,
    handleProductChange,
    handleFormSubmit,
    // 详情
    detailData,
    detailItems,
    detailItemsLoading,
    fetchDetail,
    // 初始化
    initLoad,
  });
}
