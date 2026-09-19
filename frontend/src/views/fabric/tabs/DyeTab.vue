<!--
  DyeTab.vue - 染色批次 Tab
  来源：原 fabric/index.vue 中 染色批次 tab 内容
  拆分日期：2026-06-15 B3-4
-->
<template>
  <div class="dye-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('fabric.dyeTab.title') }}</h2>
      <el-button type="primary" @click="openCreate">
        <el-icon><Plus /></el-icon>
        {{ t('fabric.dyeTab.buttonCreate') }}
      </el-button>
    </div>

    <el-card shadow="hover">
      <el-table
        v-loading="loading"
        :data="batches"
        stripe
        :aria-label="t('fabric.dyeTab.tableAriaLabel')"
      >
        <el-table-column prop="batch_no" :label="t('fabric.dyeTab.columnBatchNo')" width="140" />
        <el-table-column prop="color_name" :label="t('fabric.dyeTab.columnColor')" width="120" />
        <el-table-column
          prop="greige_fabric_name"
          :label="t('fabric.dyeTab.columnGreige')"
          width="150"
        />
        <el-table-column
          prop="planned_quantity"
          :label="t('fabric.dyeTab.columnPlannedQuantity')"
          width="100"
          align="right"
        />
        <el-table-column
          prop="actual_quantity"
          :label="t('fabric.dyeTab.columnActualQuantity')"
          width="100"
          align="right"
        />
        <el-table-column
          prop="status"
          :label="t('fabric.dyeTab.columnStatus')"
          width="100"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)" size="small">
              {{ getStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="start_date"
          :label="t('fabric.dyeTab.columnStartDate')"
          width="120"
        />
        <el-table-column :label="t('fabric.dyeTab.columnAction')" width="200" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="openEdit(row)">{{
              t('fabric.dyeTab.buttonEdit')
            }}</el-button>
            <el-button
              v-if="
                [
                  'preparing',
                  'dyeing',
                  'washing',
                  'fixing',
                  'dehydrating',
                  'drying',
                  'inspecting',
                ].includes(row.status)
              "
              type="success"
              link
              size="small"
              @click="handleComplete(row)"
              >{{ t('fabric.dyeTab.buttonComplete') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, defineEmits } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import { completeDyeBatch, type DyeBatch } from '@/api/dye-batch';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

const emit = defineEmits<{ openDialog: [row: DyeBatch | null] }>();

const batches = ref<DyeBatch[]>([]);
const loading = ref(false);

const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    pending_schedule: 'info',
    scheduled: 'info',
    preparing: 'warning',
    dyeing: 'warning',
    washing: 'warning',
    fixing: 'warning',
    dehydrating: 'warning',
    drying: 'warning',
    inspecting: 'warning',
    stored: 'success',
    shipped: 'success',
    cancelled: 'danger',
    terminated: 'danger',
    rework: 'warning',
    on_hold: 'warning',
    failed: 'danger',
  };
  return map[status] || 'info';
};

const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    pending_schedule: t('fabric.dyeTab.statusPendingSchedule'),
    scheduled: t('fabric.dyeTab.statusScheduled'),
    preparing: t('fabric.dyeTab.statusPreparing'),
    dyeing: t('fabric.dyeTab.statusDyeing'),
    washing: t('fabric.dyeTab.statusWashing'),
    fixing: t('fabric.dyeTab.statusFixing'),
    dehydrating: t('fabric.dyeTab.statusDehydrating'),
    drying: t('fabric.dyeTab.statusDrying'),
    inspecting: t('fabric.dyeTab.statusInspecting'),
    stored: t('fabric.dyeTab.statusStored'),
    shipped: t('fabric.dyeTab.statusShipped'),
    cancelled: t('fabric.dyeTab.statusCancelled'),
    terminated: t('fabric.dyeTab.statusTerminated'),
    rework: t('fabric.dyeTab.statusRework'),
    on_hold: t('fabric.dyeTab.statusOnHold'),
    failed: t('fabric.dyeTab.statusFailed'),
  };
  return map[status] || status;
};

const fetchBatches = async () => {
  loading.value = true;
  try {
    const { getDyeBatchList } = await import('@/api/dye-batch');
    const res = await getDyeBatchList();
    // 响应形态兜底：数组或 { items } 分页包装（防 el-table r is not iterable 白屏）
    const _p = res.data as unknown;
    batches.value = Array.isArray(_p) ? _p : ((_p as { items?: DyeBatch[] })?.items ?? []);
  } catch (error) {
    const err = error as Error;
    logger.error(t('fabric.dyeTab.fetchFailed'), err.message);
  } finally {
    loading.value = false;
  }
};

const openCreate = () => emit('openDialog', null);
const openEdit = (row: DyeBatch) => emit('openDialog', row);

const handleComplete = async (row: DyeBatch) => {
  try {
    await ElMessageBox.confirm(
      t('fabric.dyeTab.confirmCompleteContent'),
      t('fabric.common.confirmTitle'),
      { type: 'info' }
    );
    await completeDyeBatch(row.id);
    ElMessage.success(t('fabric.common.success'));
    fetchBatches();
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error;
      ElMessage.error(err.message || t('fabric.common.failed'));
    }
  }
};

onMounted(() => fetchBatches());

defineExpose({ fetchBatches });
</script>
