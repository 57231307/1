<!--
  AdjustmentFormDialogTab.vue - 库存调整编辑对话框
  功能：新建或修改库存调整单表头（仓库/日期/方向/原因），建单期逐行选定目标库存行并录入
  调整量与单位成本；查看态（pending 单）支持后端明细回源与行级增删改。
  调用方：views/inventory-adjustment/index.vue（列表页挂载本对话框）。
  入参：modelValue 显隐、currentRow 编辑或查看的行、warehouses/products 主数据、mode 三态。
  传给谁：建单走 createInventoryAdjustment（POST /inventory/adjustments，明细键 stock_id/
  quantity/unit_cost/notes 与后端 AdjustmentItemPayload 逐字段对齐），行级维护走
  createAdjustmentItem/updateAdjustmentItem/deleteAdjustmentItem。
  存什么·存哪里：调整量与单位成本是 Decimal 契约字段，前端按十进制串采集提交，不折成 number
  避免浮点失真；留空即省略键（未采集），不塌成 0 伪装实测量。

  数值录入一律 el-input 加范围校验器（全项目禁步进数字框）；调整目标从手输主键改为
  按仓库下推真实库存行的可搜索名称选择（明细 stock_id 必须是 inventory_stocks 行主键，
  历史版本把产品下拉的 product_id 塞进 stock_id 键，落库即指向错误行或 404）。
