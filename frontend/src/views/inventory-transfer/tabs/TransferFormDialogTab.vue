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
          @change="() => void onItemProductChange(item)"
        >
          <el-option
            v-for="p in products"
            :key="p.id"
            :label="`${p.product_code} - ${p.product_name}`"
            :value="p.id"
          />
        </el-select>
        <!-- 出库四维库存行选择（色号+批次+缸号一次选定），数据来自调出仓真实库存行 -->
        <el-select
          v-model="item.stock_row_key"
          :placeholder="$t('inventoryTransfer.transferForm.stockRowPlaceholder')"
          :disabled="!formData.from_warehouse_id || !item.product_id"
          style="flex: 3"
          @visible-change="(v: boolean) => v && void onStockRowDropdownOpen(item)"
          @change="(v: string) => onStockRowChange(item, v)"
        >
          <el-option
            v-for="s in stockRowsMap[item.product_id] || []"
            :key="stockRowKey(s)"
            :label="stockRowLabel(s)"
            :value="stockRowKey(s)"
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
  type CreateInventoryTransferPayload,
  type InventoryTransferEntity,
  type InventoryTransferItemPayload,
} from '@/api/inventory-transfer';
import { getStockList, type InventoryStock } from '@/api/inventory';
import type { Warehouse } from '@/api/warehouse';
import type { Product } from '@/api/product';
import { logger } from '@/utils/logger';
import {
  INVENTORY_TRANSFER_STATUS,
  type InventoryTransferStatus,
} from '@/utils/inventory-transfer-status';

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

interface TransferItemForm {
  product_id: number;
  quantity: number;
  cost_price: number;
  amount: number;
  remark: string;
  /** 出库四维（款号+色号+缸号+批次）：由调出仓真实库存行选定 */
  color_no: string;
  dye_lot_no: string;
  batch_no: string;
  /** 选中库存行的组合键（选项定位用） */
  stock_row_key: string;
}

const newItemForm = (): TransferItemForm => ({
  product_id: 0,
  quantity: 1,
  cost_price: 0,
  amount: 0,
  remark: '',
  color_no: '',
  dye_lot_no: '',
  batch_no: '',
  stock_row_key: '',
});

const formData = reactive({
  id: 0,
  /** 日期控件按 YYYY-MM-DD 取值，提交前转 RFC3339（后端 transfer_date 是 DateTime<Utc>） */
  transfer_date: new Date().toISOString().split('T')[0],
  from_warehouse_id: undefined as number | undefined,
  to_warehouse_id: undefined as number | undefined,
  status: INVENTORY_TRANSFER_STATUS.PENDING as InventoryTransferStatus,
  items: [newItemForm()] as TransferItemForm[],
});

// ===== 出库四维库存行（GET /inventory/stock 按调出仓+产品下推查询，不手写死数据） =====
const stockRowsMap = ref<Record<number, InventoryStock[]>>({});

const stockRowKey = (row: Pick<InventoryStock, 'color_no' | 'batch_no' | 'dye_lot_no'>) =>
  `${row.color_no ?? ''}__${row.batch_no ?? ''}__${row.dye_lot_no ?? ''}`;

const stockRowLabel = (s: InventoryStock) =>
  `${t('inventoryTransfer.transferForm.colColorNo')}: ${s.color_no} · ${t('inventoryTransfer.transferForm.colBatchNo')}: ${s.batch_no} · ${t('inventoryTransfer.transferForm.colDyeLotNo')}: ${s.dye_lot_no || '-'} · ${t('inventoryTransfer.transferForm.availableQty')}: ${s.quantity_available}`;

const loadStockRows = async (productId: number) => {
  if (!formData.from_warehouse_id || !productId) return;
  if (stockRowsMap.value[productId]) return;
  try {
    const res = await getStockList({
      warehouse_id: formData.from_warehouse_id,
      product_id: productId,
      page: 1,
      page_size: 100,
    });
    stockRowsMap.value = {
      ...stockRowsMap.value,
      [productId]: (res.data as { items?: InventoryStock[] } | null)?.items || [],
    };
  } catch (error) {
    logger.error(t('inventoryTransfer.transferForm.stockRowLoadFailed'), error);
    ElMessage.error(t('inventoryTransfer.transferForm.stockRowLoadFailed'));
  }
};

