<!--
  ReturnEditDialog.vue - 销售退货新建/编辑对话框
  任务编号: P14 批 2 I-3 第 7 批
  拆分原 sales-returns/index.vue 的新建/编辑对话框部分
  内部维护 local formData，通过 props.initialData 同步初始值
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="
      dialogMode === 'create'
        ? t('salesReturns.editDialog.titleCreate')
        : t('salesReturns.editDialog.titleEdit')
    "
    width="900px"
    :aria-label="
      dialogMode === 'create'
        ? t('salesReturns.editDialog.titleCreate')
        : t('salesReturns.editDialog.titleEdit')
    "
    @update:model-value="onClose"
    @close="onDialogClose"
  >
    <el-form
      ref="formRef"
      :model="localFormData"
      :rules="mergedRules"
      label-width="120px"
      :aria-label="t('salesReturns.editDialog.formAriaLabel')"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('salesReturns.editDialog.labelSalesOrderNo')" prop="salesOrderId">
            <el-select
              v-model="localFormData.salesOrderId"
              :placeholder="t('salesReturns.editDialog.placeholderSalesOrder')"
              style="width: 100%"
              filterable
              @change="onSalesOrderChange"
            >
              <el-option
                v-for="order in salesOrderList"
                :key="order.id"
                :label="order.order_no"
                :value="order.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('salesReturns.editDialog.labelCustomer')" prop="customerId">
            <el-select
              v-model="localFormData.customerId"
              :placeholder="t('salesReturns.editDialog.placeholderCustomer')"
              style="width: 100%"
              filterable
            >
              <el-option
                v-for="customer in customerList"
                :key="customer.id"
                :label="customer.customer_name"
                :value="customer.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('salesReturns.editDialog.labelReturnDate')" prop="returnDate">
            <el-date-picker
              v-model="localFormData.returnDate"
              type="date"
              :placeholder="t('salesReturns.editDialog.placeholderReturnDate')"
              style="width: 100%"
              format="YYYY-MM-DD"
              value-format="YYYY-MM-DD"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('salesReturns.editDialog.labelWarehouse')" prop="warehouseId">
            <el-select
              v-model="localFormData.warehouseId"
              :placeholder="t('salesReturns.editDialog.placeholderWarehouse')"
              style="width: 100%"
              filterable
            >
              <el-option
                v-for="warehouse in warehouseList"
                :key="warehouse.id"
                :label="warehouse.warehouse_name"
                :value="warehouse.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('salesReturns.editDialog.labelReasonType')" prop="reasonType">
            <el-select
              v-model="localFormData.reasonType"
              :placeholder="t('salesReturns.editDialog.placeholderReasonType')"
              style="width: 100%"
            >
              <el-option
                v-for="opt in reasonOptions"
                :key="opt.value"
                :label="t(opt.labelKey)"
                :value="opt.value"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('salesReturns.editDialog.labelReasonDetail')" prop="reasonDetail">
            <el-input
              v-model="localFormData.reasonDetail"
              :placeholder="t('salesReturns.editDialog.placeholderReasonDetail')"
            />
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :span="24">
          <el-form-item :label="t('salesReturns.editDialog.labelRemarks')" prop="remarks">
            <el-input
              v-model="localFormData.remarks"
              type="textarea"
              :rows="3"
              :placeholder="t('salesReturns.editDialog.placeholderRemarks')"
            />
          </el-form-item>
        </el-col>
      </el-row>

      <el-divider />

      <el-form-item :label="t('salesReturns.editDialog.labelReturnDetails')">
        <el-button type="primary" size="small" style="margin-bottom: 10px" @click="onAddItem">
          {{ t('salesReturns.editDialog.buttonAddDetail') }}
        </el-button>
        <el-table
          :data="localFormData.items"
          border
          style="width: 100%"
          :aria-label="t('salesReturns.editDialog.detailsTableAriaLabel')"
        >
          <el-table-column :label="t('salesReturns.editDialog.columnProductName')" width="200">
            <template #default="{ row }">
              <el-select
                v-model="row.productId"
                :placeholder="t('salesReturns.editDialog.placeholderProduct')"
                style="width: 100%"
                filterable
              >
                <el-option
                  v-for="product in productList"
                  :key="product.id"
                  :label="product.product_name"
                  :value="product.id"
                />
              </el-select>
            </template>
          </el-table-column>
          <el-table-column :label="t('salesReturns.editDialog.columnQuantity')" width="120">
            <template #default="{ row }">
              <!-- 手写录入 行模型保持数值（提交与算术消费面不变）DOM 文本走草稿
                   范围与精度在提交前逐行判定 不在录入途中改写用户已输入的文本 -->
              <el-input
                :model-value="cellText(row, 'quantity')"
                inputmode="decimal"
                style="width: 100%"
                @update:model-value="(v: string) => cellWrite(row, 'quantity', v)"
                @blur="cellCommit(row, 'quantity')"
                @change="onCalculate"
              />
            </template>
          </el-table-column>
          <el-table-column :label="t('salesReturns.editDialog.columnUnitPriceWithTax')" width="140">
            <template #default="{ row }">
              <!--
                含税单价是本表价格的主输入（录入事实），字符串模型直接进行对象：
                带加减号的步进控件是全项目禁项；关联订单行可留空，由后端按订单行含税额除法还原。
              -->
              <el-input
                v-model="row.unitPriceWithTax"
                inputmode="decimal"
                :placeholder="t('salesReturns.editDialog.placeholderUnitPriceWithTax')"
                style="width: 100%"
                @change="onCalculate"
              />
            </template>
          </el-table-column>
          <el-table-column :label="t('salesReturns.editDialog.columnTaxPercent')" width="120">
            <template #default="{ row }">
              <!-- 税率三态 空文本写回 undefined 提交时省略该键 由后端取关联订单的权威税率 -->
              <el-input
                :model-value="cellText(row, 'taxPercent')"
                inputmode="decimal"
                :placeholder="t('salesReturns.editDialog.placeholderTaxPercent')"
                style="width: 100%"
                @update:model-value="(v: string) => cellWrite(row, 'taxPercent', v)"
                @blur="cellCommit(row, 'taxPercent')"
                @change="onCalculate"
              />
            </template>
          </el-table-column>
          <el-table-column :label="t('salesReturns.editDialog.columnDiscountPercent')" width="120">
            <template #default="{ row }">
              <!-- 折扣三态同税率 空文本即省略该键 -->
              <el-input
                :model-value="cellText(row, 'discountPercent')"
                inputmode="decimal"
                style="width: 100%"
                @update:model-value="(v: string) => cellWrite(row, 'discountPercent', v)"
                @blur="cellCommit(row, 'discountPercent')"
                @change="onCalculate"
              />
            </template>
          </el-table-column>
          <el-table-column :label="t('salesReturns.editDialog.columnNetPrice')" width="120">
            <template #default="{ row }">
              <!-- 不含税单价是派生只读值：按含税单价与本行税率除法反算，改它必然被后端单价钩稽门拒绝 -->
              <span>{{ displayNetPrice(row) }}</span>
            </template>
          </el-table-column>
          <el-table-column :label="t('salesReturns.editDialog.columnAmount')" width="120">
            <template #default="{ row }">
              <!-- 行金额是成交含税额：含税聚合金额在含税侧减行折扣，与后端落库 total_amount 同口径 -->
              <span>{{ displayAmount(row) }}</span>
            </template>
          </el-table-column>
          <el-table-column :label="t('salesReturns.editDialog.columnReason')" width="150">
            <template #default="{ row }">
              <el-input
                v-model="row.reason"
                :placeholder="t('salesReturns.editDialog.placeholderReasonShort')"
                size="small"
              />
            </template>
          </el-table-column>
          <el-table-column :label="t('salesReturns.editDialog.columnAction')" width="80">
            <template #default="{ $index }">
              <el-button type="danger" size="small" @click="onRemoveItem($index)">{{
                t('salesReturns.editDialog.buttonDelete')
              }}</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-form-item>

      <el-row :gutter="20">
        <el-col :span="12" :offset="12">
          <el-form-item :label="t('salesReturns.editDialog.labelTotalAmount')">
            <!-- 总额由 calculateTotal 维护，保持数值模型，只读展示经 numericView 按 2 位归一 -->
            <el-input
              :model-value="numericView(localFormData.totalAmount, 2)"
              disabled
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
    </el-form>

    <template #footer>
      <el-button @click="onClose(false)">{{ t('salesReturns.editDialog.buttonCancel') }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="onSubmit">{{
        t('salesReturns.editDialog.buttonConfirm')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { deepClone } from '@/utils';
import { ref, watch, reactive, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, type FormInstance, type FormRules } from 'element-plus';
import {
  numericView,
  parseNumberText,
  sanitizeNumberText,
  validateNumberInput,
  type NumberInputRuleOptions,
} from '@/utils/number-input';
import {
  dealInclusiveFromLine,
  formatDecimalAmount,
  netFromInclusive,
  parseDecimalInput,
} from '@/utils/money';
import { getWarehouseList, type Warehouse } from '@/api/warehouse';
import { logAuxLoadFailure } from '@/utils/logger';
// v11 批次 174 P2-1 修复：从 useSr 导入具体类型替代 any
import {
  type ReturnFormItem,
  type ReturnForm,
  type SalesOrderOption,
  type CustomerOption,
  type ProductOption,
} from '../composables/useSr';
import { RETURN_REASON_OPTIONS } from '@/constants/return-reason';

const { t } = useI18n({ useScope: 'global' });

const props = defineProps<{
  visible: boolean;
  dialogMode: 'create' | 'edit';
  formData: ReturnForm;
  salesOrderList: SalesOrderOption[];
  customerList: CustomerOption[];
  productList: ProductOption[];
  formRules: FormRules;
  submitLoading: boolean;
}>();

const emit = defineEmits<{
  (e: 'update:visible', val: boolean): void;
  (e: 'submit', data: ReturnForm): void;
  (e: 'salesOrderChange', orderId: number): void;
  (e: 'addItem'): void;
  (e: 'removeItem', index: number): void;
  (e: 'calculate'): void;
  (e: 'dialogClose'): void;
}>();

const formRef = ref<FormInstance>();

// 浅拷贝 props.formData 到 local（避免直接修改 prop）
// v11 批次 174 P2-1 修复：reactive<any>({}) 改为 reactive<Partial<ReturnForm>>({})
const localFormData = reactive<Partial<ReturnForm>>({});
watch(
  () => props.formData,
  newVal => {
    // v11 批次 174 P2-1 修复：使用 keyof ReturnForm 断言避免 string 索引错误
    Object.keys(localFormData).forEach(k => {
      delete (localFormData as Record<string, unknown>)[k];
    });
    Object.assign(localFormData, deepClone(newVal));
  },
  { immediate: true, deep: true }
);

// 仓库主数据（表头 warehouse_id 下拉），对话框自持，父级 index.vue 未注入该列表
const warehouseList = ref<Warehouse[]>([]);
const loadWarehouses = async () => {
  try {
    const res = await getWarehouseList({ page_size: 100 });
    warehouseList.value = res.data?.items || [];
  } catch (error: unknown) {
    logAuxLoadFailure('loadWarehouseListFailed', error);
  }
};
onMounted(loadWarehouses);

// 退货原因分类候选：value 为落库业务数据，label 走 i18n
const reasonOptions = RETURN_REASON_OPTIONS;

// 校验规则 = 父级规则 + 表头新增必填（仓库、原因类型）
const mergedRules = computed<FormRules>(() => ({
  ...props.formRules,
  warehouseId: [
    {
      required: true,
      message: t('salesReturns.editDialog.ruleWarehouseRequired'),
      trigger: 'change',
    },
  ],
  reasonType: [
    {
      required: true,
      message: t('salesReturns.editDialog.ruleReasonTypeRequired'),
      trigger: 'change',
    },
  ],
}));

const onClose = (val: boolean) => {
  emit('update:visible', val);
};

// ===== 明细行数值单元格的手写录入面 =====
// 行模型的含税单价（unitPriceWithTax）是字符串主输入 直接绑 el-input 原文进行对象
// quantity taxPercent discountPercent 三个数值列保持数值模型 DOM 文本走草稿
// 不含税单价与行成交含税额是派生只读值 显示腿只经 money.ts 单源算式换算 不可手输
// 输入途中不改写用户已敲的字符 "3." 这类半成形态不会被格式化打回
// 边界判定见 validateCells 失焦只弃草稿 显示回落到 numericView 的格式化形态
type NumericCell = 'quantity' | 'taxPercent' | 'discountPercent';

/** 三个数值草稿列的边界词表 label 走既有 i18n 键 失败消息点名是哪个字段 */
const cellSpecs = computed<Record<NumericCell, NumberInputRuleOptions>>(() => ({
  quantity: {
    label: t('salesReturns.editDialog.columnQuantity'),
    required: true,
    min: 1,
    precision: 2,
  },
  taxPercent: {
    label: t('salesReturns.editDialog.columnTaxPercent'),
    min: 0,
    max: 100,
    precision: 2,
  },
  discountPercent: {
    label: t('salesReturns.editDialog.columnDiscountPercent'),
    min: 0,
    max: 100,
    precision: 2,
  },
}));

const NUMERIC_CELLS: NumericCell[] = ['quantity', 'taxPercent', 'discountPercent'];

/** 草稿表 键为 行身份.列名 行对象重建（编辑回显 增删行）后旧键自然失配 */
const cellDrafts = reactive<Record<string, string>>({});
const cellRowIds = new WeakMap<object, number>();
let cellSeq = 0;

const cellKey = (row: ReturnFormItem, col: NumericCell): string => {
  let id = cellRowIds.get(row);
  if (id === undefined) {
    id = ++cellSeq;
    cellRowIds.set(row, id);
  }
  return `${id}.${col}`;
};

/** 绑 el-input :model-value 的录入文本 草稿优先 无草稿时按两位小数显示模型 */
const cellText = (row: ReturnFormItem, col: NumericCell): string => {
  const draft = cellDrafts[cellKey(row, col)];
  return draft === undefined ? numericView(row[col], 2) : draft;
};

/** 绑 el-input @update:model-value 文本按字符约束收进草稿 解析出的数值写回行模型 */
const cellWrite = (row: ReturnFormItem, col: NumericCell, raw: string): void => {
  const text = sanitizeNumberText(raw, cellSpecs.value[col]);
  cellDrafts[cellKey(row, col)] = text;
  const parsed = parseNumberText(text);
  if (col === 'taxPercent' || col === 'discountPercent') {
    // 可空三态 空文本写回 undefined 提交时省略该键 由后端取订单权威值
    row[col] = parsed;
  } else if (parsed !== undefined) {
    // 必填列在半成形态保持模型原值 空与越界由提交前的逐行判定点名拦截
    row[col] = parsed;
  }
};

/** 绑 el-input @blur 弃草稿 显示收敛回模型的格式化形态 */
const cellCommit = (row: ReturnFormItem, col: NumericCell): void => {
  delete cellDrafts[cellKey(row, col)];
};

/**
 * 提交前逐行校验数值单元格 判的是用户看到的录入文本（草稿优先）
 * 表格行内单元格挂不进表头的 el-form rules 故在此单点走同一份校验器
 * 首个非法值即外显并拦下提交 不做钳值 不做默认值兜底
 */
const validateCells = (): boolean => {
  for (const item of localFormData.items ?? []) {
    for (const col of NUMERIC_CELLS) {
      const message = validateNumberInput(cellText(item, col), cellSpecs.value[col]);
      if (message) {
        ElMessage.warning(message);
        return false;
      }
    }
  }
  return true;
};

/**
 * 派生只读列的显示腿（不含税单价）。
 * 入参，row 是本表一行；含税原文有效时按本行税率除法反算（money.ts 的 netFromInclusive，
 * 与后端 utils/tax.rs 同源），实时反映未提交的编辑；含税留空时回落行上既有派生值
 * （带出与回显放上的后端权威数）。出参是两位小数文本，无值显示占位符而不是 0。
 * 传给谁：不含税单价列模板。本函数只算展示，提交面仍以父级 calculateTotal 收敛后的
 * 行模型为准，两处调同一枚 money.ts 原语，不存在第二套公式。
 */
const displayNetPrice = (row: ReturnFormItem): string => {
  const inc = parseDecimalInput(row.unitPriceWithTax);
  const rate = row.taxPercent;
  if (
    inc !== undefined &&
    Number.isFinite(inc) &&
    inc >= 0 &&
    rate !== undefined &&
    Number.isFinite(rate) &&
    rate >= 0
  ) {
    return formatDecimalAmount(netFromInclusive(inc, rate));
  }
  return row.unitPrice ? formatDecimalAmount(row.unitPrice) : '--';
};

/**
 * 派生只读列的显示腿（行成交含税额）。
 * 入参，row 是本表一行；含税原文有效时按「含税聚合金额在含税侧减行折扣」现算
 * （money.ts 的 dealInclusiveFromLine，与后端 line_split 的 deal_inclusive 同口径），
 * 含税留空时回落行上 amount（带出与回显放上的后端 total_amount）。
 * 传给谁：行金额列模板。绝不写 数量 乘 不含税单价 的第二套合计。
 */
const displayAmount = (row: ReturnFormItem): string => {
  const inc = parseDecimalInput(row.unitPriceWithTax);
  const rate = row.taxPercent;
  if (
    inc !== undefined &&
    Number.isFinite(inc) &&
    inc >= 0 &&
    rate !== undefined &&
    Number.isFinite(rate) &&
    rate >= 0
  ) {
    return formatDecimalAmount(dealInclusiveFromLine(inc, row.quantity, row.discountPercent));
  }
  return row.amount ? formatDecimalAmount(row.amount) : '--';
};

const onSubmit = async () => {
  let valid = false;
  await formRef.value?.validate(ok => {
    valid = ok;
  });
  if (!valid) return;
  if (!validateCells()) return;
  // 编辑结果只经 emit 交回父组件，由父组件写入共享表单（单向数据流）
  emit('submit', deepClone(localFormData) as ReturnForm);
};

const onSalesOrderChange = (orderId: number) => {
  emit('salesOrderChange', orderId);
};

const onAddItem = () => {
  emit('addItem');
};

const onRemoveItem = (index: number) => {
  emit('removeItem', index);
};

const onCalculate = () => {
  emit('calculate');
};

const onDialogClose = () => {
  emit('dialogClose');
};

defineExpose({ formRef });
</script>