-->
<template>
  <el-dialog
    :model-value="modelValue"
    :title="
      formData.id
        ? t('inventoryAdjustment.formDialogTab.titleEdit')
        : t('inventoryAdjustment.formDialogTab.titleCreate')
    "
    width="800px"
    :aria-label="
      mode === 'view'
        ? t('inventoryAdjustment.formDialogTab.ariaLabelView')
        : formData.id
          ? t('inventoryAdjustment.formDialogTab.ariaLabelEdit')
          : t('inventoryAdjustment.formDialogTab.ariaLabelCreate')
    "
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-form
      ref="formRef"
      :model="formData"
      label-width="100px"
      :disabled="mode === 'view'"
      :aria-label="t('inventoryAdjustment.formDialogTab.ariaLabelForm')"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item
            :label="t('inventoryAdjustment.formDialogTab.labelAdjustNo')"
            prop="adjustment_no"
          >
            <!-- 单号由后端 generate_adjustment_no 生成，前端只做展示（建单入参无该字段） -->
            <el-input v-model="formData.adjustment_no" disabled />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item
            :label="t('inventoryAdjustment.formDialogTab.labelAdjustDate')"
            prop="adjustment_date"
          >
            <el-date-picker
              v-model="formData.adjustment_date"
              type="date"
              value-format="YYYY-MM-DD"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item
        :label="t('inventoryAdjustment.formDialogTab.labelWarehouse')"
        prop="warehouse_id"
      >
        <el-select
          v-model="formData.warehouse_id"
          style="width: 100%"
          @change="onWarehouseChange"
        >
          <el-option
            v-for="wh in warehouses"
            :key="wh.id"
            :label="wh.warehouse_name"
            :value="wh.id"
          />
        </el-select>
      </el-form-item>
      <!-- 后端 CreateAdjustmentRequestPayload 必填 adjustment_type / reason_type，
           取值域与后端 services/inventory_adjustment_service.rs 的取值门同源：
           方向只有 increase/decrease 两值，越界值后端整单拒绝 -->
      <el-form-item
        :label="t('inventoryAdjustment.formDialogTab.labelAdjustmentType')"
        prop="adjustment_type"
      >
        <el-radio-group v-model="formData.adjustment_type">
          <el-radio value="increase">{{
            t('inventoryAdjustment.formDialogTab.adjustmentTypeIncrease')
          }}</el-radio>
          <el-radio value="decrease">{{
            t('inventoryAdjustment.formDialogTab.adjustmentTypeDecrease')
          }}</el-radio>
        </el-radio-group>
      </el-form-item>
      <el-form-item
        :label="t('inventoryAdjustment.formDialogTab.labelReasonType')"
        prop="reason_type"
      >
        <el-select v-model="formData.reason_type" style="width: 100%">
          <el-option
            :label="t('inventoryAdjustment.formDialogTab.reasonTypeDamage')"
            value="damage"
          />
          <el-option
            :label="t('inventoryAdjustment.formDialogTab.reasonTypeSample')"
            value="sample"
          />
          <el-option
            :label="t('inventoryAdjustment.formDialogTab.reasonTypeCorrection')"
            value="correction"
          />
          <el-option
            :label="t('inventoryAdjustment.formDialogTab.reasonTypeOther')"
            value="other"
          />
        </el-select>
      </el-form-item>
      <el-form-item
        :label="t('inventoryAdjustment.formDialogTab.labelReason')"
        prop="reason_description"
      >
        <el-input
          v-model="formData.reason_description"
          type="textarea"
          :rows="2"
          :placeholder="t('inventoryAdjustment.formDialogTab.placeholderReason')"
        />
      </el-form-item>
      <el-divider content-position="left">{{
        t('inventoryAdjustment.formDialogTab.dividerItems')
      }}</el-divider>
      <div
        v-for="(item, index) in formData.items"
        :key="index"
        style="display: flex; gap: 8px; margin-bottom: 8px; align-items: flex-start"
      >
        <!-- 调整目标：按所选仓库下推真实库存行（GET /inventory/stock），名称可搜索选择，
             手输行主键的旧入口废弃；仓库未选时禁用，选定仓库前无任何候选可出 -->
        <el-form-item
          :prop="`items.${index}.stock_id`"
          :rules="stockRowRules"
          label-width="0"
          class="inline-form-item"
          style="flex: 3"
        >
          <el-select
            v-model="item.stock_id"
            filterable
            :disabled="!formData.warehouse_id"
            :placeholder="
              formData.warehouse_id
                ? t('inventoryAdjustment.formDialogTab.placeholderStock')
                : t('inventoryAdjustment.formDialogTab.warehouseRequired')
            "
            style="width: 100%"
          >
            <el-option
              v-for="s in stockOptions"
              :key="s.id"
              :label="stockRowLabel(s)"
              :value="s.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item
          :prop="`items.${index}.quantity`"
          :rules="quantityRules"
          label-width="0"
          class="inline-form-item"
          style="flex: 1"
        >
          <el-input
            v-model="item.quantity"
            :placeholder="t('inventoryAdjustment.formDialogTab.placeholderQuantity')"
          />
        </el-form-item>
        <el-form-item
          :prop="`items.${index}.unit_cost`"
          :rules="unitCostRules"
          label-width="0"
          class="inline-form-item"
          style="flex: 1"
        >
          <el-input
            v-model="item.unit_cost"
            :placeholder="t('inventoryAdjustment.formDialogTab.placeholderPrice')"
          />
        </el-form-item>
        <el-input
          v-model="item.remark"
          :placeholder="t('inventoryAdjustment.formDialogTab.placeholderRemark')"
          style="flex: 1.5"
        />
        <el-button
          type="danger"
          :icon="Delete"
          circle
          :disabled="formData.items.length <= 1"
          @click="removeItem(index)"
        />
      </div>
      <el-button type="primary" link @click="addItem">
        <el-icon><Plus /></el-icon>{{ t('inventoryAdjustment.formDialogTab.buttonAddItem') }}
      </el-button>
    </el-form>

    <!-- 详情回源：查看模式下展示后端最新明细，pending 态支持行编辑/删除/新增。
         本节必须位于 el-form 之外：el-form 的 :disabled 沿表单上下文级联禁用全部子控件，
         原实现在表单内渲染行级编辑器，编辑框与保存按钮恒为禁用态（死 UI），移出后生效。 -->
    <template v-if="mode === 'view' && formData.id">
      <el-divider content-position="left">{{
        t('inventoryAdjustment.formDialogTab.dividerServerItems')
      }}</el-divider>
      <el-table v-loading="detailLoading" :data="serverItems" border size="small" max-height="300">
        <el-table-column prop="id" label="ID" width="60" />
        <el-table-column
          prop="stock_id"
          :label="t('inventoryAdjustment.formDialogTab.colStockId')"
          width="90"
        />
        <el-table-column
          prop="quantity_before"
          :label="t('inventoryAdjustment.formDialogTab.colQuantityBefore')"
          width="110"
        />
        <el-table-column :label="t('inventoryAdjustment.formDialogTab.colQuantity')" width="150">
          <template #default="{ row }">
            <el-input v-if="isPending" v-model="row.quantity" size="small" />
            <span v-else>{{ row.quantity }}</span>
          </template>
        </el-table-column>
        <el-table-column
          prop="quantity_after"
          :label="t('inventoryAdjustment.formDialogTab.colQuantityAfter')"
          width="110"
        />
        <el-table-column
          :label="t('inventoryAdjustment.formDialogTab.colOperation')"
          width="140"
          fixed="right"
        >
          <template #default="{ row }">
            <template v-if="isPending">
              <el-button link type="primary" size="small" @click="handleSaveItem(row)">{{
                t('inventoryAdjustment.formDialogTab.buttonSaveItem')
              }}</el-button>
              <el-button link type="danger" size="small" @click="handleDeleteItem(row)">{{
                t('inventoryAdjustment.formDialogTab.buttonDeleteItem')
              }}</el-button>
            </template>
            <span v-else>-</span>
          </template>
        </el-table-column>
      </el-table>
      <div v-if="isPending" class="add-item-bar">
        <el-select
          v-model="newItem.stock_id"
          filterable
          :placeholder="t('inventoryAdjustment.formDialogTab.placeholderStock')"
          style="width: 260px"
          @visible-change="(v: boolean) => v && void ensureStockOptions()"
        >
          <el-option
            v-for="s in stockOptions"
            :key="s.id"
            :label="stockRowLabel(s)"
            :value="s.id"
          />
        </el-select>
        <el-input
          v-model="newItem.quantity"
          :placeholder="t('inventoryAdjustment.formDialogTab.placeholderQuantity')"
          style="width: 130px"
        />
        <el-button type="primary" plain :loading="itemSaving" @click="handleAddItem">{{
          t('inventoryAdjustment.formDialogTab.buttonAddItem')
        }}</el-button>
      </div>
    </template>
    <template v-if="mode !== 'view'" #footer>
      <el-button @click="emit('update:modelValue', false)">{{
        t('inventoryAdjustment.formDialogTab.buttonCancel')
      }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
        t('inventoryAdjustment.formDialogTab.buttonSave')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox, type FormItemRule, type FormInstance } from 'element-plus';
