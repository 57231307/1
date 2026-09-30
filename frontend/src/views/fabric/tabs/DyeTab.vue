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
        <!-- 完工登记实际产出三列（真实列名对齐后端 models/dye_batch.rs；Decimal 字符串
             Number() 归一后格式化，NULL 显 '-'，不做兜底掩盖） -->
        <el-table-column
          prop="actual_output_kg"
          :label="t('fabric.dyeTab.columnActualOutputKg')"
          width="130"
          align="right"
        >
          <template #default="{ row }">{{ formatDecimal(row.actual_output_kg) }}</template>
        </el-table-column>
        <el-table-column
          prop="actual_output_m"
          :label="t('fabric.dyeTab.columnActualOutputM')"
          width="130"
          align="right"
        >
          <template #default="{ row }">{{ formatDecimal(row.actual_output_m) }}</template>
        </el-table-column>
        <el-table-column
          prop="greige_input_kg"
          :label="t('fabric.dyeTab.columnGreigeInputKg')"
          width="130"
          align="right"
        >
          <template #default="{ row }">{{ formatDecimal(row.greige_input_kg) }}</template>
        </el-table-column>
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
        <el-table-column prop="started_at" :label="t('fabric.dyeTab.columnStartDate')" width="120">
          <template #default="{ row }">{{
            row.started_at ? row.started_at.slice(0, 10) : '-'
          }}</template>
        </el-table-column>
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

    <!-- 完工登记对话框：强制采集实际产出三值（后端 CompleteDyeBatchRequest 必填契约） -->
    <CompleteDyeBatchDialog
      v-model="completeDialogVisible"
      :batch="completingBatch"
      @success="fetchBatches"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, defineEmits } from 'vue';
import { useI18n } from 'vue-i18n';
import { Plus } from '@element-plus/icons-vue';
import type { DyeBatch } from '@/api/dye-batch';
import CompleteDyeBatchDialog from '@/components/CompleteDyeBatchDialog.vue';
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
    // 后端返回 PaginatedResponse ⇒ data.items 为唯一形状（不再双形状宽容）
    batches.value = res.data.items;
  } catch (error) {
    const err = error as Error;
    logger.error(t('fabric.dyeTab.fetchFailed'), err.message);
  } finally {
    loading.value = false;
  }
};

const openCreate = () => emit('openDialog', null);
const openEdit = (row: DyeBatch) => emit('openDialog', row);

// 完成：后端 complete 端点强制采集实际产出三值（CompleteDyeBatchRequest），
// 原"确认框即提交"形态已废弃——无产出登记的完工会让成本/能耗分母重新断链。
const completeDialogVisible = ref(false);
const completingBatch = ref<DyeBatch | null>(null);

const handleComplete = (row: DyeBatch) => {
  completingBatch.value = row;
  completeDialogVisible.value = true;
};

// Decimal 出参为字符串（rust_decimal serde），Number() 归一后按列精度(12,2)格式化；
// null（未完工/历史行）显 '-'，不做假值兜底
const formatDecimal = (value: string | null | undefined): string =>
  value === null || value === undefined ? '-' : Number(value).toFixed(2);

onMounted(() => fetchBatches());

defineExpose({ fetchBatches });
</script>
