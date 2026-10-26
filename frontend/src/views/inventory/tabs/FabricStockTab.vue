<!--
  FabricStockTab.vue - 面料库存（批次+色号+仓库维度）
  列表查询（getStockFabricList）+ 面料入库（createStockFabric）
  数据源：/inventory/stock/fabric（api/inventory.ts 封装）
  入库受理侧必填口径与后端 inbound_admission 同源：数量米>0、等级取库存等级词表、
  公斤数实称优先缺失时按克重与幅宽换算两条都缺即拒、来源单号必填；界面按此录入并前置校验，
  不做静默兜底。数值录入一律用 el-input + 校验器，禁步进数字框。
-->
<template>
  <div class="fabric-stock-tab">
    <div class="tab-toolbar">
      <el-button type="primary" @click="openCreate">
        {{ t('inventory.fabricStock.create') }}
      </el-button>
      <el-button :loading="loading" @click="fetchFabricStock">
        {{ t('common.refresh') }}
      </el-button>
    </div>

    <el-table v-loading="loading" :data="rows" border stripe>
      <el-table-column prop="id" label="ID" width="70" />
      <el-table-column prop="product_id" :label="t('inventory.fabricStock.product')" width="100" />
      <el-table-column
        prop="warehouse_id"
        :label="t('inventory.stockTab.colWarehouse')"
        width="100"
      />
      <el-table-column prop="batch_no" :label="t('inventory.stockTab.colBatchNo')" width="130" />
      <el-table-column prop="color_no" :label="t('inventory.fabricStock.colorNo')" width="110" />
      <el-table-column prop="dye_lot_no" :label="t('inventory.fabricStock.dyeLot')" width="110" />
      <el-table-column prop="grade" :label="t('inventory.fabricStock.grade')" width="90" />
      <el-table-column
        prop="quantity_meters"
        :label="t('inventory.fabricStock.meters')"
        width="110"
        align="right"
      >
        <template #default="{ row }">{{
          Number(row.quantity_meters ?? 0).toLocaleString()
        }}</template>
      </el-table-column>
      <el-table-column
        prop="quantity_kg"
        :label="t('inventory.fabricStock.kg')"
        width="110"
        align="right"
      >
        <template #default="{ row }">{{ Number(row.quantity_kg ?? 0).toLocaleString() }}</template>
      </el-table-column>
      <el-table-column
        prop="gram_weight"
        :label="t('inventory.fabricStock.gramWeight')"
        width="120"
        align="right"
      >
        <template #default="{ row }">{{ row.gram_weight ?? '-' }}</template>
      </el-table-column>
      <el-table-column
        prop="width"
        :label="t('inventory.fabricStock.width')"
        width="110"
        align="right"
      >
        <template #default="{ row }">{{ row.width ?? '-' }}</template>
      </el-table-column>
      <el-table-column :label="t('inventory.stockTab.colLocation')" width="120">
        <template #default="{ row }">{{
          locationLabel(row.location_id) +
          (row.shelf_no ? ' / ' + row.shelf_no : '') +
          (row.layer_no ? ' / ' + row.layer_no : '')
        }}</template>
      </el-table-column>
      <el-table-column :label="t('inventory.stockTab.colStatus')" width="90">
        <template #default="{ row }">{{ row.stock_status }}</template>
      </el-table-column>
      <el-table-column :label="t('inventory.stockTab.colQualityStatus')" width="90">
        <template #default="{ row }">{{ row.quality_status }}</template>
      </el-table-column>
      <el-table-column prop="created_at" :label="t('common.createTime')" width="170" />
    </el-table>

    <el-dialog v-model="dialogVisible" :title="t('inventory.fabricStock.create')" width="640px">
      <el-form ref="formRef" :model="form" :rules="rules" label-width="120px">
        <el-form-item :label="t('inventory.fabricStock.product')" prop="product_id">
          <el-select v-model="form.product_id" filterable style="width: 100%">
            <el-option
              v-for="p in products"
              :key="p.id"
              :label="`${p.product_code} - ${p.product_name}`"
              :value="p.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('inventory.stockTab.colWarehouse')" prop="warehouse_id">
          <el-select
            v-model="form.warehouse_id"
            filterable
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
        <el-form-item :label="t('inventory.stockTab.colBatchNo')" prop="batch_no">
          <el-input v-model="form.batch_no" />
        </el-form-item>
        <el-form-item :label="t('inventory.fabricStock.colorNo')" prop="color_no">
          <el-input v-model="form.color_no" :placeholder="t('inventory.fabricStock.colorNo')" />
        </el-form-item>
        <el-form-item :label="t('inventory.fabricStock.dyeLot')" prop="dye_lot_no">
          <el-input v-model="form.dye_lot_no" />
        </el-form-item>
        <el-form-item :label="t('inventory.fabricStock.grade')" prop="grade">
          <el-select v-model="form.grade" style="width: 100%">
            <el-option
              v-for="g in STOCK_GRADE_VALUES"
              :key="g"
              :label="t(STOCK_GRADE_LABEL_KEY[g])"
              :value="g"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('inventory.fabricStock.meters')" prop="quantity_meters">
          <el-input v-model="form.quantity_meters" />
        </el-form-item>
        <el-form-item :label="t('inventory.fabricStock.kg')" prop="quantity_kg">
          <el-input
            v-model="form.quantity_kg"
            :placeholder="t('inventory.fabricStock.kgOrMeasure')"
          />
        </el-form-item>
        <el-form-item :label="t('inventory.fabricStock.gramWeight')" prop="gram_weight">
          <el-input v-model="form.gram_weight" />
        </el-form-item>
        <el-form-item :label="t('inventory.fabricStock.width')" prop="width">
          <el-input v-model="form.width" />
        </el-form-item>
        <el-form-item :label="t('inventory.stockTab.colLocation')" prop="location_id">
          <el-select
            v-model="form.location_id"
            clearable
            filterable
            :disabled="!form.warehouse_id"
            style="width: 100%"
          >
            <el-option
              v-for="loc in locations"
              :key="loc.id"
              :label="loc.location_code"
              :value="loc.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('inventory.fabricStock.shelfNo')" prop="shelf_no">
          <el-input v-model="form.shelf_no" />
        </el-form-item>
        <el-form-item :label="t('inventory.fabricStock.layerNo')" prop="layer_no">
          <el-input v-model="form.layer_no" />
        </el-form-item>
        <el-form-item :label="t('inventory.fabricStock.sourceBillNo')" prop="source_bill_no">
          <el-input v-model="form.source_bill_no" />
        </el-form-item>
        <el-form-item :label="t('inventory.fabricStock.notes')" prop="notes">
          <el-input
            v-model="form.notes"
            type="textarea"
            :rows="2"
            maxlength="500"
            show-word-limit
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="submitting" @click="handleSubmit">
          {{ t('common.save') }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue';
import { logAuxLoadFailure } from '@/utils/logger';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import { getStockFabricList, createStockFabric } from '@/api/inventory';
import {
  getWarehouseList,
  getWarehouseLocationList,
  type Warehouse,
  type WarehouseLocation,
} from '@/api/warehouse';
import { getProductList, type Product } from '@/api/product';
import { STOCK_GRADE_VALUES, STOCK_GRADE_LABEL_KEY } from '@/constants/stock-grade';

const { t } = useI18n({ useScope: 'global' });

const rows = ref<Record<string, unknown>[]>([]);
const warehouses = ref<Warehouse[]>([]);
const locations = ref<WarehouseLocation[]>([]);
const products = ref<Product[]>([]);
const loading = ref(false);
const dialogVisible = ref(false);
const submitting = ref(false);
const formRef = ref<FormInstance>();

const fetchFabricStock = async () => {
  loading.value = true;
  try {
    const res = await getStockFabricList({ page: 1, page_size: 100 });
    const d = res.data as unknown as
      | { items?: Record<string, unknown>[]; list?: Record<string, unknown>[] }
      | Record<string, unknown>[]
      | undefined;
    rows.value =
      (d && typeof d === 'object' && !Array.isArray(d)
        ? d.items || d.list
        : (d as Record<string, unknown>[])) || [];
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  } finally {
    loading.value = false;
  }
};

/** 库位 ID → 编码标签（列表展示用，未加载到对应库位时回落到 ID，绝不留假空） */
const locationLabel = (locationId: unknown): string => {
  if (locationId == null) return '-';
  const hit = locations.value.find(l => l.id === locationId);
  return hit ? hit.location_code : String(locationId);
};

// 十进制串校验器：留空放行（可选语义交给业务合并校验），填了必须是有限正数或大于零的数
const positiveDecimalRule = (label: string, allowEmpty: boolean) => ({
  validator: (_rule: unknown, value: string, callback: (err?: Error) => void) => {
    const raw = (value ?? '').trim();
    if (raw === '') {
      if (allowEmpty) {
        callback();
        return;
      }
      callback(new Error(label));
      return;
    }
    const n = Number(raw);
    if (!Number.isFinite(n) || n <= 0) {
      callback(new Error(t('inventory.fabricStock.measurePositive')));
      return;
    }
    callback();
  },
});

const form = reactive({
  product_id: undefined as number | undefined,
  warehouse_id: undefined as number | undefined,
  batch_no: '',
  color_no: '',
  dye_lot_no: '',
  // 等级空起步：直建成品布入库即时写真库存行，grade 即验布定级结论，
  // 预置任一档（哪怕一等品）都等于把未定级伪装成质检结论，必须让用户显式选。
  grade: '' as string,
  quantity_meters: '',
  quantity_kg: '',
  gram_weight: '',
  width: '',
  location_id: undefined as number | undefined,
  shelf_no: '',
  layer_no: '',
  source_bill_no: '',
  notes: '',
});

const rules: FormRules = {
  product_id: [
    { required: true, message: t('inventory.reservation.productRequired'), trigger: 'change' },
  ],
  warehouse_id: [
    { required: true, message: t('inventory.reservation.warehouseRequired'), trigger: 'change' },
  ],
  batch_no: [
    { required: true, message: t('inventory.fabricStock.batchRequired'), trigger: 'blur' },
  ],
  grade: [{ required: true, message: t('inventory.fabricStock.gradeRequired'), trigger: 'change' }],
  quantity_meters: [
    {
      validator: (_rule: unknown, value: string, callback: (err?: Error) => void) => {
        const raw = (value ?? '').trim();
        if (raw === '') {
          callback(new Error(t('inventory.fabricStock.metersPositive')));
          return;
        }
        const n = Number(raw);
        if (!Number.isFinite(n) || n <= 0) {
          callback(new Error(t('inventory.fabricStock.metersPositive')));
          return;
        }
        callback();
      },
      trigger: 'blur',
    },
  ],
  // 公斤数与克重幅宽是「实称 / 推算」二选一，合并校验在提交前做（见 handleSubmit），
  // 单字段规则只保证「填了就必须是正数」，避免留空即误报
  quantity_kg: [positiveDecimalRule(t('inventory.fabricStock.kgOrMeasure'), true)],
  gram_weight: [positiveDecimalRule(t('inventory.fabricStock.measurePositive'), true)],
  width: [positiveDecimalRule(t('inventory.fabricStock.measurePositive'), true)],
  source_bill_no: [
    {
      required: true,
      message: t('inventory.fabricStock.sourceBillRequired'),
      trigger: 'blur',
    },
  ],
};

const openCreate = () => {
  dialogVisible.value = true;
};

const onWarehouseChange = async (warehouseId?: number) => {
  form.location_id = undefined;
  locations.value = [];
  if (!warehouseId) return;
  try {
    const res = await getWarehouseLocationList(warehouseId);
    locations.value = res.data?.items || [];
  } catch (error) {
    logAuxLoadFailure(t('inventory.fabricStock.loadWarehousesFailed'), error);
    locations.value = [];
  }
};

/** 公斤数能否确定的合并判定：实称>0，或克重与幅宽同时>0（与后端 resolve_inbound_quantity_kg 同口径） */
const canResolveKg = (): boolean => {
  const kg = Number((form.quantity_kg ?? '').trim());
  if (Number.isFinite(kg) && kg > 0) return true;
  const gram = Number((form.gram_weight ?? '').trim());
  const width = Number((form.width ?? '').trim());
  return Number.isFinite(gram) && gram > 0 && Number.isFinite(width) && width > 0;
};

// 可选字段：留空省略键（不送空串占位，后端 Option 缺席即不写）
const optionalStr = (v: string): string | undefined => {
  const s = (v ?? '').trim();
  return s === '' ? undefined : s;
};

const handleSubmit = async () => {
  const valid = await formRef.value?.validate().catch(() => false);
  if (!valid || !form.product_id || !form.warehouse_id) return;
  if (!canResolveKg()) {
    ElMessage.warning(t('inventory.fabricStock.kgOrMeasure'));
    return;
  }
  submitting.value = true;
  try {
    // 表单字段与 CreateStockRequest 对齐，显式构造强类型请求体（数量传十进制串保精度）
    await createStockFabric({
      product_id: form.product_id,
      warehouse_id: form.warehouse_id,
      batch_no: form.batch_no.trim(),
      color_no: form.color_no.trim(),
      dye_lot_no: optionalStr(form.dye_lot_no),
      grade: form.grade,
      quantity_meters: form.quantity_meters.trim(),
      quantity_kg: optionalStr(form.quantity_kg),
      gram_weight: optionalStr(form.gram_weight),
      width: optionalStr(form.width),
      location_id: form.location_id,
      shelf_no: optionalStr(form.shelf_no),
      layer_no: optionalStr(form.layer_no),
      source_bill_no: form.source_bill_no.trim(),
      notes: optionalStr(form.notes),
    });
    ElMessage.success(t('common.success'));
    dialogVisible.value = false;
    fetchFabricStock();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  } finally {
    submitting.value = false;
  }
};

const fetchWarehouses = async () => {
  try {
    const res = await getWarehouseList({ page: 1, page_size: 1000 });
    warehouses.value = res.data?.items || [];
  } catch (error) {
    logAuxLoadFailure(t('inventory.fabricStock.loadWarehousesFailed'), error);
    warehouses.value = [];
  }
};

const fetchProducts = async () => {
  try {
    const res = await getProductList({ page: 1, page_size: 1000 });
    products.value = res.data?.items || [];
  } catch (error) {
    logAuxLoadFailure(t('inventory.fabricStock.product'), error);
    products.value = [];
  }
};

defineExpose({ refresh: fetchFabricStock });

onMounted(() => {
  fetchFabricStock();
  fetchWarehouses();
  fetchProducts();
});
</script>

<style scoped>
.tab-toolbar {
  margin-bottom: 12px;
  display: flex;
  gap: 8px;
}
</style>
