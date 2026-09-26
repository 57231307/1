<!--
  CountDetailDialogTab.vue - 盘点单详情对话框
  来源：原 inventoryCount/index.vue 中 盘点单详情弹窗
  拆分日期：2026-06-15 B3-4
  扩展：详情回源 + 盘点明细实盘录入/行编辑/删除
-->
<template>
  <el-dialog
    :model-value="modelValue"
    :title="t('inventoryCount.detailDialogTab.titleDetail')"
    width="900px"
    :aria-label="t('inventoryCount.detailDialogTab.ariaLabelDetail')"
    @update:model-value="(val: boolean) => onVisibleChange(val)"
  >
    <el-descriptions v-if="currentRow" :column="2" border>
      <el-descriptions-item :label="t('inventoryCount.detailDialogTab.labelCountNo')">{{
        currentRow.count_no
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('inventoryCount.detailDialogTab.labelCountDate')">{{
        currentRow.count_date
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('inventoryCount.detailDialogTab.labelWarehouse')">{{
        currentRow.warehouse_name
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('inventoryCount.detailDialogTab.labelStatus')">
        <el-tag :type="inventoryCountStatusTagType(currentRow.status)" size="small">
          {{ t(inventoryCountStatusLabelKey(currentRow.status)) }}
        </el-tag>
      </el-descriptions-item>
      <el-descriptions-item :label="t('inventoryCount.detailDialogTab.labelCreatedBy')">{{
        currentRow.created_by_name
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('inventoryCount.detailDialogTab.labelCreatedAt')">{{
        currentRow.created_at
      }}</el-descriptions-item>
      <!-- 完成时间取详情接口（CountResponse.completed_at）：列表行 CountSummary 无该字段 -->
      <el-descriptions-item :label="t('inventoryCount.detailDialogTab.labelCompletedAt')" :span="2">
        {{ detail?.completed_at }}
      </el-descriptions-item>
    </el-descriptions>

    <div v-if="currentRow" class="items-section">
      <div class="items-toolbar">
        <span class="items-title">{{ t('inventoryCount.detailDialogTab.itemsTitle') }}</span>
        <el-button
          v-if="isPending"
          type="primary"
          size="small"
          :loading="recording"
          @click="handleRecordItems"
          >{{ t('inventoryCount.detailDialogTab.buttonRecord') }}</el-button
        >
      </div>
      <el-table v-loading="itemsLoading" :data="items" border size="small" max-height="320">
        <el-table-column prop="id" label="ID" width="60" />
        <el-table-column
          prop="stock_id"
          :label="t('inventoryCount.detailDialogTab.colStockId')"
          width="90"
        />
        <el-table-column
          prop="product_id"
          :label="t('inventoryCount.detailDialogTab.colProductId')"
          width="90"
        />
        <el-table-column
          prop="quantity_before"
          :label="t('inventoryCount.detailDialogTab.colQuantityBefore')"
          width="110"
        />
        <el-table-column :label="t('inventoryCount.detailDialogTab.colQuantityActual')" width="140">
          <template #default="{ row }">
            <el-input-number
              v-if="isPending"
              v-model="row.quantity_actual"
              :min="0"
              size="small"
              controls-position="right"
            />
            <span v-else>{{ row.quantity_actual }}</span>
          </template>
        </el-table-column>
        <el-table-column
          prop="quantity_difference"
          :label="t('inventoryCount.detailDialogTab.colQuantityDiff')"
          width="110"
        >
          <template #default="{ row }">
            <span :class="Number(row.quantity_difference) < 0 ? 'diff-negative' : 'diff-positive'">
              {{ row.quantity_difference }}
            </span>
          </template>
        </el-table-column>
        <el-table-column
          :label="t('inventoryCount.detailDialogTab.colOperation')"
          width="140"
          fixed="right"
        >
          <template #default="{ row }">
            <template v-if="isPending">
              <el-button link type="primary" size="small" @click="handleUpdateItem(row)">{{
                t('inventoryCount.detailDialogTab.buttonSaveItem')
              }}</el-button>
              <el-button link type="danger" size="small" @click="handleDeleteItem(row)">{{
                t('inventoryCount.detailDialogTab.buttonDeleteItem')
              }}</el-button>
            </template>
            <span v-else>-</span>
          </template>
        </el-table-column>
      </el-table>
    </div>

    <template #footer>
      <el-button @click="emit('update:modelValue', false)">{{
        t('inventoryCount.detailDialogTab.buttonClose')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  getInventoryCount,
  recordCountItems,
  updateCountItem,
  deleteCountItem,
  type InventoryCountDetail,
  type InventoryCountEntity,
} from '@/api/inventory-count';
import {
  INVENTORY_COUNT_STATUS,
  inventoryCountStatusLabelKey,
  inventoryCountStatusTagType,
} from '@/utils/inventory-count-status';

const { t } = useI18n({ useScope: 'global' });

interface Props {
  modelValue: boolean;
  currentRow: InventoryCountEntity | null;
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();

/** 明细行键名与后端 CountItemResponse 一致；数量列是 Decimal 的字符串序列化，入表时转数值以便行内编辑 */
interface CountDetailItem {
  id: number;
  stock_id: number;
  product_id: number;
  quantity_before: number;
  quantity_actual: number;
  quantity_difference: number;
  notes?: string | null;
}

const items = ref<CountDetailItem[]>([]);
const itemsLoading = ref(false);
const recording = ref(false);
const detail = ref<InventoryCountDetail | null>(null);

const isPending = computed(() => props.currentRow?.status === INVENTORY_COUNT_STATUS.PENDING);

// 对话框打开时回源最新详情（含明细）
const fetchDetail = async () => {
  if (!props.currentRow?.id) return;
  itemsLoading.value = true;
  try {
    const res = (await getInventoryCount(props.currentRow.id)) as {
      data?: InventoryCountDetail;
    };
    detail.value = res.data ?? null;
    items.value = (res.data?.items ?? []).map(i => ({
      id: i.id,
      stock_id: i.stock_id,
      product_id: i.product_id,
      quantity_before: Number(i.quantity_before),
      quantity_actual: Number(i.quantity_actual),
      quantity_difference: Number(i.quantity_difference),
      notes: i.notes,
    }));
  } catch (error) {
    ElMessage.error((error as Error).message || t('inventoryCount.listTab.messageFailure'));
  } finally {
    itemsLoading.value = false;
  }
};

const onVisibleChange = (val: boolean) => {
  emit('update:modelValue', val);
  if (val) void fetchDetail();
};

watch(
  () => props.modelValue,
  val => {
    if (val) void fetchDetail();
  }
);

// 批量提交实盘数量（recordCountItems：按 stock_id 匹配明细并自动计算差异）
const handleRecordItems = async () => {
  if (!props.currentRow?.id) return;
  recording.value = true;
  try {
    await recordCountItems(
      props.currentRow.id,
      items.value.map(it => ({
        stock_id: it.stock_id,
        // 后端 RecordItemInput.quantity_actual 是字符串（避免 JSON 浮点精度丢失）
        quantity_actual: String(it.quantity_actual),
        notes: it.notes || undefined,
      }))
    );
    ElMessage.success(t('inventoryCount.listTab.messageSuccess'));
    await fetchDetail();
  } catch (error) {
    ElMessage.error((error as Error).message || t('inventoryCount.listTab.messageFailure'));
  } finally {
    recording.value = false;
  }
};

// 单行保存实盘数量
const handleUpdateItem = async (row: CountDetailItem) => {
  try {
    await updateCountItem(row.id, { quantity_actual: String(Number(row.quantity_actual)) });
    ElMessage.success(t('inventoryCount.listTab.messageSuccess'));
    await fetchDetail();
  } catch (error) {
    ElMessage.error((error as Error).message || t('inventoryCount.listTab.messageFailure'));
  }
};

// 删除明细行
const handleDeleteItem = async (row: CountDetailItem) => {
  try {
    await ElMessageBox.confirm(
      t('inventoryCount.detailDialogTab.messageDeleteItemConfirm'),
      t('inventoryCount.listTab.titleDeleteConfirm'),
      { type: 'warning' }
    );
  } catch {
    return;
  }
  try {
    await deleteCountItem(row.id);
    ElMessage.success(t('inventoryCount.listTab.messageSuccess'));
    await fetchDetail();
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error((error as Error).message || t('inventoryCount.listTab.messageFailure'));
    }
  }
};
</script>

<style scoped>
.items-section {
  margin-top: 16px;
}
.items-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}
.items-title {
  font-weight: 600;
  font-size: 14px;
}
.diff-negative {
  color: var(--el-color-danger);
}
.diff-positive {
  color: var(--el-color-success);
}
</style>
