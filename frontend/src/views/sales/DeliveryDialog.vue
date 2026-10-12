<!--
  DeliveryDialog.vue - 销售发货对话框
  来源：原 sales/index.vue 中 发货 dialog
  拆分日期：2026-06-15 B3-1
  P9-3 批次 F 重构：移除 vue/no-mutating-props 抑制，通过 emit 整体覆盖 + 局部 update
  出库维度规则（用户 2026-10-02 口径：出库对染色布强制四维=缸号/色号/批次/匹号，款号由产品承载）：
  每条发货明细必须选择真实入库库存行（批次必填；色号非空的染色布还须带缸号与匹号，
  色号为空的白坯布免缸号免匹号）。
  前三维选项来自后端 GET /inventory/stock 按产品+仓库下推查询，第四维匹号来自
  GET /inventory/pieces 按 产品+仓库+批次+缸号+status=AVAILABLE 下推查询（父组件加载后传入），
  不手写死数据、不允许自由输入。
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('sales.delivery.title')"
    width="800px"
    :aria-label="t('sales.delivery.dialogAriaLabel')"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form :aria-label="t('sales.delivery.formAriaLabel')">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('sales.delivery.salesOrderNo')">
            <el-input :model-value="form.order_no" readonly />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('sales.delivery.customer')">
            <el-input :model-value="form.customer_name" readonly />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('sales.delivery.deliveryDate')" required>
            <el-date-picker
              :model-value="form.delivery_date"
              type="date"
              :placeholder="t('sales.delivery.datePlaceholder')"
              style="width: 100%"
              @update:model-value="(v: string) => updateForm('delivery_date', v)"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('sales.delivery.warehouse')" required>
            <el-select
              :model-value="form.warehouse_id"
              :placeholder="t('sales.delivery.warehousePlaceholder')"
              style="width: 100%"
              @update:model-value="(v: number) => onWarehouseChange(v)"
            >
              <el-option
                v-for="w in warehouses"
                :key="w.id"
                :label="w.warehouse_name"
                :value="w.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('sales.delivery.deliveryItems')">
        <el-table
          :data="form.items"
          border
          style="width: 100%"
          :aria-label="t('sales.delivery.itemsTableAriaLabel')"
        >
          <el-table-column prop="product_name" :label="t('sales.delivery.product')" width="150" />
          <el-table-column :label="t('sales.delivery.stockRow')" min-width="220">
            <template #default="{ row }">
              <!-- 出库前三维库存行选择：色号/批次/缸号一次选定，来源为真实库存行 -->
              <el-select
                :model-value="row.stock_row_key"
                :disabled="!form.warehouse_id"
                :placeholder="t('sales.delivery.stockRowPlaceholder')"
                size="small"
                style="width: 100%"
                @update:model-value="(v: string) => onStockRowChange(row, v)"
              >
                <el-option
                  v-for="s in props.stockRows[row.product_id] || []"
                  :key="stockRowKey(s)"
                  :label="stockRowLabel(s)"
                  :value="stockRowKey(s)"
                />
              </el-select>
            </template>
          </el-table-column>
          <el-table-column prop="dye_lot_no" :label="t('sales.delivery.dyeLotNo')" width="120" />
          <el-table-column prop="color_no" :label="t('sales.delivery.colorNo')" width="100" />
          <el-table-column prop="batch_no" :label="t('sales.delivery.batchNo')" width="110" />
          <!-- 出库第四维（染色布=色号非空强制）：匹号只能从该缸该批现存 AVAILABLE 真实匹中选定；
               白坯不参与匹号维度，不渲染本控件 -->
          <el-table-column :label="t('sales.delivery.pieceNo')" width="180">
            <template #default="{ row }">
              <el-select
                v-if="row.color_no"
                :model-value="row.piece_no"
                size="small"
                style="width: 100%"
                :disabled="!row.batch_no || !row.dye_lot_no"
                :placeholder="t('sales.delivery.pieceNoPlaceholder')"
                @visible-change="(v: boolean) => v && emit('load-pieces', row)"
                @update:model-value="(v: string) => updateItem(row, 'piece_no', v)"
              >
                <el-option
                  v-for="p in props.pieceRows[deliveryPiecesKey(row)] || []"
                  :key="p.piece_no"
                  :label="pieceOptionLabel(p)"
                  :value="p.piece_no"
                />
              </el-select>
              <span v-else>—</span>
            </template>
          </el-table-column>
          <el-table-column prop="quantity" :label="t('sales.delivery.orderQuantity')" width="100" />
          <el-table-column
            prop="delivered_quantity"
            :label="t('sales.delivery.delivered')"
            width="100"
          />
          <el-table-column :label="t('sales.delivery.currentDelivery')" width="120">
            <template #default="{ row }">
              <el-input-number
                :model-value="row.deliver_quantity"
                :min="0"
                :max="row.quantity - (row.delivered_quantity || 0)"
                size="small"
                @update:model-value="(v: number) => updateItem(row, 'deliver_quantity', v)"
              />
            </template>
          </el-table-column>
          <el-table-column prop="unit_price" :label="t('sales.delivery.unitPrice')" width="100" />
          <el-table-column :label="t('sales.delivery.remark')" min-width="150">
            <template #default="{ row }">
              <el-input
                :model-value="row.remarks"
                size="small"
                :placeholder="t('sales.delivery.remarkPlaceholder')"
                @update:model-value="(v: string) => updateItem(row, 'remarks', v)"
              />
            </template>
          </el-table-column>
        </el-table>
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">{{ t('sales.delivery.cancel') }}</el-button>
      <el-button type="primary" :loading="submitting" @click="handleSubmit(form)">{{
        t('sales.delivery.confirmDelivery')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
// 注：form 由父组件通过 reactive() 创建并通过 prop 传入；
// 子组件在用户交互时通过 emit('update:form', newForm) 整体覆盖，避免直接修改 prop。
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import type { InventoryStock } from '@/api/inventory';
import type { InventoryPieceRow } from '@/api/inventory-transfer';
import { stockRowKey, deliveryPiecesKey, type DeliveryItemForm } from './composables/useOlv';

const { t } = useI18n({ useScope: 'global' });

interface DeliveryForm {
  order_id: number;
  order_no: string;
  customer_name: string;
  delivery_date: string;
  warehouse_id: number | undefined;
  items: DeliveryItemForm[];
}

const props = defineProps<{
  visible: boolean;
  form: DeliveryForm;
  warehouses: { id: number; warehouse_name?: string; warehouse_code?: string }[];
  /** 出库前三维候选库存行：key=product_id，由父组件按所选发货仓查询后端库存接口获得 */
  stockRows: Record<number, InventoryStock[]>;
  /** 出库第四维候选真实匹：key=deliveryPiecesKey(row)，由父组件按 产品+缸+批 下推查询获得 */
  pieceRows: Record<string, InventoryPieceRow[]>;
  submitting?: boolean;
}>();

const emit = defineEmits<{
  'update:visible': [value: boolean];
  'update:form': [value: DeliveryForm];
  'warehouse-change': [warehouseId: number];
  'load-pieces': [row: DeliveryItemForm];
  submit: [data: DeliveryForm];
}>();

// 通过 emit 通知父组件更新 form 字段（顶层字段）
const updateForm = <K extends keyof DeliveryForm>(key: K, value: DeliveryForm[K]) => {
  emit('update:form', { ...props.form, [key]: value });
};

// 通过 emit 通知父组件更新 items 数组中的指定行
const updateItem = <K extends keyof DeliveryItemForm>(
  row: DeliveryItemForm,
  key: K,
  value: DeliveryItemForm[K]
) => {
  // 创建新的 items 数组（不可变更新），避免直接修改 prop.items
  const newItems = props.form.items.map(item => (item === row ? { ...item, [key]: value } : item));
  emit('update:form', { ...props.form, items: newItems });
};

// 仓库变更：写回表单并清空已选库存行与匹号（换仓后旧仓的四维行不再有效）+ 通知父组件重新加载
const onWarehouseChange = (warehouseId: number) => {
  const resetItems = props.form.items.map(item => ({
    ...item,
    stock_row_key: '',
    color_no: '',
    batch_no: '',
    dye_lot_no: '',
    piece_no: '',
  }));
  emit('update:form', { ...props.form, warehouse_id: warehouseId, items: resetItems });
  emit('warehouse-change', warehouseId);
};

/** 库存行选择：一次选定色号+批次+缸号（出库四维中的三个文本维度） */
const onStockRowChange = (row: DeliveryItemForm, key: string) => {
  const stock = (props.stockRows[row.product_id] || []).find(s => stockRowKey(s) === key);
  if (!stock) {
    return;
  }
  const updated: DeliveryItemForm = {
    ...row,
    stock_row_key: key,
    color_no: stock.color_no ?? '',
    batch_no: stock.batch_no ?? '',
    dye_lot_no: stock.dye_lot_no ?? '',
    // 维度改选即旧匹号失效（匹按 产品+缸+批 归属），先清空再由父组件按新维度重取
    piece_no: '',
  };
  const newItems = props.form.items.map(item => (item === row ? updated : item));
  emit('update:form', { ...props.form, items: newItems });
  if (updated.color_no) {
    emit('load-pieces', updated);
  }
};

const stockRowLabel = (s: InventoryStock) =>
  `${t('sales.delivery.dyeLotNo')}: ${s.dye_lot_no || '-'} · ${t('sales.delivery.colorNo')}: ${s.color_no} · ${t('sales.delivery.batchNo')}: ${s.batch_no} · ${t('sales.delivery.availableQty')}: ${s.quantity_available}`;

/** 匹选项文案：匹号 + 该匹真实米数（后端 Decimal 串，展示前 Number() 归一，禁造数） */
const pieceOptionLabel = (p: InventoryPieceRow) =>
  `${t('sales.delivery.pieceNo')}: ${p.piece_no} · ${t('sales.delivery.pieceLength')}: ${Number(p.length)}`;

const handleSubmit = (form: DeliveryForm) => {
  // 校验：确保必填项已填
  if (!form.warehouse_id) {
    ElMessage.warning(t('sales.delivery.warehouseRequired'));
    return;
  }
  if (!form.delivery_date) {
    ElMessage.warning(t('sales.delivery.deliveryDateRequired'));
    return;
  }
  const deliveringItems = form.items.filter(i => i.deliver_quantity > 0);
  if (deliveringItems.length === 0) {
    ElMessage.warning(t('sales.delivery.atLeastOneDelivery'));
    return;
  }
  // 出库维度口径与后端同源（fabric_class 唯一判定，禁止第二份规则）：
  // 必须选定真实库存行 + 批次；染色布（色号非空）必须齐缸号与匹号，白坯布（色号为空）免缸号免匹号。
  const missingDim = deliveringItems.find(
    i => !i.stock_row_key || !i.batch_no || (i.color_no && !i.dye_lot_no)
  );
  if (missingDim) {
    ElMessage.warning(t('sales.delivery.stockRowRequired'));
    return;
  }
  const missingPiece = deliveringItems.find(i => i.color_no && !i.piece_no);
  if (missingPiece) {
    ElMessage.warning(t('sales.delivery.pieceNoRequired'));
    return;
  }
  emit('submit', form);
};
</script>
