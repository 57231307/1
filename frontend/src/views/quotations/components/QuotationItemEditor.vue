<!--
  报价单明细编辑器
  - 通过 v-model 双向绑定 items 数组
  - 支持添加/删除明细行
  - 按单据税率（父组件传入的百分数税率）换算含税单价，组件不持有税率档位
  - 产品选择为服务端搜索（GET /products 分页上限 100，本地全量过滤会截断，改按名称/编码搜索取首页，
    选中后按 id 回查主档缓存），色号下拉按产品过滤
  - 选中产品后带出：单位与规格写入明细行（unit、specification 是 sales_quotation_items 实列，随保存落库）；
    产品编码、成分、幅宽、克重只读展示自产品主档（明细表无对应列，不落库；报价转销售订单换算链实时回查主档）
  - 切换/清空产品时带出值随新值重置或清空，不残留上一产品取值
-->
<template>
  <div class="item-editor">
    <el-button type="primary" plain @click="handleAdd">
      <el-icon><Plus /></el-icon>
      {{ t('quotations.itemEditor.addProduct') }}
    </el-button>

    <el-table
      :data="modelValue"
      border
      style="margin-top: 10px"
      :empty-text="t('quotations.itemEditor.emptyText')"
      :aria-label="t('quotations.itemEditor.tableAriaLabel')"
    >
      <el-table-column type="index" label="#" width="50" align="center" />

      <el-table-column :label="t('quotations.itemEditor.colProduct')" min-width="180">
        <template #default="{ row }">
          <el-select
            v-model="row.product_id"
            filterable
            remote
            :remote-method="searchProducts"
            :loading="productSearchLoading"
            clearable
            :placeholder="t('quotations.itemEditor.selectProduct')"
            style="width: 100%"
            @change="(v: number | undefined) => handleProductChange(row, v)"
          >
            <el-option
              v-for="p in optionsFor(row)"
              :key="p.id"
              :label="p.product_name"
              :value="p.id"
            />
          </el-select>
        </template>
      </el-table-column>

      <!--
        以下四列为产品主档只读带出列：取值 GET /products 列表出参与 GET /products/{id} 详情出参
        （后端 models/product.rs::Model 全字段 serde 出参，Decimal 序列化为字符串），
        仅用于界面核对，不进提交体；主档未配置该字段显示占位符，绝不以 0 或默认值充数。
      -->
      <el-table-column :label="t('quotations.itemEditor.colProductCode')" min-width="140">
        <template #default="{ row }">{{ masterText(masterOf(row)?.product_code) }}</template>
      </el-table-column>

      <el-table-column :label="t('quotations.itemEditor.colComposition')" min-width="140">
        <template #default="{ row }">{{ masterText(masterOf(row)?.fabric_composition) }}</template>
      </el-table-column>

      <el-table-column :label="t('quotations.itemEditor.colWidth')" min-width="110">
        <template #default="{ row }">{{ masterText(masterOf(row)?.width, 'cm') }}</template>
      </el-table-column>

      <el-table-column :label="t('quotations.itemEditor.colGramWeight')" min-width="120">
        <template #default="{ row }">{{ masterText(masterOf(row)?.gram_weight, 'g/m²') }}</template>
      </el-table-column>

      <el-table-column :label="t('quotations.itemEditor.colColor')" min-width="120">
        <template #default="{ row }">
          <el-select
            v-model="row.color_id"
            clearable
            :placeholder="t('quotations.itemEditor.noColor')"
            style="width: 100%"
            :disabled="!row.product_id"
          >
            <el-option
              v-for="c in row._colors || []"
              :key="c.id"
              :label="c.color_code || c.color_name"
              :value="c.id"
            />
          </el-select>
        </template>
      </el-table-column>

      <el-table-column :label="t('quotations.itemEditor.colSpec')" min-width="140">
        <template #default="{ row }">
          <el-input
            v-model="row.specification"
            :placeholder="t('quotations.itemEditor.specPlaceholder')"
          />
        </template>
      </el-table-column>

      <el-table-column :label="t('quotations.itemEditor.colUnit')" min-width="110">
        <template #default="{ row }">
          <!--
            单位由所选产品的交易单位（product.unit）只读带出并锁定：
            后端 validate_item_units_against_products 要求报价行 unit 逐字符等于 product.unit，
            不一致即 400，故此处不可自由选/手输。无产品时禁用。
          -->
          <el-input
            v-model="row.unit"
            disabled
            :placeholder="t('quotations.itemEditor.unitFollowProductPlaceholder')"
          />
        </template>
      </el-table-column>

      <el-table-column :label="t('quotations.itemEditor.colQuantity')" min-width="120">
        <template #default="{ row }">
          <NumericInput v-model="row.quantity" :min="0" :precision="2" style="width: 100%" />
        </template>
      </el-table-column>

      <el-table-column :label="t('quotations.itemEditor.colUnitPrice')" min-width="130">
        <template #default="{ row }">
          <!-- 单价提交后经 change 转发 按单据税率重算含税单价（模型已由绑定器归一写回） -->
          <NumericInput
            v-model="row.unit_price"
            :min="0"
            :precision="2"
            style="width: 100%"
            @change="recalcTax(row, row.unit_price)"
          />
        </template>
      </el-table-column>

      <el-table-column :label="t('quotations.itemEditor.colUnitPriceWithTax')" min-width="130">
        <template #default="{ row }">
          <NumericInput
            v-model="row.unit_price_with_tax"
            :min="0"
            :precision="2"
            style="width: 100%"
          />
        </template>
      </el-table-column>

      <el-table-column :label="t('quotations.itemEditor.colAmount')" min-width="130" align="right">
        <template #default="{ row }"> {{ currency }} {{ formatAmount(row.amount) }} </template>
      </el-table-column>

      <el-table-column
        :label="t('quotations.itemEditor.colAction')"
        width="80"
        fixed="right"
        align="center"
      >
        <template #default="{ $index }">
          <el-button link type="danger" @click="handleRemove($index)">{{
            t('quotations.itemEditor.delete')
          }}</el-button>
        </template>
      </el-table-column>
    </el-table>
  </div>