import { isDialogDismissal, rethrowNonDismissal } from '@/utils/monitor';
import { Plus, Delete } from '@element-plus/icons-vue';
import {
  createInventoryAdjustment,
  updateInventoryAdjustment,
  generateInventoryAdjustmentNo,
  getInventoryAdjustment,
  createAdjustmentItem,
  updateAdjustmentItem,
  deleteAdjustmentItem,
  type InventoryAdjustmentEntity,
} from '@/api/inventory-adjustment';
import { getStockList, type InventoryStock } from '@/api/inventory';
import type { Warehouse } from '@/api/warehouse';
import type { Product } from '@/api/product';
import { logger } from '@/utils/logger';
import {
  INVENTORY_ADJUSTMENT_STATUS,
  type InventoryAdjustmentStatus,
} from '@/utils/inventory-adjustment-status';

const { t } = useI18n({ useScope: 'global' });

interface Props {
  modelValue: boolean;
  currentRow: InventoryAdjustmentEntity | null;
  warehouses: Warehouse[];
  products: Product[];
  mode: 'create' | 'edit' | 'view';
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void;
  (e: 'submitted'): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();

const formRef = ref<FormInstance>();
const submitLoading = ref(false);

// 建单明细行：stock_id 是 inventory_stocks 行主键（名称选择得来，不再手输、也不再误用产品 id）；
// quantity 与 unit_cost 是十进制串（后端 Decimal 契约，留空=未采集，省略键而非塌 0）
interface AdjustmentItemForm {
  stock_id: number | undefined;
  quantity: string;
  unit_cost: string;
  remark: string;
}

const newItemForm = (): AdjustmentItemForm => ({
  stock_id: undefined,
  quantity: '',
  unit_cost: '',
  remark: '',
});

const formData = reactive({
  id: 0,
  /** 后端生成、入参不接收，仅展示 */
  adjustment_no: '',
  /** 日期控件按 YYYY-MM-DD 取值，提交前转 RFC3339（后端 adjustment_date 是 DateTime<Utc>） */
  adjustment_date: new Date().toISOString().split('T')[0],
  warehouse_id: undefined as number | undefined,
  adjustment_type: 'increase',
  reason_type: 'damage',
  reason_description: '',
  status: INVENTORY_ADJUSTMENT_STATUS.PENDING as InventoryAdjustmentStatus,
  items: [newItemForm()] as AdjustmentItemForm[],
});

// ===== 库存行候选（GET /inventory/stock 按仓库下推，不手写死数据、不手输主键） =====
const stockOptions = ref<InventoryStock[]>([]);
const stockOptionsLoadedFor = ref<number | null>(null);

const stockRowLabel = (s: InventoryStock) =>
  `${s.product_code ?? s.product_id} ${s.product_name ?? ''} · ${t('inventoryAdjustment.formDialogTab.colBatchNo')}: ${s.batch_no || '-'} · ${t('inventoryAdjustment.formDialogTab.colColorNo')}: ${s.color_no || '-'} · ${s.stock_status}/${s.quality_status} · ${t('inventoryAdjustment.formDialogTab.availableQty')}: ${s.quantity_available}`;

const loadStockOptions = async (warehouseId?: number) => {
  if (!warehouseId) {
    stockOptions.value = [];
    stockOptionsLoadedFor.value = null;
    return;
  }
  if (stockOptionsLoadedFor.value === warehouseId) return;
  try {
    const res = await getStockList({ warehouse_id: warehouseId, page: 1, page_size: 200 });
    stockOptions.value = (res.data as { items?: InventoryStock[] } | null)?.items ?? [];
    stockOptionsLoadedFor.value = warehouseId;
  } catch (error) {
    logger.error(t('inventoryAdjustment.formDialogTab.stockLoadFailed'), error);
    ElMessage.error(t('inventoryAdjustment.formDialogTab.stockLoadFailed'));
    stockOptions.value = [];
  }
};

const ensureStockOptions = async () => {
  await loadStockOptions(formData.warehouse_id);
};

// 仓库变更：旧仓候选失效，清空候选与各行已选库存行（跨仓的 stock_id 不属于本单）
const onWarehouseChange = async () => {
  stockOptionsLoadedFor.value = null;
  formData.items.forEach(item => {
    item.stock_id = undefined;
  });
  await loadStockOptions(formData.warehouse_id);
};

// 数值校验器：调整量必填且为有限正数（与后端受理门"零与负数不构成调整"同口径）；
// 单位成本留空放行（未采集省略键），填了必须是不小于 0 的有限数
const quantityRules: FormItemRule[] = [
  {
    validator: (_rule: unknown, value: string, callback: (err?: Error) => void) => {
      const raw = (value ?? '').trim();
      if (raw === '') {
        callback(new Error(t('inventoryAdjustment.formDialogTab.qtyPositive')));
        return;
      }
      const n = Number(raw);
      callback(
        Number.isFinite(n) && n > 0
          ? undefined
          : new Error(t('inventoryAdjustment.formDialogTab.qtyPositive'))
      );
    },
    trigger: 'blur',
  },
];

const unitCostRules: FormItemRule[] = [
  {
    validator: (_rule: unknown, value: string, callback: (err?: Error) => void) => {
      const raw = (value ?? '').trim();
      if (raw === '') {
        callback();
        return;
      }
      const n = Number(raw);
      callback(
        Number.isFinite(n) && n >= 0
          ? undefined
          : new Error(t('inventoryAdjustment.formDialogTab.priceNonNegative'))
      );
    },
    trigger: 'blur',
  },
];

const stockRowRules: FormItemRule[] = [
  {
    validator: (_rule: unknown, value: number | undefined, callback: (err?: Error) => void) => {
      callback(value ? undefined : new Error(t('inventoryAdjustment.formDialogTab.stockRequired')));
    },
    trigger: 'change',
  },
];

const resetForm = () => {
  formData.id = 0;
  formData.adjustment_no = '';
  formData.adjustment_date = new Date().toISOString().split('T')[0];
  formData.warehouse_id = undefined;
  formData.adjustment_type = 'increase';
  formData.reason_type = 'damage';
  formData.reason_description = '';
  formData.status = INVENTORY_ADJUSTMENT_STATUS.PENDING;
  formData.items = [newItemForm()];
  stockOptions.value = [];
  stockOptionsLoadedFor.value = null;
};

const addItem = () => {
  formData.items.push(newItemForm());
};
const removeItem = (index: number) => {
  if (formData.items.length > 1) formData.items.splice(index, 1);
};

const generateNo = async () => {
  try {
    const res = await generateInventoryAdjustmentNo();
    formData.adjustment_no = res.data?.adjustment_no || '';
  } catch (error) {
    logger.error(
      t('inventoryAdjustment.formDialogTab.messageGenerateNoFailed'),
      (error as Error).message
    );
  }
};

watch(
  () => props.modelValue,
  async val => {
    if (val) {
      if (props.currentRow) {
        Object.assign(formData, props.currentRow);
        // 服务端明细归一为建单行形状（Decimal 串原样保留；备注列服务端键名为 notes）
        const serverItems = (
          props.currentRow as {
            items?: { stock_id: number; quantity?: string; unit_cost?: string | null; notes?: string | null }[];
          }
        ).items;
        formData.items =
          Array.isArray(serverItems) && serverItems.length > 0
            ? serverItems.map(i => ({
                stock_id: i.stock_id,
                quantity: String(i.quantity ?? ''),
                unit_cost: String(i.unit_cost ?? ''),
                remark: String(i.notes ?? ''),
              }))
            : [newItemForm()];
        await loadStockOptions(formData.warehouse_id);
        if (props.mode === 'view' && formData.id) await fetchServerItems();
      } else {
        resetForm();
        await generateNo();
      }
    }
  }
);

onMounted(() => {
  if (props.modelValue && !props.currentRow) {
    generateNo();
  }
});

// ===== 查看模式：后端明细回源与行级维护（getInventoryAdjustment + 明细 CRUD） =====
// 明细出参的数值列都是 Decimal 串（api/inventory-adjustment.ts AdjustmentItem 同口径），
// 按串承载、编辑后原样送串，不在前端做 number 往返
interface ServerItem {
  id: number;
  stock_id: number;
  quantity: string;
  quantity_before: string;
  quantity_after: string;
  unit_cost?: string | null;
  amount?: string | null;
  notes?: string | null;
}

const serverItems = ref<ServerItem[]>([]);
const detailLoading = ref(false);
const itemSaving = ref(false);
const newItem = reactive({ stock_id: undefined as number | undefined, quantity: '' });

const isPending = computed(() => formData.status === INVENTORY_ADJUSTMENT_STATUS.PENDING);

const fetchServerItems = async () => {
  detailLoading.value = true;
  try {
    const res = (await getInventoryAdjustment(formData.id)) as {
      data?: { items?: ServerItem[] };
    };
    serverItems.value = res.data?.items ?? [];
  } catch (e) {
    ElMessage.error((e as Error).message || t('inventoryAdjustment.listTab.messageFailure'));
  } finally {
    detailLoading.value = false;
  }
};

/** 行内数量文本的受理判定：有限正数（后端建明细/改明细同口径拒绝零、负数与非法串） */
const parsePositiveQuantity = (raw: string): number | null => {
  const n = Number((raw ?? '').trim());
  return Number.isFinite(n) && n > 0 ? n : null;
};

const handleAddItem = async () => {
  if (!formData.id || !newItem.stock_id) {
    ElMessage.warning(t('inventoryAdjustment.formDialogTab.stockRequired'));
    return;
  }
  if (parsePositiveQuantity(newItem.quantity) === null) {
    ElMessage.warning(t('inventoryAdjustment.formDialogTab.qtyPositive'));
    return;
  }
  itemSaving.value = true;
  try {
    await createAdjustmentItem(formData.id, {
      stock_id: newItem.stock_id,
      quantity: newItem.quantity.trim(),
    });
    ElMessage.success(t('inventoryAdjustment.listTab.messageSuccess'));
    newItem.stock_id = undefined;
    newItem.quantity = '';
    await fetchServerItems();
  } catch (e) {
    ElMessage.error((e as Error).message || t('inventoryAdjustment.listTab.messageFailure'));
  } finally {
    itemSaving.value = false;
  }
};

const handleSaveItem = async (row: ServerItem) => {
  if (parsePositiveQuantity(row.quantity) === null) {
    ElMessage.warning(t('inventoryAdjustment.formDialogTab.qtyPositive'));
    return;
  }
  try {
    await updateAdjustmentItem(row.id, {
      stock_id: row.stock_id,
      quantity: row.quantity.trim(),
    });
    ElMessage.success(t('inventoryAdjustment.listTab.messageSuccess'));
    await fetchServerItems();
  } catch (e) {
    ElMessage.error((e as Error).message || t('inventoryAdjustment.listTab.messageFailure'));
  }
};

const handleDeleteItem = async (row: ServerItem) => {
  try {
    await ElMessageBox.confirm(
      t('inventoryAdjustment.formDialogTab.messageDeleteItemConfirm'),
      t('inventoryAdjustment.listTab.titleDeleteConfirm'),
      { type: 'warning' }
    );
  } catch (error: unknown) {
    if (isDialogDismissal(error)) return;
    rethrowNonDismissal('inventoryAdjustment.handleDeleteItem', error);
  }
  try {
    await deleteAdjustmentItem(row.id);
    ElMessage.success(t('inventoryAdjustment.listTab.messageSuccess'));
    await fetchServerItems();
  } catch (e) {
    if (!isDialogDismissal(e)) {
      ElMessage.error((e as Error).message || t('inventoryAdjustment.listTab.messageFailure'));
    }
  }
};

const handleSubmit = async () => {
  if (!formData.warehouse_id) {
    ElMessage.warning(t('inventoryAdjustment.formDialogTab.warehouseRequired'));
    return;
  }
  // 列表行的 adjustment_date 由后端补齐前为空（见 api 层注释），
  // 空值直接提交会被后端解析成 1970 或报日期格式错，故先行拦截。
  if (!formData.adjustment_date) {
    ElMessage.warning(t('inventoryAdjustment.formDialogTab.adjustmentDateRequired'));
    return;
  }
  const valid = await formRef.value?.validate().catch(() => false);
  if (!valid) return;
  submitLoading.value = true;
  try {
    // 后端入参键名逐一对应 CreateAdjustmentRequestPayload / UpdateAdjustmentRequestPayload
    // （handlers/inventory_adjustment_handler.rs 的建单与改单 DTO）：adjustment_date 需
    // RFC3339，明细数量为十进制串；单号与状态由后端生成/流转，前端不提交。
    // 单位成本留空=未采集，省略 unit_cost 键（后端 Option<Decimal> 缺席不写列），
    // 塌成 "0" 会把未采集伪装成实际成本并参与金额重算。
    const fields = {
      warehouse_id: formData.warehouse_id,
      adjustment_date: new Date(formData.adjustment_date).toISOString(),
      adjustment_type: formData.adjustment_type,
      reason_type: formData.reason_type,
      reason_description: formData.reason_description,
    };
    if (formData.id) {
      // 三态语义（后端 UpdateAdjustmentRequestPayload DoubleOption，RFC 7386）：
      // 对话框已回显原值，NOT NULL 列（warehouse_id/adjustment_date/adjustment_type/
      // reason_type）恒送值（送 null 被后端 400 拒绝）；DB 可空列 reason_description
      // UI 清空时送显式 null（=清空为 NULL），塌成省略会"改了不生效"；
      // notes 不在本表单采集范围，不提交该键（缺席=保持原值）。
      await updateInventoryAdjustment(formData.id, {
        warehouse_id: formData.warehouse_id,
        adjustment_date: new Date(formData.adjustment_date).toISOString(),
        adjustment_type: formData.adjustment_type,
        reason_type: formData.reason_type,
        reason_description: formData.reason_description || null,
      });
    } else {
      await createInventoryAdjustment({
        ...fields,
        items: formData.items.map(item => ({
          stock_id: item.stock_id as number,
          quantity: item.quantity.trim(),
          ...(item.unit_cost.trim() ? { unit_cost: item.unit_cost.trim() } : {}),
          notes: item.remark,
        })),
      });
    }
    ElMessage.success(t('inventoryAdjustment.formDialogTab.messageSuccess'));
    emit('update:modelValue', false);
    emit('submitted');
  } catch (error) {
    ElMessage.error(
      (error as Error).message || t('inventoryAdjustment.formDialogTab.messageFailed')
    );
    logger.error(
      t('inventoryAdjustment.formDialogTab.messageSaveFailed'),
      (error as Error).message
    );
  } finally {
    submitLoading.value = false;
  }
};
</script>

<style scoped>
/* 明细行内 el-form-item 去默认下边距，保持与原行内控件一致的贴边布局 */
.inline-form-item {
  margin-bottom: 0;
}
</style>
