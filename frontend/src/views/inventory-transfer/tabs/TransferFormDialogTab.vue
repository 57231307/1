<!--
  TransferFormDialogTab.vue - 调拨单编辑对话框
  来源：原 inventoryTransfer/index.vue 中 调拨单编辑对话框
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    :title="
      formData.id
        ? $t('inventoryTransfer.transferForm.editTitle')
        : $t('inventoryTransfer.transferForm.createTitle')
    "
    width="800px"
    :aria-label="
      mode === 'view'
        ? $t('inventoryTransfer.transferForm.viewDialogTitle')
        : formData.id
          ? $t('inventoryTransfer.transferForm.editDialogTitle')
          : $t('inventoryTransfer.transferForm.createDialogTitle')
    "
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-form
      ref="formRef"
      :model="formData"
      label-width="100px"
      :disabled="mode === 'view'"
      :aria-label="$t('inventoryTransfer.transferForm.formAriaLabel')"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item
            :label="$t('inventoryTransfer.transferForm.fromWarehouse')"
            prop="from_warehouse_id"
          >
            <el-select v-model="formData.from_warehouse_id" style="width: 100%">
              <el-option
                v-for="wh in warehouses"
                :key="wh.id"
                :label="wh.warehouse_name"
                :value="wh.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item
            :label="$t('inventoryTransfer.transferForm.toWarehouse')"
            prop="to_warehouse_id"
          >
            <el-select v-model="formData.to_warehouse_id" style="width: 100%">
              <el-option
                v-for="wh in warehouses"
                :key="wh.id"
                :label="wh.warehouse_name"
                :value="wh.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="$t('inventoryTransfer.transferForm.transferDate')" prop="transfer_date">
        <el-date-picker
          v-model="formData.transfer_date"
          type="date"
          value-format="YYYY-MM-DD"
          style="width: 100%"
        />
      </el-form-item>
      <el-divider content-position="left">{{
        $t('inventoryTransfer.transferForm.detailDivider')
      }}</el-divider>
      <div
        v-for="(item, index) in formData.items"
        :key="index"
        style="display: flex; gap: 8px; margin-bottom: 8px; align-items: center"
      >
        <el-select
          v-model="item.product_id"
          :placeholder="$t('inventoryTransfer.transferForm.productPlaceholder')"
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
          :placeholder="$t('inventoryTransfer.transferForm.quantityPlaceholder')"
          style="flex: 1"
        />
        <el-input-number
          v-model="item.cost_price"
          :min="0"
          :precision="2"
          :placeholder="$t('inventoryTransfer.transferForm.pricePlaceholder')"
          style="flex: 1"
        />
        <el-input
          v-model="item.remark"
          :placeholder="$t('inventoryTransfer.transferForm.remarkPlaceholder')"
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
        <el-icon><Plus /></el-icon>{{ $t('inventoryTransfer.transferForm.addItem') }}
      </el-button>

      <!-- 查看模式：后端明细回源与行级维护 -->
      <template v-if="mode === 'view' && formData.id">
        <el-divider content-position="left">{{
          $t('inventoryTransfer.transferForm.serverItemsDivider')
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
            prop="product_id"
            :label="$t('inventoryTransfer.transferForm.colProductId')"
            width="100"
          />
          <el-table-column :label="$t('inventoryTransfer.transferForm.colQuantity')" width="150">
            <template #default="{ row }">
              <el-input-number
                v-if="editable"
                v-model="row.quantity"
                :min="1"
                size="small"
                controls-position="right"
              />
              <span v-else>{{ row.quantity }}</span>
            </template>
          </el-table-column>
          <el-table-column
            prop="shipped_quantity"
            :label="$t('inventoryTransfer.transferForm.colShipped')"
            width="110"
          />
          <el-table-column
            prop="received_quantity"
            :label="$t('inventoryTransfer.transferForm.colReceived')"
            width="110"
          />
          <el-table-column
            :label="$t('inventoryTransfer.transferForm.colOperation')"
            width="140"
            fixed="right"
          >
            <template #default="{ row }">
              <template v-if="editable">
                <el-button link type="primary" size="small" @click="handleSaveItem(row)">{{
                  $t('inventoryTransfer.transferForm.saveItem')
                }}</el-button>
                <el-button link type="danger" size="small" @click="handleDeleteItem(row)">{{
                  $t('inventoryTransfer.transferForm.deleteItem')
                }}</el-button>
              </template>
              <span v-else>-</span>
            </template>
          </el-table-column>
        </el-table>
        <div v-if="editable" class="add-item-bar">
          <el-input-number
            v-model="newItem.product_id"
            :min="1"
            :placeholder="$t('inventoryTransfer.transferForm.colProductId')"
            style="width: 150px"
          />
          <el-input-number v-model="newItem.quantity" :min="1" style="width: 130px" />
          <el-button type="primary" plain :loading="itemSaving" @click="handleAddItem">
            {{ $t('inventoryTransfer.transferForm.addItem') }}
          </el-button>
        </div>
      </template>
    </el-form>
    <template v-if="mode !== 'view'" #footer>
      <el-button @click="emit('update:modelValue', false)">{{
        $t('inventoryTransfer.transferForm.cancel')
      }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
        $t('inventoryTransfer.transferForm.save')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import type { FormInstance } from 'element-plus';
import { Plus, Delete } from '@element-plus/icons-vue';
import {
  createInventoryTransfer,
  updateInventoryTransfer,
  getInventoryTransfer,
  createTransferItem,
  updateTransferItem,
  deleteTransferItem,
  type InventoryTransferEntity,
} from '@/api/inventory-transfer';
import type { Warehouse } from '@/api/warehouse';
import type { Product } from '@/api/product';
import { logger } from '@/utils/logger';

// 批次 34 v9 P1：接入 i18n，替换硬编码中文 ElMessage
const { t } = useI18n({ useScope: 'global' });

interface Props {
  modelValue: boolean;
  currentRow: InventoryTransferEntity | null;
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
  transfer_date: new Date().toISOString().split('T')[0],
  from_warehouse_id: undefined as number | undefined,
  to_warehouse_id: undefined as number | undefined,
  total_amount: 0,
  status: 'pending' as 'pending' | 'approved' | 'rejected' | 'shipped' | 'completed',
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
  formData.transfer_date = new Date().toISOString().split('T')[0];
  formData.from_warehouse_id = undefined;
  formData.to_warehouse_id = undefined;
  formData.total_amount = 0;
  formData.status = 'pending';
  formData.items = [{ product_id: 0, quantity: 1, cost_price: 0, amount: 0, remark: '' }];
};

const addItem = () => {
  formData.items.push({ product_id: 0, quantity: 1, cost_price: 0, amount: 0, remark: '' });
};

const removeItem = (index: number) => {
  if (formData.items.length > 1) {
    formData.items.splice(index, 1);
  }
};

const calcAmount = (item: { quantity: number; cost_price: number }) => {
  return item.quantity * item.cost_price;
};

watch(
  () => props.modelValue,
  val => {
    if (val) {
      if (props.currentRow) {
        Object.assign(formData, props.currentRow);
        if (!formData.items || formData.items.length === 0) {
          formData.items = [{ product_id: 0, quantity: 1, cost_price: 0, amount: 0, remark: '' }];
        }
        if (props.mode === 'view' && formData.id) void fetchServerItems();
      } else {
        resetForm();
      }
    }
  }
);

// ===== 查看模式：后端明细回源与行级维护（getInventoryTransfer + 明细 CRUD） =====
interface ServerItem {
  id: number;
  product_id: number;
  quantity: number;
  shipped_quantity: number;
  received_quantity: number;
  unit_cost?: number;
  notes?: string | null;
}

const serverItems = ref<ServerItem[]>([]);
const detailLoading = ref(false);
const itemSaving = ref(false);
const newItem = reactive({ product_id: 1, quantity: 1 });

const editable = computed(() => {
  const s = (formData.status || '').toLowerCase();
  // 已发出/已完成不可再编辑（后端状态集合里没有 received 这一值）
  return s !== 'shipped' && s !== 'completed';
});

const fetchServerItems = async () => {
  detailLoading.value = true;
  try {
    const res = (await getInventoryTransfer(formData.id)) as {
      data?: { items?: ServerItem[] };
      items?: ServerItem[];
    };
    serverItems.value = res.data?.items ?? res.items ?? [];
  } catch (e) {
    ElMessage.error((e as Error).message || t('inventoryTransfer.transferList.message.failure'));
  } finally {
    detailLoading.value = false;
  }
};

const handleAddItem = async () => {
  if (!formData.id || !newItem.product_id || !newItem.quantity) return;
  itemSaving.value = true;
  try {
    await createTransferItem(formData.id, {
      product_id: newItem.product_id,
      quantity: newItem.quantity,
    });
    ElMessage.success(t('inventoryTransfer.transferList.message.success'));
    await fetchServerItems();
  } catch (e) {
    ElMessage.error((e as Error).message || t('inventoryTransfer.transferList.message.failure'));
  } finally {
    itemSaving.value = false;
  }
};

const handleSaveItem = async (row: ServerItem) => {
  try {
    await updateTransferItem(row.id, {
      product_id: row.product_id,
      quantity: Number(row.quantity),
    });
    ElMessage.success(t('inventoryTransfer.transferList.message.success'));
    await fetchServerItems();
  } catch (e) {
    ElMessage.error((e as Error).message || t('inventoryTransfer.transferList.message.failure'));
  }
};

const handleDeleteItem = async (row: ServerItem) => {
  try {
    await ElMessageBox.confirm(
      t('inventoryTransfer.transferForm.deleteItemConfirm'),
      t('inventoryTransfer.transferList.message.deleteTitle'),
      { type: 'warning' }
    );
  } catch {
    return;
  }
  try {
    await deleteTransferItem(row.id);
    ElMessage.success(t('inventoryTransfer.transferList.message.success'));
    await fetchServerItems();
  } catch (e) {
    if (e !== 'cancel') {
      ElMessage.error((e as Error).message || t('inventoryTransfer.transferList.message.failure'));
    }
  }
};

const handleSubmit = async () => {
  submitLoading.value = true;
  try {
    formData.items.forEach(calcAmount);
    formData.total_amount = formData.items.reduce(
      (sum, item) => sum + item.cost_price * item.quantity,
      0
    );
    if (formData.id) {
      await updateInventoryTransfer(formData.id, formData as Partial<InventoryTransferEntity>);
    } else {
      await createInventoryTransfer(formData as Partial<InventoryTransferEntity>);
    }
    ElMessage.success(t('message.operationSuccess'));
    emit('update:modelValue', false);
    emit('submitted');
  } catch (error) {
    ElMessage.error((error as Error).message || t('message.operationFailed'));
    logger.error(t('inventoryTransfer.transferForm.saveFailed'), (error as Error).message);
  } finally {
    submitLoading.value = false;
  }
};
</script>
