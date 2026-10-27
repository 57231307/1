/**
 * useSkuMappingBatch - SKU 对照表批量添加（多行录入）composable
 *
 * 形态：表头定住「我方产品 + 供应商 + 供应商商品」三件主档，一行 = 该组合下的一条完整对照
 * （我方色号 + 可选供应商色号 + 协议价/最小起订量/交期/优先级/主供/启用）。
 * 勾选多个我方色号即一次生成多行，也可手工增删行；每行都带齐一条对照的全部字段。
 *
 * 校验分两层，口径同源、各不缺席：
 * - 界面先跑（本文件 validateRows）：提交前把「行号 + 字段 + 成因」汇总点名，
 *   与已有对照重复、批内互撞在此就报出，不靠数据库报错当界面反馈；
 * - 后端 `batch_create`（POST /purchase/sku-mappings/batch）再跑一遍并保证整批原子，
 *   任一行不合格整批零落库；返回的 errors[].row 是 items 的 1 基下标，回填到同一行。
 *
 * 取数口径（防分页截断）：
 * - 我方色号：GET /products/{id}/colors 不传 page 即后端不分页，一次取全，勾选面不缺项；
 * - 已有对照：按 (产品, 供应商) 逐页取到 total 为止，不受列表默认 page_size=20 截断；
 *   触到翻页护栏时 existingTruncated 置真并在界面如实说明「查重面不完整」，不谎称无重复。
 */
