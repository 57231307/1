<!--
  TransferFormDialogTab.vue - 调拨单编辑对话框
  功能：新建或修改库存调拨单（调出仓与调入仓、日期、逐行选定出库库存四维与数量），
  查看态回源后端明细并支持 pending 单行级增删改。
  调用方：views/inventory-transfer/index.vue（列表页挂载本对话框）。
  入参：modelValue 显隐、currentRow 编辑或查看的行、warehouses/products 主数据、mode 三态。
  传给谁：建单走 createInventoryTransfer（POST /inventory/transfers），明细键与后端
  services/inv/mod.rs InventoryTransferItemRequest 逐字段对齐（quantity/unit_cost 为
  Decimal 十进制串）；行级维护走 createTransferItem/updateTransferItem/deleteTransferItem。
  存什么·存哪里：数量与单价按串采集原样送后端落 Decimal 列；金额不在前端计算（明细金额
  由后端按生效值重算，前端乘算属第二口径，禁止恢复 calcAmount 类本地折算）。

  数值录入一律 el-input 加范围校验器（全项目禁步进数字框）；明细行数量必须大于 0，
  与后端建单门（inv/stock.rs 与 inv/batch.rs 的"零与负数不构成出库"拒绝）同口径。
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
        <el-form-item
          :prop="`items.${index}.quantity`"
          :rules="quantityRules"
          label-width="0"
          class="item-number inline-form-item"
        >
          <el-input
            v-model="item.quantity"
            :placeholder="$t('inventoryTransfer.transferForm.quantityPlaceholder')"
          />
        </el-form-item>
        <el-form-item
          :prop="`items.${index}.unit_cost`"
          :rules="unitCostRules"
          label-width="0"
          class="item-number inline-form-item"
        >
          <el-input
            v-model="item.unit_cost"
            :placeholder="$t('inventoryTransfer.transferForm.pricePlaceholder')"
          />
        </el-form-item>
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
    </el-form>

    <!-- 查看模式：后端明细回源与行级维护。本节必须位于 el-form 之外：el-form 的
         :disabled 沿表单上下文级联禁用全部子控件，原实现在表单内渲染行级编辑器，
         编辑框与保存按钮恒为禁用态（死 UI），移出后生效。 -->
    <template v-if="mode === 'view' && formData.id">
      <el-divider content-position="left">{{
        $t('inventoryTransfer.transferForm.serverItemsDivider')
      }}</el-divider>
      <el-table v-loading="detailLoading" :data="serverItems" border size="small" max-height="300">
        <el-table-column prop="id" label="ID" width="60" />
        <el-table-column
          prop="product_id"
          :label="$t('inventoryTransfer.transferForm.colProductId')"
          width="100"
        />
        <el-table-column :label="$t('inventoryTransfer.transferForm.colQuantity')" width="150">
          <template #default="{ row }">
            <el-input v-if="editable" v-model="row.quantity" size="small" />
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
        <!-- 手输物料主键的旧入口废弃：改为按名称可搜索的产品选择（主数据 products 全量） -->
        <el-select
          v-model="newItem.product_id"
          filterable
          :placeholder="$t('inventoryTransfer.transferForm.productSelectPlaceholder')"
          style="width: 260px"
        >
          <el-option
            v-for="p in products"
            :key="p.id"
            :label="`${p.product_code} - ${p.product_name}`"
            :value="p.id"
          />
        </el-select>
        <el-input
          v-model="newItem.quantity"
          :placeholder="$t('inventoryTransfer.transferForm.quantityPlaceholder')"
          style="width: 130px"
        />
        <el-button type="primary" plain :loading="itemSaving" @click="handleAddItem">
          {{ $t('inventoryTransfer.transferForm.addItem') }}
        </el-button>
      </div>
    </template>
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
import { ElMessage, ElMessageBox, type FormItemRule, type FormInstance } from 'element-plus';
import { isDialogDismissal, rethrowNonDismissal } from '@/utils/monitor';
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

// 明细行数量与单价是十进制串（后端 Decimal 契约），金额不在前端计算：
// 明细金额由后端按生效值重算，前端本地乘算是会被漂移的第二口径（已按红线移除 calcAmount）
interface TransferItemForm {
  product_id: number;
  quantity: string;
  unit_cost: string;
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
  quantity: '',
  unit_cost: '',
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

/** 十进制串解析：有限正数返回数值，否则 null（建单门与后端"零与负数不构成出库"同口径） */
const parsePositive = (raw: string): number | null => {
  const n = Number((raw ?? '').trim());
  return Number.isFinite(n) && n > 0 ? n : null;
};

// 明细数量校验器：必填且为有限正数
const quantityRules: FormItemRule[] = [
  {
    validator: (_rule: unknown, value: string, callback: (err?: Error) => void) => {
      callback(
        parsePositive(value) !== null
          ? undefined
          : new Error(t('inventoryTransfer.transferForm.qtyPositive'))
      );
    },
    trigger: 'blur',
  },
];

// 单价校验器：留空=未采集（提交省略键），填了必须是不小于 0 的有限数
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
          : new Error(t('inventoryTransfer.transferForm.priceNonNegative'))
      );
    },
    trigger: 'blur',
  },
];

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

