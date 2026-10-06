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
    width="1000px"
    destroy-on-close
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
      <div v-for="(item, index) in formData.items" :key="index" class="transfer-item-row">
        <el-select
          v-model="item.product_id"
          :placeholder="$t('inventoryTransfer.transferForm.productPlaceholder')"
          class="item-product"
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
        <!-- 出库四维库存行选择（色号+批次+缸号一次选定），数据来自调出仓真实库存行。
             长标签（色号·批次·缸号·可用量）在窄列内由 EP 自带 ellipsis 截断，
             全长内容经 tooltip 悬停可见，不依赖截断后的残段 -->
        <el-tooltip
          :content="selectedStockRowLabel(item)"
          :disabled="!selectedStockRowLabel(item)"
          placement="top"
        >
          <el-select
            v-model="item.stock_row_key"
            :placeholder="$t('inventoryTransfer.transferForm.stockRowPlaceholder')"
            :disabled="!formData.from_warehouse_id || !item.product_id"
            class="item-stock"
            @visible-change="(v: boolean) => v && void onStockRowDropdownOpen(item)"
            @change="(v: string) => void onStockRowChange(item, v)"
          >
            <el-option
              v-for="s in stockRowsMap[item.product_id] || []"
              :key="stockRowKey(s)"
              :label="stockRowLabel(s)"
              :value="stockRowKey(s)"
            />
          </el-select>
        </el-tooltip>
        <!-- 出库第四维（染色布=色号非空强制）：匹号只能从调出仓该缸该批真实 AVAILABLE 匹
             （GET /inventory/pieces 下推查询）中选定；白坯不参与匹号维度，不渲染本控件 -->
        <el-select
          v-if="item.color_no"
          v-model="item.piece_no"
          :placeholder="$t('inventoryTransfer.transferForm.pieceNoPlaceholder')"
          :disabled="!item.batch_no || !item.dye_lot_no"
          class="item-piece"
          @visible-change="(v: boolean) => v && void loadAvailablePieces(item)"
        >
          <el-option
            v-for="p in availablePieces(item)"
            :key="p.piece_no"
            :label="pieceOptionLabel(p)"
            :value="p.piece_no"
          />
        </el-select>
        <el-input-number
          v-model="item.quantity"
          :min="1"
          :placeholder="$t('inventoryTransfer.transferForm.quantityPlaceholder')"
          class="item-number"
        />
        <el-input-number
          v-model="item.cost_price"
          :min="0"
          :precision="2"
          :placeholder="$t('inventoryTransfer.transferForm.pricePlaceholder')"
          class="item-number"
        />
        <el-input
          v-model="item.remark"
          :placeholder="$t('inventoryTransfer.transferForm.remarkPlaceholder')"
          class="item-remark"
        />
        <el-button
          type="danger"
          :icon="Delete"
          circle
          class="item-remove"
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
import { isDialogDismissal, rethrowNonDismissal } from '@/utils/monitor';
import type { FormInstance } from 'element-plus';
import { Plus, Delete } from '@element-plus/icons-vue';
import {
  createInventoryTransfer,
  updateInventoryTransfer,
  getInventoryTransfer,
  createTransferItem,
  updateTransferItem,
  deleteTransferItem,
  getAvailablePieces,
  INVENTORY_PIECE_STATUS,
  type CreateInventoryTransferPayload,
  type InventoryPieceRow,
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
  /**
   * 出库第四维匹号：染色布（色号非空）必填，取值仅允许来自该缸该批现存 AVAILABLE 真实匹
   * （GET /inventory/pieces 下推查询）；白坯恒为空串且提交时省略该键。
   */
  piece_no: string;
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
  piece_no: '',
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

/** 已选库存行的全长标签（下拉窄列内被 ellipsis 截断的部分经行上 tooltip 悬停展示）；未选定为空串=tooltip 禁用 */
const selectedStockRowLabel = (item: TransferItemForm): string => {
  if (!item.stock_row_key) return '';
  const stock = (stockRowsMap.value[item.product_id] || []).find(
    s => stockRowKey(s) === item.stock_row_key
  );
  return stock ? stockRowLabel(stock) : '';
};

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

// ===== 出库第四维（匹号）：调出仓该缸该批现存可出库真实匹 =====
// GET /inventory/pieces 按 产品+调出仓+批次+缸号+status=AVAILABLE 全维下推查询（不手写死数据、
// 不自由输入）；缓存键 = 产品 + 库存行三维组合键，跨缸/跨批互不混用。
const piecesMap = ref<Record<string, InventoryPieceRow[]>>({});

const piecesKey = (item: TransferItemForm) => `${item.product_id}__${stockRowKey(item)}`;

const availablePieces = (item: TransferItemForm) => piecesMap.value[piecesKey(item)] || [];

const pieceOptionLabel = (p: InventoryPieceRow) =>
  `${t('inventoryTransfer.transferForm.colPieceNo')}: ${p.piece_no} · ${t('inventoryTransfer.transferForm.colPieceLength')}: ${p.length}`;

const loadAvailablePieces = async (item: TransferItemForm) => {
  // 白坯（色号空）不参与匹号维度；染色布须先选定 批次+缸号 才可取匹
  if (!item.color_no || !item.product_id || !item.batch_no || !item.dye_lot_no) return;
  if (!formData.from_warehouse_id) return;
  const key = piecesKey(item);
  if (piecesMap.value[key]) return;
  try {
    const res = await getAvailablePieces({
      product_id: item.product_id,
      warehouse_id: formData.from_warehouse_id,
      batch_no: item.batch_no,
      dye_lot_no: item.dye_lot_no,
      status: INVENTORY_PIECE_STATUS.AVAILABLE,
      page: 1,
      page_size: 100,
    });
    piecesMap.value = {
      ...piecesMap.value,
      [key]: (res.data as { items?: InventoryPieceRow[] } | null)?.items || [],
    };
  } catch (error) {
    logger.error(t('inventoryTransfer.transferForm.pieceNoLoadFailed'), error);
    ElMessage.error(t('inventoryTransfer.transferForm.pieceNoLoadFailed'));
  }
};

const onItemProductChange = async (item: TransferItemForm) => {
  item.color_no = '';
  item.dye_lot_no = '';
  item.batch_no = '';
  item.stock_row_key = '';
  item.piece_no = '';
  await loadStockRows(item.product_id);
};

const onStockRowChange = async (item: TransferItemForm, key: string) => {
  // 维度改选即旧匹号失效（匹按 产品+缸+批 tuple 归属），先清空再按新维度重取
  item.piece_no = '';
  const stock = (stockRowsMap.value[item.product_id] || []).find(s => stockRowKey(s) === key);
  if (!stock) return;
  item.color_no = stock.color_no ?? '';
  item.batch_no = stock.batch_no ?? '';
  item.dye_lot_no = stock.dye_lot_no ?? '';
  await loadAvailablePieces(item);
};

// 调出仓变更：旧仓库存行失效，清空缓存与各行已选维度
watch(
  () => formData.from_warehouse_id,
  () => {
    stockRowsMap.value = {};
    piecesMap.value = {};
    formData.items.forEach(item => {
      item.color_no = '';
      item.dye_lot_no = '';
      item.batch_no = '';
      item.stock_row_key = '';
      item.piece_no = '';
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
  piecesMap.value = {};
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
        // 归一化明细行：补齐出库四维字段默认值（服务端行可能缺 stock_row_key 等前端字段；
        // 服务端匹号为可空列，白坯/历史行为 null，归一为空串=未选定态）
        formData.items = (formData.items || []).map(i => ({
          ...newItemForm(),
          ...i,
          piece_no: i.piece_no ?? '',
        }));
        if (!formData.items || formData.items.length === 0) {
          formData.items = [newItemForm()];
        }
        stockRowsMap.value = {};
        piecesMap.value = {};
        if (props.mode === 'view' && formData.id) void fetchServerItems();
      } else {
        resetForm();
      }
    } else {
      // 关闭即清空明细数据：destroy-on-close 已卸载对话框 DOM，此处同步重置 reactive items，
      // 确保复用的常驻组件在下一次打开时只渲染干净单行，杜绝残留的隐藏数量输入。
      resetForm();
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
  } catch (error: unknown) {
    if (isDialogDismissal(error)) return;
    rethrowNonDismissal('inventoryTransfer.handleDeleteItem', error);
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

/**
 * 明细行 → 后端入参：字段名逐一对应 services/inv/mod.rs InventoryTransferItemRequest。
 * 匹号（第四维）：染色布经提交前校验保证已选定真实 AVAILABLE 匹后携带；
 * 白坯/未选定**省略该键**（禁发 null/空串占位，后端 Option<String> 缺席即不写）。
 */
const toItemPayload = (item: TransferItemForm): InventoryTransferItemPayload => {
  const payload: InventoryTransferItemPayload = {
    product_id: item.product_id,
    quantity: String(item.quantity),
    unit_cost: String(item.cost_price),
    notes: item.remark,
    color_no: item.color_no,
    dye_lot_no: item.dye_lot_no,
    batch_no: item.batch_no,
  };
  if (item.color_no && item.piece_no) {
    payload.piece_no = item.piece_no;
  }
  return payload;
};

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
  // 第四维匹号：色号非空的染色布必选（后端 fabric_class::normalize_outbound_piece_no 强制，
  // 且建单期按 产品+调出仓+缸+批+匹 全 tuple 预检真实可用匹）；白坯布免匹号，行为不变。
  const missingPiece = formData.items.find(
    i => i.product_id && i.quantity > 0 && i.color_no && !i.piece_no
  );
  if (missingPiece) {
    ElMessage.warning(t('inventoryTransfer.transferForm.pieceNoRequired'));
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

<style scoped>
/*
  明细行布局约束（数量/单价 stepper 可用宽度不变式）：
  EP 2.14.4 `.el-input-number .el-input__wrapper{padding-left:42px;padding-right:42px}`
  为左右 −/+ 步进按钮预留 84px；内层数字 input 的计算宽度 = stepper 盒宽 − 84px，
  stepper 盒宽必须严格大于 84px 数字区才存在（否则 input 宽 0、不可填也不可聚焦）。
  `flex: N` 速记会把 flex-basis 归零、宽度纯按比例分配且无 min-width 保底，
  因此这里给两个 stepper 显式 flex-basis=150px 且 flex-shrink:0（150 = 84 + 约 64px
  数字区，可容 "99999.99" 精度的单价），select/备注给足 basis + 允许收缩 + min-width
  下限（EP `.el-select__selection` 自带 min-width:0 + ellipsis，长标签截断不撑破行），
  行容器超宽时 flex-wrap 换行兜底——换行后每行重新分配，stepper 仍不低于 150px。
*/
.transfer-item-row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
  margin-bottom: 8px;
}
.transfer-item-row .item-product {
  flex: 2 1 160px;
  min-width: 120px;
}
.transfer-item-row .item-stock {
  flex: 3 1 180px;
  min-width: 140px;
}
.transfer-item-row .item-piece {
  flex: 1.5 1 120px;
  min-width: 100px;
}
.transfer-item-row .item-number {
  flex: 1 0 150px;
  min-width: 150px;
}
.transfer-item-row .item-number {
  flex: 1 0 150px;
  min-width: 150px;
}
.transfer-item-row .item-remark {
  flex: 1.5 1 120px;
  min-width: 100px;
}
.transfer-item-row .item-remove {
  flex: none;
}
</style>