</template>

<script setup lang="ts">
// 报价单明细编辑器脚本
// - v-model 双向绑定
// - 产品：服务端搜索选择 + 主档缓存回查（GET /products 分页上限 100，本地全量过滤不可靠）
// - 单位与规格写入行对象随保存落库；编码/成分/幅宽/克重只读展示自产品主档
// - 含税单价 = 单价 乘 (1 + 单据税率% 除 100)，税率由父组件（单据）传入
import { onMounted, ref, watch } from 'vue';
import { logAuxLoadFailure } from '@/utils/logger';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import { getProductColorList, getProductById, getProductList } from '@/api/product';
import type { Product } from '@/api/product';
import type { QuotationItemEditForm } from '@/api/quotation';
import { formatDecimalAmount } from '@/utils/money';
// 数值手写录入单源件 明细行数量与单价保持 number 模型 边界经 utils/number-input 的绑定器归一
import NumericInput from '@/components/NumericInput.vue';

// 明细行编辑态：金额/数量字段是 number 与 数值录入控件 双向绑定；
// 提交时由 quotationToWire 在 create.vue 边界转 DecimalWire 字符串上送后端
interface QuotationItemRow extends QuotationItemEditForm {
  product_id?: number;
  _colors?: Array<{ id: number; color_code?: string; color_name?: string }>;
  amount?: number;
}

const { t } = useI18n({ useScope: 'global' });

const props = defineProps<{
  modelValue: QuotationItemRow[];
  currency: string;
  /** 单据税率（百分数口径，13 表示 13%），未填写时不换算含税单价，组件不持有档位 */
  taxRate?: number;
}>();
const emit = defineEmits<{
  (e: 'update:modelValue', value: QuotationItemRow[]): void;
}>();

/** 产品下拉当前展示项：远程搜索命中的首页结果（每次搜索整体替换） */
const products = ref<Product[]>([]);
/**
 * 产品主档缓存（按 id）：只读带出列（编码/成分/幅宽/克重）与单位、规格带出的取值源。
 * 搜索结果与按 id 回查详情都会并入此处，保证选中项即使不在当前搜索页也能取到主档真值。
 */
