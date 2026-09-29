<!--
  FabricStockTab.vue - 面料库存（批次+色号+仓库维度）
  列表查询（getStockFabricList）+ 面料入库（createStockFabric）
  数据源：/inventory/stock/fabric（api/inventory.ts 封装）
-->
<template>
  <div class="fabric-stock-tab">
    <div class="tab-toolbar">
      <el-button type="primary" @click="dialogVisible = true">
        {{ t('inventory.fabricStock.create') }}
      </el-button>
      <el-button :loading="loading" @click="fetchFabricStock">
        {{ t('common.refresh') }}
      </el-button>
    </div>

    <el-table v-loading="loading" :data="rows" border stripe>
      <el-table-column prop="id" label="ID" width="70" />
      <el-table-column prop="product_id" :label="t('inventory.reservation.product')" width="100" />
      <el-table-column
        prop="warehouse_id"
        :label="t('inventory.stockTab.colWarehouse')"
        width="100"
      />
      <el-table-column prop="batch_no" :label="t('inventory.stockTab.colBatchNo')" width="130" />
      <el-table-column prop="color_no" :label="t('inventory.fabricStock.colorNo')" width="110" />
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
      <el-table-column prop="created_at" :label="t('common.createTime')" width="170" />
    </el-table>

    <el-dialog v-model="dialogVisible" :title="t('inventory.fabricStock.create')" width="500px">
      <el-form ref="formRef" :model="form" :rules="rules" label-width="110px">
        <el-form-item :label="t('inventory.reservation.product')" prop="product_id">
          <el-input-number v-model="form.product_id" :min="1" />
        </el-form-item>
        <el-form-item :label="t('inventory.stockTab.colWarehouse')" prop="warehouse_id">
          <el-select v-model="form.warehouse_id" filterable>
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
          <el-input v-model="form.color_no" />
        </el-form-item>
        <el-form-item :label="t('inventory.fabricStock.grade')" prop="grade">
          <el-select v-model="form.grade">
            <el-option label="一等品" value="一等品" />
            <el-option label="二等品" value="二等品" />
            <el-option label="合格品" value="合格品" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('inventory.fabricStock.meters')" prop="quantity_meters">
          <el-input-number v-model="form.quantity_meters" :min="0" :precision="2" />
        </el-form-item>
        <el-form-item :label="t('inventory.fabricStock.kg')" prop="quantity_kg">
          <el-input-number v-model="form.quantity_kg" :min="0" :precision="2" />
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
import { getWarehouseList, type Warehouse } from '@/api/warehouse';

const { t } = useI18n({ useScope: 'global' });

const rows = ref<Record<string, unknown>[]>([]);
const warehouses = ref<Warehouse[]>([]);
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

const form = reactive({
  product_id: undefined as number | undefined,
  warehouse_id: undefined as number | undefined,
  batch_no: '',
  color_no: '',
  grade: '一等品',
  quantity_meters: 0,
  quantity_kg: 0,
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
};

const handleSubmit = async () => {
  const valid = await formRef.value?.validate();
  if (!valid || !form.product_id || !form.warehouse_id) return;
  submitting.value = true;
  try {
    // 表单字段与 CreateStockRequest 一致，显式构造强类型请求体
    await createStockFabric({
      product_id: form.product_id,
      warehouse_id: form.warehouse_id,
      batch_no: form.batch_no,
      color_no: form.color_no,
      grade: form.grade,
      quantity_meters: form.quantity_meters,
      quantity_kg: form.quantity_kg,
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

defineExpose({ refresh: fetchFabricStock });

onMounted(() => {
  fetchFabricStock();
  fetchWarehouses();
});
</script>

<style scoped>
.tab-toolbar {
  margin-bottom: 12px;
  display: flex;
  gap: 8px;
}
</style>
