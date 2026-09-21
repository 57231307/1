<!--
  InventoryReservationTab.vue - 库存预留管理
  预留列表 + 新建预留 + 锁定/释放/取消（5 函数全链）
  数据源：/inventory/reservations（api/inventory.ts 封装）
-->
<template>
  <div class="reservation-tab">
    <div class="tab-toolbar">
      <el-button type="primary" @click="dialogVisible = true">
        {{ t('inventory.reservation.create') }}
      </el-button>
      <el-button :loading="loading" @click="fetchReservations">
        {{ t('common.refresh') || '刷新' }}
      </el-button>
    </div>

    <el-table v-loading="loading" :data="reservations" border stripe>
      <el-table-column prop="id" label="ID" width="70" />
      <el-table-column prop="product_id" :label="t('inventory.reservation.product')" width="100" />
      <el-table-column
        prop="warehouse_id"
        :label="t('inventory.stockTab.colWarehouse')"
        width="100"
      />
      <el-table-column
        prop="quantity"
        :label="t('inventory.stockTab.colQuantity')"
        width="110"
        align="right"
      >
        <template #default="{ row }">{{ Number(row.quantity ?? 0).toLocaleString() }}</template>
      </el-table-column>
      <el-table-column prop="status" :label="t('common.status')" width="100">
        <template #default="{ row }">
          <el-tag size="small" :type="statusType(row.status)">{{ row.status }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="remark" :label="t('inventory.reservation.remark')" min-width="150" />
      <el-table-column prop="created_at" :label="t('common.createTime')" width="170" />
      <el-table-column :label="t('common.action')" width="220" fixed="right">
        <template #default="{ row }">
          <el-button
            v-if="row.status === 'pending'"
            size="small"
            type="warning"
            link
            @click="lockReservationRow(row)"
          >
            {{ t('inventory.reservation.lock') }}
          </el-button>
          <el-button
            v-if="row.status === 'locked'"
            size="small"
            type="success"
            link
            @click="releaseReservationRow(row)"
          >
            {{ t('inventory.reservation.release') }}
          </el-button>
          <el-button
            v-if="row.status === 'pending' || row.status === 'locked'"
            size="small"
            type="danger"
            link
            @click="cancelReservationRow(row)"
          >
            {{ t('common.cancel') }}
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- 新建预留对话框 -->
    <el-dialog v-model="dialogVisible" :title="t('inventory.reservation.create')" width="480px">
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
        <el-form-item :label="t('inventory.stockTab.colQuantity')" prop="quantity">
          <el-input-number v-model="form.quantity" :min="0.01" :precision="2" />
        </el-form-item>
        <el-form-item :label="t('inventory.reservation.remark')">
          <el-input v-model="form.remark" type="textarea" :rows="2" />
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
import { logger } from '@/utils/logger';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import {
  getReservationList,
  createReservation,
  cancelReservation,
  lockReservation,
  releaseReservation,
} from '@/api/inventory';
import { getWarehouseList, type Warehouse } from '@/api/warehouse';

const { t } = useI18n({ useScope: 'global' });

const reservations = ref<Record<string, unknown>[]>([]);
const warehouses = ref<Warehouse[]>([]);
const loading = ref(false);
const dialogVisible = ref(false);
const submitting = ref(false);
const formRef = ref<FormInstance>();

const statusType = (status: string) => {
  const map: Record<string, string> = {
    pending: 'warning',
    locked: 'info',
    released: 'success',
    cancelled: 'info',
    completed: 'success',
  };
  return map[status] || 'info';
};

const fetchReservations = async () => {
  loading.value = true;
  try {
    const res = await getReservationList({ page: 1, page_size: 100 });
    const d = res.data as unknown as
      { items?: Record<string, unknown>[] } | Record<string, unknown>[] | undefined;
    reservations.value =
      (d && typeof d === 'object' && !Array.isArray(d)
        ? d.items
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
  quantity: 0,
  remark: '',
});

const rules: FormRules = {
  product_id: [
    { required: true, message: t('inventory.reservation.productRequired'), trigger: 'change' },
  ],
  warehouse_id: [
    { required: true, message: t('inventory.reservation.warehouseRequired'), trigger: 'change' },
  ],
  quantity: [
    { required: true, message: t('inventory.reservation.quantityRequired'), trigger: 'blur' },
  ],
};

const handleSubmit = async () => {
  const valid = await formRef.value?.validate();
  if (!valid || !form.product_id || !form.warehouse_id) return;
  submitting.value = true;
  try {
    // 按 ReservationData DTO 提交：order_id/order_no 为必填占位，备注映射到 notes
    await createReservation({
      order_id: 0,
      order_no: '',
      product_id: form.product_id,
      warehouse_id: form.warehouse_id,
      quantity: form.quantity,
      notes: form.remark,
    });
    ElMessage.success(t('common.success'));
    dialogVisible.value = false;
    fetchReservations();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  } finally {
    submitting.value = false;
  }
};

const runAction = async (action: () => Promise<unknown>, msg: string) => {
  try {
    await action();
    ElMessage.success(msg);
    fetchReservations();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
  }
};

const lockReservationRow = (row: Record<string, unknown>) =>
  runAction(() => lockReservation(Number(row.id)), t('inventory.reservation.lockSuccess'));

const releaseReservationRow = (row: Record<string, unknown>) =>
  runAction(() => releaseReservation(Number(row.id)), t('inventory.reservation.releaseSuccess'));

const cancelReservationRow = async (row: Record<string, unknown>) => {
  try {
    await ElMessageBox.confirm(t('inventory.reservation.cancelConfirm'), t('common.cancel'), {
      type: 'warning',
    });
  } catch {
    return;
  }
  await runAction(
    () => cancelReservation(Number(row.id)),
    t('inventory.reservation.cancelSuccess')
  );
};

const fetchWarehouses = async () => {
  try {
    const res = await getWarehouseList({ page: 1, page_size: 1000 });
    warehouses.value = res.data?.items || [];
  } catch (error) {
    logger.error(t('inventory.reservation.loadWarehousesFailed'), error);
    warehouses.value = [];
  }
};

defineExpose({ refresh: fetchReservations });

onMounted(() => {
  fetchReservations();
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