watch(
  () => props.modelValue,
  val => {
    if (val) {
      if (props.currentRow) {
        Object.assign(formData, props.currentRow);
        // 归一化明细行：补齐出库四维字段默认值（服务端行可能缺 stock_row_key 等前端字段；
        // 服务端匹号为可空列，白坯/历史行为 null，归一为空串=未选定态；
        // 数量与单价按十进制串承载，服务端 Decimal 序列化即为串）
        formData.items = (formData.items || []).map(i => ({
          ...newItemForm(),
          ...i,
          quantity: String((i as Partial<TransferItemForm>).quantity ?? ''),
          unit_cost: String((i as { unit_cost?: string | null }).unit_cost ?? ''),
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
// 明细出参数值列为 Decimal 串；行内编辑按串承载，保存前经 parsePositive 判定，不折 number
interface ServerItem {
  id: number;
  product_id: number;
  quantity: string;
  shipped_quantity: string;
  received_quantity: string;
  unit_cost?: string | null;
  notes?: string | null;
}

const serverItems = ref<ServerItem[]>([]);
const detailLoading = ref(false);
const itemSaving = ref(false);
const newItem = reactive({ product_id: undefined as number | undefined, quantity: '' });

const editable = computed(() => {
  const s = formData.status;
  // 已发出/已完成不可再编辑（词表见 models/status/purchase_inventory.rs 的
  // inventory_transfer::ALL，后端没有 received 这一值）
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
  if (!formData.id) return;
  if (!newItem.product_id) {
    ElMessage.warning(t('inventoryTransfer.transferForm.productSelectRequired'));
    return;
  }
  if (parsePositive(newItem.quantity) === null) {
    ElMessage.warning(t('inventoryTransfer.transferForm.qtyPositive'));
    return;
  }
  itemSaving.value = true;
  try {
    await createTransferItem(formData.id, {
      product_id: newItem.product_id,
      quantity: newItem.quantity.trim(),
    });
    ElMessage.success(t('inventoryTransfer.transferList.message.success'));
    newItem.product_id = undefined;
    newItem.quantity = '';
    await fetchServerItems();
  } catch (e) {
    ElMessage.error((e as Error).message || t('inventoryTransfer.transferList.message.failure'));
  } finally {
    itemSaving.value = false;
  }
};

const handleSaveItem = async (row: ServerItem) => {
  if (parsePositive(row.quantity) === null) {
    ElMessage.warning(t('inventoryTransfer.transferForm.qtyPositive'));
    return;
  }
  try {
    await updateTransferItem(row.id, {
      product_id: row.product_id,
      quantity: row.quantity.trim(),
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
    if (!isDialogDismissal(e)) {
      ElMessage.error((e as Error).message || t('inventoryTransfer.transferList.message.failure'));
    }
  }
};

/**
 * 明细行 → 后端入参：字段名逐一对应 services/inv/mod.rs InventoryTransferItemRequest。
 * 数量与单价按十进制串原样送（后端按生效值重算金额，前端不乘算）；
 * 单价留空=未采集，省略 unit_cost 键（后端 Option<Decimal> 缺席不写列，塌 "0" 属伪装实测）。
 * 匹号（第四维）：染色布经提交前校验保证已选定真实 AVAILABLE 匹后携带；
 * 白坯/未选定省略该键（禁发 null/空串占位，后端 Option<String> 缺席即不写）。
 */
const toItemPayload = (item: TransferItemForm): InventoryTransferItemPayload => {
  const payload: InventoryTransferItemPayload = {
    product_id: item.product_id,
    quantity: item.quantity.trim(),
    notes: item.remark,
    color_no: item.color_no,
    dye_lot_no: item.dye_lot_no,
    batch_no: item.batch_no,
  };
  if (item.unit_cost.trim()) {
    payload.unit_cost = item.unit_cost.trim();
  }
  if (item.color_no && item.piece_no) {
    payload.piece_no = item.piece_no;
  }
  return payload;
};

const handleSubmit = async () => {
  // 出库维度口径与后端同源（fabric_class 唯一判定，禁止第二份规则）：
  // 批次必填；色号非空的染色布必须齐缸号，色号为空的白坯布免缸号。
  const missingDim = formData.items.find(
    i =>
      i.product_id &&
      parsePositive(i.quantity) !== null &&
      (!i.batch_no || (i.color_no && !i.dye_lot_no))
  );
  if (missingDim) {
    ElMessage.warning(t('inventoryTransfer.transferForm.stockRowRequired'));
    return;
  }
  // 第四维匹号：色号非空的染色布必选（后端 fabric_class::normalize_outbound_piece_no 强制，
  // 且建单期按 产品+调出仓+缸+批+匹 全 tuple 预检真实可用匹）；白坯布免匹号，行为不变。
  const missingPiece = formData.items.find(
    i => i.product_id && parsePositive(i.quantity) !== null && i.color_no && !i.piece_no
  );
  if (missingPiece) {
    ElMessage.warning(t('inventoryTransfer.transferForm.pieceNoRequired'));
    return;
  }
  const valid = await formRef.value?.validate().catch(() => false);
  if (!valid) return;
  const items = formData.items
    .filter(i => i.product_id && parsePositive(i.quantity) !== null)
    .map(toItemPayload);
  // 全部明细既无产品又无数量的空单不提交：后端会以空 items 建出无痕调拨单
  if (items.length === 0) {
    ElMessage.warning(t('inventoryTransfer.transferForm.noValidItem'));
    return;
  }
  submitLoading.value = true;
  try {
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
  明细行布局约束：数量/单价为 el-input（无步进按钮），最小宽度按可容纳
  "99999.99" 精度的串长设定；select/备注给足 basis + 允许收缩 + min-width 下限
  （EP .el-select__selection 自带 min-width 归零加 ellipsis，长标签截断不撑破行），
  行容器超宽时 flex-wrap 换行兜底。明细行的表单校验项（el-form-item 包裹）去默认下边距。
*/
.transfer-item-row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
  margin-bottom: 8px;
}
.transfer-item-row .inline-form-item {
  margin-bottom: 0;
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
  flex: 1 0 130px;
  min-width: 130px;
}
.transfer-item-row .item-remark {
  flex: 1.5 1 120px;
  min-width: 100px;
}
.transfer-item-row .item-remove {
  flex: none;
}
</style>
