<!--
  AdjustmentFormDialogTab.vue - 库存调整编辑对话框
  来源：原 inventoryAdjustment/index.vue 中 调整单编辑对话框
  拆分日期：2026-06-15 B3-4
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
            prop="adjust_no"
          >
            <el-input v-model="formData.adjust_no" :disabled="!!formData.id" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item
            :label="t('inventoryAdjustment.formDialogTab.labelAdjustDate')"
            prop="adjust_date"
          >
            <el-date-picker
              v-model="formData.adjust_date"
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
        <el-select v-model="formData.warehouse_id" style="width: 100%">
          <el-option
            v-for="wh in warehouses"
            :key="wh.id"
            :label="wh.warehouse_name"
            :value="wh.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('inventoryAdjustment.formDialogTab.labelReason')" prop="reason">
        <el-input
          v-model="formData.reason"
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
        style="display: flex; gap: 8px; margin-bottom: 8px; align-items: center"
      >
        <el-select
          v-model="item.product_id"
          :placeholder="t('inventoryAdjustment.formDialogTab.placeholderProduct')"
          style="flex: 2"
          filterable
        >
          <el-option
            v-for="p in products"
            :key="p.id"
            :label="`${p.product_code} - ${p.product_name}`"
            :value="p.id"
          />
        </el-select>
        <el-input-number
          v-model="item.quantity"
          :min="1"
          :placeholder="t('inventoryAdjustment.formDialogTab.placeholderQuantity')"
          style="flex: 1"
        />
        <el-input-number
          v-model="item.cost_price"
          :min="0"
          :precision="2"
          :placeholder="t('inventoryAdjustment.formDialogTab.placeholderPrice')"
          style="flex: 1"
        />
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

      <!-- 详情回源：查看模式下展示后端最新明细，pending 态支持行编辑/删除/新增 -->
      <template v-if="mode === 'view' && formData.id">
        <el-divider content-position="left">{{
          t('inventoryAdjustment.formDialogTab.dividerServerItems')
        }}</el-divider>
        <el-table
          v-loading="detailLoading"
          :data="serverItems"
          border
          size="small"
          max-height="300"
        >
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
              <el-input-number
                v-if="isPending"
                v-model="row.quantity"
                :min="0"
                size="small"
                controls-position="right"
              />
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
          <el-input-number
            v-model="newItem.stock_id"
            :min="1"
            :placeholder="t('inventoryAdjustment.formDialogTab.colStockId')"
            style="width: 150px"
          />
          <el-input-number
            v-model="newItem.quantity"
            :min="1"
            style="width: 130px"
            :placeholder="t('inventoryAdjustment.formDialogTab.placeholderQuantity')"
          />
          <el-button type="primary" plain :loading="itemSaving" @click="handleAddItem">{{
            t('inventoryAdjustment.formDialogTab.buttonAddItem')
          }}</el-button>
        </div>
      </template>
    </el-form>
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
import { ElMessage, ElMessageBox } from 'element-plus';
import type { FormInstance } from 'element-plus';
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
import type { Warehouse } from '@/api/warehouse';
import type { Product } from '@/api/product';
import { logger } from '@/utils/logger';

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

const formData = reactive({
  id: 0,
  adjust_no: '',
  adjust_date: new Date().toISOString().split('T')[0],
  warehouse_id: undefined as number | undefined,
  reason: '',
  status: 'pending' as 'pending' | 'approved' | 'rejected',
  total_amount: 0,
  items: [{ product_id: 0, quantity: 1, cost_price: 0, amount: 0, remark: '' }] as {
    product_id: number;
    quantity: number;
    cost_price: number;
    amount: number;
    remark: string;
  }[],
});

const resetForm = () => {
  formData.id = 0;
  formData.adjust_no = '';
  formData.adjust_date = new Date().toISOString().split('T')[0];
  formData.warehouse_id = undefined;
  formData.reason = '';
  formData.status = 'pending';
  formData.total_amount = 0;
  formData.items = [{ product_id: 0, quantity: 1, cost_price: 0, amount: 0, remark: '' }];
};

const addItem = () => {
  formData.items.push({ product_id: 0, quantity: 1, cost_price: 0, amount: 0, remark: '' });
};
const removeItem = (index: number) => {
  if (formData.items.length > 1) formData.items.splice(index, 1);
};

const generateNo = async () => {
  try {
    const res = await generateInventoryAdjustmentNo();
    formData.adjust_no = res.data?.adjustment_no || '';
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
        if (!formData.items || formData.items.length === 0) {
          formData.items = [{ product_id: 0, quantity: 1, cost_price: 0, amount: 0, remark: '' }];
        }
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
interface ServerItem {
  id: number;
  stock_id: number;
  quantity: number;
  quantity_before: number;
  quantity_after: number;
  unit_cost?: number;
  amount?: number;
  notes?: string | null;
}

const serverItems = ref<ServerItem[]>([]);
const detailLoading = ref(false);
const itemSaving = ref(false);
const newItem = reactive({ stock_id: 1, quantity: 1 });

const isPending = computed(() => (formData.status || '').toLowerCase() === 'pending');

const fetchServerItems = async () => {
  detailLoading.value = true;
  try {
    const res = (await getInventoryAdjustment(formData.id)) as {
      data?: { items?: ServerItem[] };
      items?: ServerItem[];
    };
    serverItems.value = res.data?.items ?? res.items ?? [];
  } catch (e) {
    ElMessage.error((e as Error).message || t('inventoryAdjustment.listTab.messageFailure'));
  } finally {
    detailLoading.value = false;
  }
};

const handleAddItem = async () => {
  if (!formData.id || !newItem.stock_id || !newItem.quantity) {
    ElMessage.warning(t('inventoryAdjustment.formDialogTab.placeholderQuantity'));
    return;
  }
  itemSaving.value = true;
  try {
    await createAdjustmentItem(formData.id, {
      stock_id: newItem.stock_id,
      quantity: newItem.quantity,
    });
    ElMessage.success(t('inventoryAdjustment.listTab.messageSuccess'));
    await fetchServerItems();
  } catch (e) {
    ElMessage.error((e as Error).message || t('inventoryAdjustment.listTab.messageFailure'));
  } finally {
    itemSaving.value = false;
  }
};

const handleSaveItem = async (row: ServerItem) => {
  try {
    await updateAdjustmentItem(row.id, { stock_id: row.stock_id, quantity: Number(row.quantity) });
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
  } catch {
    return;
  }
  try {
    await deleteAdjustmentItem(row.id);
    ElMessage.success(t('inventoryAdjustment.listTab.messageSuccess'));
    await fetchServerItems();
  } catch (e) {
    if (e !== 'cancel') {
      ElMessage.error((e as Error).message || t('inventoryAdjustment.listTab.messageFailure'));
    }
  }
};

const handleSubmit = async () => {
  submitLoading.value = true;
  try {
    formData.total_amount = formData.items.reduce(
      (sum, item) => sum + item.cost_price * item.quantity,
      0
    );
    if (formData.id) {
      await updateInventoryAdjustment(formData.id, formData as Partial<InventoryAdjustmentEntity>);
    } else {
      await createInventoryAdjustment(formData as Partial<InventoryAdjustmentEntity>);
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