const productMasters = ref<Record<number, Product>>({});
const productSearchLoading = ref(false);
/** 同一产品 id 的并发回查去重（多行同时引用未缓存产品时只发一次 GET /products/{id}） */
const inflightProductFetches = new Map<number, Promise<Product | undefined>>();
/**
 * 行级选择序号：远程搜索下两次快速切换产品的回查/色号请求可能乱序返回，
 * 只允许每行最新一次选择的结果写回，旧请求结果丢弃，防止上一个产品的值晚到覆盖。
 */
const rowChangeSeq = new WeakMap<QuotationItemRow, number>();

/** 创建一行空明细（unit/specification 由所选产品带出，创建时为空，不用兜底默认值掩盖） */
function createBlankItem(): QuotationItemRow {
  return {
    product_id: undefined,
    color_id: undefined,
    unit: '',
    quantity: 0,
    unit_price: 0,
    unit_price_with_tax: 0,
    specification: '',
    _colors: [],
  };
}

/** 添加明细 */
function handleAdd() {
  emit('update:modelValue', [...props.modelValue, createBlankItem()]);
}

/** 删除明细 */
function handleRemove(idx: number) {
  const arr = [...props.modelValue];
  arr.splice(idx, 1);
  emit('update:modelValue', arr);
}

function registerMaster(product?: Product) {
  if (product) productMasters.value[product.id] = product;
}

/**
 * 服务端搜索产品（与仓内既有可搜索选择器同口径：filterable remote + 关键词搜索取首页）。
 * 入参 keyword 走后端 ProductListQuery.search（名称/编码模糊匹配），单次取回 page_size 50；
 * 后端列表 page_size 上限为 100，一次性全量取数不可行，故不追求全量下拉，靠搜索命中。
 */
async function searchProducts(keyword: string) {
  productSearchLoading.value = true;
  try {
    const res = await getProductList({
      page: 1,
      page_size: 50,
      status: 'active',
      ...(keyword ? { search: keyword } : {}),
    });
    const items = res.data?.items || [];
    products.value = items;
    items.forEach(registerMaster);
  } catch (error) {
    logAuxLoadFailure(t('quotations.itemEditor.productLoadFailed'), error);
    products.value = [];
  } finally {
    productSearchLoading.value = false;
  }
}

/** 按 id 回查产品主档并缓存；缓存与在途请求复用，失败记录后返回 undefined（调用方据实置空，不兜底） */
function ensureProduct(productId: number): Promise<Product | undefined> {
  const cached = productMasters.value[productId];
  if (cached) return Promise.resolve(cached);
  const inflight = inflightProductFetches.get(productId);
  if (inflight) return inflight;
  const task = getProductById(productId)
    .then(res => {
      registerMaster(res.data);
      inflightProductFetches.delete(productId);
      return res.data;
    })
    .catch(error => {
      logAuxLoadFailure(t('quotations.itemEditor.productLoadFailed'), error);
      inflightProductFetches.delete(productId);
      return undefined;
    });
  inflightProductFetches.set(productId, task);
  return task;
}

/** 行上已选产品的主档（供只读带出列取值） */
function masterOf(row: QuotationItemRow): Product | undefined {
  return row.product_id ? productMasters.value[row.product_id] : undefined;
}

/**
 * 下拉展示项：搜索结果之外，把该行已选产品主档并入，保证编辑/复制回显与搜索换页后
 * 选中项名称仍可显示（否则 el-select 对不在选项内的值只回显 id）。
 */
function optionsFor(row: QuotationItemRow): Product[] {
  const selected = masterOf(row);
  if (!selected || products.value.some(p => p.id === selected.id)) return products.value;
  return [selected, ...products.value];
}

/**
 * 只读带出列单元格格式化：值原样取后端主档真值（Decimal 字段为字符串）并拼接单位符号；
 * 主档未配置该字段时显示占位符，绝不以 0 或默认值填充。
 */