const onStockRowDropdownOpen = async (item: TransferItemForm) => {
  await loadStockRows(item.product_id);
};

const onItemProductChange = async (item: TransferItemForm) => {
  item.color_no = '';
  item.dye_lot_no = '';
  item.batch_no = '';
  item.stock_row_key = '';
  await loadStockRows(item.product_id);
};

const onStockRowChange = (item: TransferItemForm, key: string) => {
  const stock = (stockRowsMap.value[item.product_id] || []).find(s => stockRowKey(s) === key);
  if (!stock) return;
  item.color_no = stock.color_no ?? '';
  item.batch_no = stock.batch_no ?? '';
  item.dye_lot_no = stock.dye_lot_no ?? '';
};

// 调出仓变更：旧仓库存行失效，清空缓存与各行已选维度
watch(
  () => formData.from_warehouse_id,
  () => {
    stockRowsMap.value = {};
    formData.items.forEach(item => {
      item.color_no = '';
      item.dye_lot_no = '';
      item.batch_no = '';
      item.stock_row_key = '';
    });
  }
);

const resetForm = () => {
  formData.id = 0;
  formData.transfer_date = new Date().toISOString().split('T')[0];
  formData.from_warehouse_id = undefined;
  formData.to_warehouse_id = undefined;
  formData.status = INVENTORY_TRANSFER_STATUS.PENDING;
  formData.items = [newItemForm()];
  stockRowsMap.value = {};
};

const addItem = () => {
  formData.items.push(newItemForm());
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
        // 归一化明细行：补齐出库四维字段默认值（服务端行可能缺 stock_row_key 等前端字段）
        formData.items = (formData.items || []).map(i => ({ ...newItemForm(), ...i }));
        if (!formData.items || formData.items.length === 0) {
          formData.items = [newItemForm()];
        }
        stockRowsMap.value = {};
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
  const s = formData.status;
  // 已发出/已完成不可再编辑（词表见 models/status/purchase_inventory.rs:63，
  // 后端没有 received 这一值）
  return s !== INVENTORY_TRANSFER_STATUS.SHIPPED && s !== INVENTORY_TRANSFER_STATUS.COMPLETED;
});

const fetchServerItems = async () => {
  detailLoading.value = true;
  try {
    const res = (await getInventoryTransfer(formData.id)) as {
      data?: { items?: ServerItem[] };
    };
    serverItems.value = res.data?.items ?? [];
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
      quantity: String(newItem.quantity),
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
      quantity: String(Number(row.quantity)),
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

/** 明细行 → 后端入参：字段名逐一对应 services/inv/mod.rs:87 InventoryTransferItemRequest */
const toItemPayload = (item: TransferItemForm): InventoryTransferItemPayload => ({
  product_id: item.product_id,
  quantity: String(item.quantity),
  unit_cost: String(item.cost_price),
  notes: item.remark,
  color_no: item.color_no,
  dye_lot_no: item.dye_lot_no,
  batch_no: item.batch_no,
});

const handleSubmit = async () => {
  // 出库维度口径与后端同源（fabric_class 唯一判定，禁止第二份规则）：
  // 批次必填；色号非空的染色布必须齐缸号，色号为空的白坯布免缸号。
  const missingDim = formData.items.find(
    i => i.product_id && i.quantity > 0 && (!i.batch_no || (i.color_no && !i.dye_lot_no))
  );
  if (missingDim) {
    ElMessage.warning(t('inventoryTransfer.transferForm.stockRowRequired'));
    return;
  }
  submitLoading.value = true;
  try {
    formData.items.forEach(calcAmount);
    const items = formData.items.filter(i => i.product_id && i.quantity > 0).map(toItemPayload);
    if (formData.id) {
      // UpdateInventoryTransferRequest 只有 status/notes/items 三字段
      await updateInventoryTransfer(formData.id, { items });
    } else {
      const payload: CreateInventoryTransferPayload = {
        from_warehouse_id: formData.from_warehouse_id,
        to_warehouse_id: formData.to_warehouse_id,
        transfer_date: new Date(formData.transfer_date).toISOString(),
        items,
      };
      await createInventoryTransfer(payload);
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