import { computed, reactive, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { logAuxLoadFailure } from '@/utils/logger';
import {
  batchCreateSkuMappings,
  getSkuMappingList,
  getProductColors,
  type CreateSkuMappingPayload,
  type SkuMappingImportError,
} from '@/api/sku-mapping';
import {
  getSupplierProductList,
  getSupplierProductColorList,
  type SupplierProduct,
  type SupplierProductColor,
} from '@/api/supplier-product';
import type { ProductColor } from '@/api/product';
import {
  hasCellText,
  isMoneyText,
  isUnsignedIntText,
} from '@/views/sku-mapping/composables/numericCell';

/** 批量录入的一行。数值列一律字符串承载（el-input + 校验器，不用带加减按钮的数字输入框）。 */
export interface SkuMappingBatchRow {
  /** 表格行标识，仅界面用，不进请求体 */
  _key: number;
  product_color_id: number | null;
  color_no: string;
  supplier_product_color_id: number | null;
  supplier_color_no: string;
  supplier_price: string;
  min_order_quantity: string;
  lead_time: string;
  priority: string;
  is_primary: boolean;
  is_enabled: boolean;
}

/** 界面侧的行级不合格明细，字段名与后端 errors[].column 同一套请求体键名。 */
export interface SkuMappingBatchRowError {
  /** 1 基行号，与后端 errors[].row 同口径 */
  row: number;
  column: string;
  message: string;
}

/**
 * 与后端 batch_create_mappings 的 MAX_BATCH_CREATE_ROWS 同口径（后端是唯一判据，
 * 超限照样拒）：界面提前拦住并指路改用批量导入，避免用户白填几百行才收到拒绝。
 */
const MAX_BATCH_ROWS = 500;

/** 已有对照逐页取数的翻页护栏（每页 100 条 × 50 页），触顶即如实提示查重面不完整。 */
const EXISTING_PAGE_SIZE = 100;
const EXISTING_MAX_PAGES = 50;

export function useSkuMappingBatch(onSuccess: () => void) {
  const { t } = useI18n({ useScope: 'global' });

  const batchDialogVisible = ref(false);
  const submitting = ref(false);

  const head = reactive({
    product_id: undefined as number | undefined,
    supplier_id: undefined as number | undefined,
    supplier_product_id: undefined as number | undefined,
  });

  /** 表头级的必填缺失提示（不属于某一行，故不带行号） */
  const headErrors = ref<string[]>([]);
  /** 界面先跑出的行级不合格明细 */
  const localErrors = ref<SkuMappingBatchRowError[]>([]);
  /** 后端回来的逐行失败明细（整批未写入时才有值） */
  const serverErrors = ref<SkuMappingImportError[]>([]);
  /** 上一次提交的结果概要，界面就地显示（失败原因不再另弹一次，避免与拦截器重复） */
  const lastSummary = ref<{ ok: boolean; text: string } | null>(null);

  const rows = reactive<SkuMappingBatchRow[]>([]);
  let rowKeySeed = 0;

  // ---- 数据源 ----
  const colorOptions = ref<ProductColor[]>([]);
  const colorLoading = ref(false);
  const supplierProducts = ref<SupplierProduct[]>([]);
  const supplierProductLoading = ref(false);
  const pinnedSupplierProduct = ref<SupplierProduct | null>(null);
  const supplierColors = ref<SupplierProductColor[]>([]);
  const supplierColorLoading = ref(false);
  /** 供应商色号 label 缓存：远程搜索换页后，已选行仍能显示色号而非空标签 */
  const supplierColorLabelCache = ref(new Map<number, string>());

  // ---- 既有对照（查重面）----
  const existingColorIds = ref(new Set<number>());
  const existingTruncated = ref(false);
  const existingLoading = ref(false);

  const nextRowKey = () => {
    rowKeySeed += 1;
    return rowKeySeed;
  };

  const supplierColorLabel = (c: SupplierProductColor) => `${c.color_no} - ${c.color_name}`;

  const cacheSupplierColor = (c: SupplierProductColor) => {
    supplierColorLabelCache.value.set(c.id, supplierColorLabel(c));
  };

  // ============ 数据加载 ============

  /** 一次取全该产品的色号（不传 page 即后端不分页），避免勾选面被默认页大小截断。 */
  const loadColors = async (productId: number, keyword?: string) => {
    colorLoading.value = true;
    try {
      const res = await getProductColors(productId, { keyword });
      colorOptions.value = res.data;
    } catch (error: unknown) {
      logAuxLoadFailure('[sku-mapping-batch] loadColors error:', error);
      colorOptions.value = [];
      ElMessage.warning(t('skuMapping.batch.loadColorsFailed'));
    } finally {
      colorLoading.value = false;
    }
  };

  const loadSupplierProducts = async (supplierId: number, keyword?: string) => {
    supplierProductLoading.value = true;
    try {
      const res = await getSupplierProductList({
        supplier_id: supplierId,
        keyword,
        page: 1,
        page_size: 50,
      });
      supplierProducts.value = res.data.items;
    } catch (error: unknown) {
      logAuxLoadFailure('[sku-mapping-batch] loadSupplierProducts error:', error);
      supplierProducts.value = [];
      ElMessage.warning(t('skuMapping.batch.loadSupplierProductsFailed'));
    } finally {
      supplierProductLoading.value = false;
    }
  };

  /** 供应商色号由表头与各行共用，故一次加载即可覆盖全部行的下拉。 */
  const loadSupplierColors = async (supplierProductId: number, keyword?: string) => {
    supplierColorLoading.value = true;
    try {
      const res = await getSupplierProductColorList({
        supplier_product_id: supplierProductId,
        keyword,
        page: 1,
        page_size: 50,
      });
      supplierColors.value = res.data.items;
      res.data.items.forEach(cacheSupplierColor);
    } catch (error: unknown) {
      logAuxLoadFailure('[sku-mapping-batch] loadSupplierColors error:', error);
      supplierColors.value = [];
      ElMessage.warning(t('skuMapping.batch.loadSupplierColorsFailed'));
    } finally {
      supplierColorLoading.value = false;
    }
  };

  /**
   * 取该 (产品, 供应商) 下已有对照的我方色号集合，用于「重复对照先提示、下拉里直接禁选」。
   * 逐页取到 total 为止；触护栏则 existingTruncated 置真（宁可提示不完整，不可谎报无重复）。
   */
  const loadExistingKeys = async () => {
    existingColorIds.value = new Set();
    existingTruncated.value = false;
    if (!head.product_id || !head.supplier_id) return;

    existingLoading.value = true;
    try {
      let page = 1;
      let collected = 0;
      let total = Number.POSITIVE_INFINITY;
      while (collected < total && page <= EXISTING_MAX_PAGES) {
        const res = await getSkuMappingList({
          product_id: head.product_id,
          supplier_id: head.supplier_id,
          page,
          page_size: EXISTING_PAGE_SIZE,
        });
        total = res.data.total;
        const items = res.data.items;
        for (const item of items) {
          // 只记「本批可能撞上的色号」：色号为空的行是产品级对照，与本批按色号生成的行不同键
          if (item.product_color_id !== null) existingColorIds.value.add(item.product_color_id);
        }
        // 空页即停：total 与实际可读页不一致时（并发删除）不应无限翻页
        if (items.length === 0) break;
        collected += items.length;
        page += 1;
      }
      existingTruncated.value = collected < total;
    } catch (error: unknown) {
      logAuxLoadFailure('[sku-mapping-batch] loadExistingKeys error:', error);
      // 取不到查重面时如实标注不完整，让界面提示「以后端校验为准」，不谎称无重复
      existingTruncated.value = true;
    } finally {
      existingLoading.value = false;
    }
  };

  // ============ 表头联动 ============

  const clearRows = () => {
    rows.splice(0, rows.length);
    localErrors.value = [];
    serverErrors.value = [];
  };

  const handleHeadProductChange = async (productId?: number) => {
    head.product_id = productId;
    clearRows();
    colorOptions.value = [];
    if (productId) {
      await loadColors(productId);
      await loadExistingKeys();
    }
  };

  const handleHeadSupplierChange = async (supplierId?: number) => {
    head.supplier_id = supplierId;
    // 换供应商只作废「供应商侧」的引用：表头的供应商商品与各行已选供应商色号都属于上一个供应商，
    // 必须清；但我方色号行是本批的主体，保留，不让用户白填一遍。
    head.supplier_product_id = undefined;
    pinnedSupplierProduct.value = null;
    supplierProducts.value = [];
    supplierColors.value = [];
    supplierColorLabelCache.value = new Map();
    for (const row of rows) {
      row.supplier_product_color_id = null;
      row.supplier_color_no = '';
    }
    localErrors.value = [];
    serverErrors.value = [];
    if (supplierId) {
      await loadSupplierProducts(supplierId);
      await loadExistingKeys();
    }
  };

  const handleHeadSupplierProductChange = async (supplierProductId?: number) => {
    head.supplier_product_id = supplierProductId;
    const picked = supplierProducts.value.find(p => p.id === supplierProductId) ?? null;
    pinnedSupplierProduct.value = picked;
    supplierColors.value = [];
    // 各行原选的供应商色号属于上一个供应商商品，换商品后一律失效
    for (const row of rows) {
      row.supplier_product_color_id = null;
      row.supplier_color_no = '';
    }
    if (supplierProductId) {
      await loadSupplierColors(supplierProductId);
    }
  };

  // ============ 行集合 ============

  const makeRow = (color?: ProductColor): SkuMappingBatchRow => ({
    _key: nextRowKey(),
    product_color_id: color ? color.id : null,
    color_no: color ? color.color_no : '',
    supplier_product_color_id: null,
    supplier_color_no: '',
    supplier_price: '',
    min_order_quantity: '',
    lead_time: '',
    priority: '1',
    is_primary: false,
    is_enabled: true,
  });

  /** 勾选项 → 行集合：新增按色号表顺序补行，取消即删行，保留已填写行的其它列。 */
  const syncRowsWithSelection = (colorIds: number[]) => {
    const picked = new Set(colorIds);
    for (let i = rows.length - 1; i >= 0; i -= 1) {
      const id = rows[i].product_color_id;
      if (id !== null && !picked.has(id)) rows.splice(i, 1);
    }
    const already = new Set(rows.map(r => r.product_color_id));
    for (const color of colorOptions.value) {
      if (picked.has(color.id) && !already.has(color.id)) rows.push(makeRow(color));
    }
    localErrors.value = [];
    serverErrors.value = [];
  };

  const selectedColorIds = computed<number[]>({
    get: () =>
      rows
        .map(r => r.product_color_id)
        .filter((id): id is number => id !== null && id !== undefined),
    set: (ids: number[]) => syncRowsWithSelection(ids),
  });

  const addBlankRow = () => {
    if (rows.length >= MAX_BATCH_ROWS) {
      ElMessage.warning(t('skuMapping.batch.maxRows', { max: MAX_BATCH_ROWS }));
      return;
    }
    rows.push(makeRow());
  };

  const removeRow = (key: number) => {
    const idx = rows.findIndex(r => r._key === key);
    if (idx >= 0) rows.splice(idx, 1);
    localErrors.value = [];
    serverErrors.value = [];
  };

  const clearAllRows = () => {
    clearRows();
  };

  /** 只重取「已有对照」集合（查重面），不动已录入的行。 */
  const reloadExistingKeys = async () => {
    await loadExistingKeys();
  };

  // ============ 选项（含重复项禁选）============

  /** 已有对照的色号在勾选中禁选并标注，从源头避免整批被后端拒。 */
  const colorSelectOptions = computed(() =>
    colorOptions.value.map(c => ({
      value: c.id,
      label: existingColorIds.value.has(c.id)
        ? `${c.color_no} - ${c.color_name}（${t('skuMapping.batch.existingTag')}）`
        : `${c.color_no} - ${c.color_name}`,
      disabled: existingColorIds.value.has(c.id),
    }))
  );

  const supplierProductOptions = computed(() => {
    const merged: SupplierProduct[] = [];
    if (pinnedSupplierProduct.value) merged.push(pinnedSupplierProduct.value);
    for (const p of supplierProducts.value) {
      if (!merged.some(m => m.id === p.id)) merged.push(p);
    }
    return merged.map(p => ({ value: p.id, label: `${p.product_code} - ${p.product_name}` }));
  });

  const supplierColorOptions = computed(() => {
    const merged: { value: number; label: string }[] = [];
    const seen = new Set<number>();
    for (const c of supplierColors.value) {
      if (!seen.has(c.id)) {
        seen.add(c.id);
        merged.push({ value: c.id, label: supplierColorLabel(c) });
      }
    }
    // 补回被远程搜索换页挤出列表的已选值，保证行内显示的是色号而不是空白
    for (const row of rows) {
      const id = row.supplier_product_color_id;
      if (id !== null && !seen.has(id)) {
        const label = supplierColorLabelCache.value.get(id);
        if (label) {
          seen.add(id);
          merged.push({ value: id, label });
        }
      }
    }
    return merged;
  });

  const searchSupplierProducts = (query: string) => {
    if (head.supplier_id) loadSupplierProducts(head.supplier_id, query);
  };

  const searchSupplierColors = (query: string) => {
    if (head.supplier_product_id) loadSupplierColors(head.supplier_product_id, query);
  };

  /** 行内改我方色号：编码随行回显；选空即撤掉该行（勾选集合随之更新） */
  const applyRowColor = (row: SkuMappingBatchRow, colorId: number | null) => {
    const picked = colorOptions.value.find(c => c.id === colorId);
    if (!picked) {
      const idx = rows.findIndex(r => r._key === row._key);
      if (idx >= 0) rows.splice(idx, 1);
      return;
    }
    row.product_color_id = picked.id;
    row.color_no = picked.color_no;
    serverErrors.value = [];
  };

  const applyRowSupplierColor = (row: SkuMappingBatchRow, colorId: number | null) => {
    row.supplier_product_color_id = colorId;
    if (colorId === null) {
      row.supplier_color_no = '';
      return;
    }
    const picked = supplierColors.value.find(c => c.id === colorId);
    if (picked) {
      cacheSupplierColor(picked);
      row.supplier_color_no = picked.color_no;
      return;
    }
    const cached = supplierColorLabelCache.value.get(colorId);
    row.supplier_color_no = cached ? cached.split(' - ')[0] : '';
  };

  // ============ 校验 ============

  const validateHead = (): string[] => {
    const errors: string[] = [];
    if (!head.product_id) errors.push(t('skuMapping.batch.headProductRequired'));
    if (!head.supplier_id) errors.push(t('skuMapping.batch.headSupplierRequired'));
    if (!head.supplier_product_id) {
      errors.push(t('skuMapping.batch.headSupplierProductRequired'));
    }
    if (rows.length === 0) errors.push(t('skuMapping.batch.noRows'));
    if (rows.length > MAX_BATCH_ROWS) {
      errors.push(t('skuMapping.batch.maxRows', { max: MAX_BATCH_ROWS }));
    }
    return errors;
  };

  /** 数值单元格是否不合格（未填=该列不写，算合格；填了就要按列的取值域判）。 */
  const invalidTextCell = (value: string, integer: boolean): boolean => {
    if (!hasCellText(value)) return false;
    return integer ? !isUnsignedIntText(value) : !isMoneyText(value);
  };

  /**
   * 逐行校验，产出「行号 + 字段 + 成因」。一次跑全、不遇错即停：
   * 用户要的是把这一批里所有不合格行一次看清，而不是改一行提交一次。
   */
  const validateRows = (): SkuMappingBatchRowError[] => {
    const errors: SkuMappingBatchRowError[] = [];
    const seenColorRow = new Map<number, number>();

    rows.forEach((row, idx) => {
      const rowNo = idx + 1;

      if (row.product_color_id === null) {
        errors.push({
          row: rowNo,
          column: 'product_color_id',
          message: t('skuMapping.batch.rowColorRequired', { row: rowNo }),
        });
      }

      if (invalidTextCell(row.supplier_price, false)) {
        errors.push({
          row: rowNo,
          column: 'supplier_price',
          message: t('skuMapping.batch.rowPriceInvalid', { row: rowNo }),
        });
      }
      if (invalidTextCell(row.min_order_quantity, false)) {
        errors.push({
          row: rowNo,
          column: 'min_order_quantity',
          message: t('skuMapping.batch.rowMoqInvalid', { row: rowNo }),
        });
      }
      if (invalidTextCell(row.lead_time, true)) {
        errors.push({
          row: rowNo,
          column: 'lead_time',
          message: t('skuMapping.batch.rowLeadTimeInvalid', { row: rowNo }),
        });
      }
      if (invalidTextCell(row.priority, true)) {
        errors.push({
          row: rowNo,
          column: 'priority',
          message: t('skuMapping.batch.rowPriorityInvalid', { row: rowNo }),
        });
      }

      if (row.product_color_id === null) return;

      const first = seenColorRow.get(row.product_color_id);
      if (first !== undefined) {
        errors.push({
          row: rowNo,
          column: 'product_color_id',
          message: t('skuMapping.batch.rowDupInBatch', { row: rowNo, first }),
        });
        return;
      }
      seenColorRow.set(row.product_color_id, rowNo);
      if (existingColorIds.value.has(row.product_color_id)) {
        errors.push({
          row: rowNo,
          column: 'product_color_id',
          message: t('skuMapping.batch.rowExists', { row: rowNo }),
        });
      }
    });

    return errors;
  };

  /** 行标识 → 该行的不合格明细（界面校验与后端点名合成一份），供表格「校验」列取用。 */
  const errorsByRowKey = computed(() => {
    const map = new Map<number, { column: string; message: string }[]>();
    const push = (rowNo: number, column: string, message: string) => {
      const row = rows[rowNo - 1];
      if (!row) return;
      const list = map.get(row._key) ?? [];
      list.push({ column, message });
      map.set(row._key, list);
    };
    for (const e of localErrors.value) push(e.row, e.column, e.message);
    for (const e of serverErrors.value) push(e.row, e.column, e.message);
    return map;
  });

  /** 提交前/被拒后要在界面逐条列出的原因（表头缺失 + 界面行校验 + 后端逐行点名）。 */
  const summaryText = computed<string[]>(() => [
    ...headErrors.value,
    ...localErrors.value.map(e => e.message),
    ...serverErrors.value.map(e => e.message),
  ]);

  const canSubmit = computed(() => !submitting.value && rows.length > 0);

  // ============ 提交 ============

  /** 行 → 请求体一行：字段名与后端 CreateMappingRequest 逐一对应，行号即数组下标。 */
  const buildPayload = (): CreateSkuMappingPayload[] =>
    rows.map(row => ({
      product_id: head.product_id as number,
      product_color_id: row.product_color_id,
      supplier_id: head.supplier_id as number,
      supplier_product_id: head.supplier_product_id as number,
      supplier_product_color_id: row.supplier_product_color_id,
      supplier_price: row.supplier_price.trim() || null,
      min_order_quantity: row.min_order_quantity.trim() || null,
      lead_time: row.lead_time.trim() === '' ? null : Number(row.lead_time.trim()),
      is_primary: row.is_primary,
      priority: row.priority.trim() === '' ? 1 : Number(row.priority.trim()),
      is_enabled: row.is_enabled,
    }));

  const submit = async () => {
    headErrors.value = validateHead();
    localErrors.value = validateRows();
    serverErrors.value = [];
    if (headErrors.value.length > 0 || localErrors.value.length > 0) {
      lastSummary.value = {
        ok: false,
        text: t('skuMapping.batch.localReject', {
          count: new Set(localErrors.value.map(e => e.row)).size,
        }),
      };
      return;
    }

    submitting.value = true;
    try {
      const res = await batchCreateSkuMappings({ items: buildPayload() });
      const result = res.data;
      if (result.created_count === result.total_count && result.errors.length === 0) {
        lastSummary.value = {
          ok: true,
          text: t('skuMapping.batch.created', { count: result.created_count }),
        };
        ElMessage.success(lastSummary.value.text);
        batchDialogVisible.value = false;
        onSuccess();
        return;
      }
      // 整批未写入：后端逐行点名的原因回填到对应录入行
      serverErrors.value = result.errors;
      lastSummary.value = {
        ok: false,
        text: t('skuMapping.batch.serverReject', { count: result.failed_count }),
      };
      await loadExistingKeys();
      onSuccess();
    } catch (error: unknown) {
      logAuxLoadFailure('[sku-mapping-batch] submit error:', error);
      // 拒绝原因由请求拦截器按后端 message 外显一次，这里只就地补一行状态，不重复弹
      lastSummary.value = { ok: false, text: t('skuMapping.batch.submitFailed') };
    } finally {
      submitting.value = false;
    }
  };

  const openBatchDialog = () => {
    head.product_id = undefined;
    head.supplier_id = undefined;
    head.supplier_product_id = undefined;
    colorOptions.value = [];
    supplierProducts.value = [];
    supplierColors.value = [];
    supplierColorLabelCache.value = new Map();
    pinnedSupplierProduct.value = null;
    existingColorIds.value = new Set();
    existingTruncated.value = false;
    headErrors.value = [];
    clearRows();
    lastSummary.value = null;
    batchDialogVisible.value = true;
  };

  return {
    batchDialogVisible,
    submitting,
    head,
    headErrors,
    localErrors,
    serverErrors,
    summaryText,
    lastSummary,
    canSubmit,
    rows,
    errorsByRowKey,
    // 数据源
    colorLoading,
    colorSelectOptions,
    supplierProductOptions,
    supplierProductLoading,
    supplierColorOptions,
    supplierColorLoading,
    selectedColorIds,
    existingLoading,
    existingCount: computed(() => existingColorIds.value.size),
    existingTruncated,
    maxBatchRows: MAX_BATCH_ROWS,
    // 动作
    handleHeadProductChange,
    handleHeadSupplierChange,
    handleHeadSupplierProductChange,
    searchSupplierProducts,
    searchSupplierColors,
    applyRowColor,
    applyRowSupplierColor,
    addBlankRow,
    removeRow,
    clearAllRows,
    reloadExistingKeys,
    openBatchDialog,
    submit,
  };
}

export type SkuMappingBatchProc = ReturnType<typeof useSkuMappingBatch>;