function masterText(value?: string | number | null, unit = ''): string {
  if (value === undefined || value === null || value === '') return '—';
  return unit ? `${value} ${unit}` : String(value);
}

/** 产品变化时带出主档资料：先重置行上全部带出值，再按新产品回填；清空产品则全部置空 */
async function handleProductChange(row: QuotationItemRow, productId: number | undefined) {
  const seq = (rowChangeSeq.get(row) ?? 0) + 1;
  rowChangeSeq.set(row, seq);
  // 重置色号与本地色号列表，同时清掉单位与规格快照，杜绝残留上一个产品的数据
  row.color_id = undefined;
  row._colors = [];
  row.unit = '';
  row.specification = '';
  if (!productId) {
    emit('update:modelValue', [...props.modelValue]);
    return;
  }
  const product = await ensureProduct(productId);
  if (rowChangeSeq.get(row) !== seq) return;
  // 单位跟随产品交易单位：后端要求报价行 unit 逐字符等于 product.unit，不一致会 400。
  // 产品无 unit（或主档回查失败）时据实置空并提示去产品主数据配置，绝不用默认值掩盖。
  if (product?.unit) {
    row.unit = product.unit;
  } else {
    ElMessage.warning(t('quotations.itemEditor.productUnitMissing'));
  }
  // 规格写入行对象 specification 字段（sales_quotation_items.specification 实列，随保存落库）：
  // 选中即带出主档规格，带出后仍可按单编辑；清空/切换产品时按新产品重置
  row.specification = product?.specification || '';
  try {
    const res = await getProductColorList(productId);
    if (rowChangeSeq.get(row) !== seq) return;
    // 直接修改 row（因为是父组件 v-model 持有的对象引用）
    row._colors = res.data || [];
  } catch (error) {
    if (rowChangeSeq.get(row) !== seq) return;
    logAuxLoadFailure(t('quotations.itemEditor.colorLoadFailed'), error);
    row._colors = [];
  }
  // 强制响应式：发出新数组
  emit('update:modelValue', [...props.modelValue]);
}

/** 单价变化时按单据税率（百分数）重算含税单价，税率未填写则不造含税价 */
function recalcTax(row: QuotationItemRow, unitPrice: number | undefined) {
  if (unitPrice === undefined) return;
  row.unit_price = unitPrice;
  row.unit_price_with_tax =
    props.taxRate === undefined ? 0 : +(unitPrice * (1 + props.taxRate / 100)).toFixed(2);
  // 触发金额更新
  emit('update:modelValue', [...props.modelValue]);
}

/** 数量或单价变化时同步金额字段（金额仅用于显示） */
watch(
  () => props.modelValue.map(i => [i.quantity, i.unit_price]),
  () => {
    props.modelValue.forEach((i: QuotationItemRow) => {
      i.amount = (i.quantity || 0) * (i.unit_price || 0);
    });
  },
  { deep: true }
);

/** 单据税率变化后重算所有已填单价行的含税单价（先填价后填税率是常见顺序） */
watch(
  () => props.taxRate,
  () => {
    props.modelValue.forEach((i: QuotationItemRow) => {
      if (i.unit_price) {
        recalcTax(i, i.unit_price);
      }
    });
  }
);

/**
 * 编辑/复制回显的行已带 product_id：按 id 回查主档填充缓存，
 * 让只读带出列与下拉选中名称显示主档真值（只填充显示缓存，不重写行上已落库的 specification 快照）。
 */
watch(
  () => props.modelValue.map(i => i.product_id),
  ids => {
    ids.forEach(id => {
      if (id) void ensureProduct(id);
    });
  },
  { immediate: true }
);

/** 金额格式化：编辑态本地行内 amount 为 number（由 quantity×unit_price 计算），显示统一走 utils/money */
function formatAmount(value?: number): string {
  return formatDecimalAmount(value);
}

onMounted(() => {
  // 初始拉取首页可选项；下拉完整命中依赖 searchProducts 的服务端搜索
  void searchProducts('');
});
</script>

<style scoped>
.item-editor {
  width: 100%;
}
</style>
