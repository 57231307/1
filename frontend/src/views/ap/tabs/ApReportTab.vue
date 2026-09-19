<!--
  ApReportTab.vue - 应付报表中心
  四类报表查询：统计汇总 / 日报 / 月报 / 账龄分析
  数据源：/ap/reports/*（api/ap.ts 封装）
-->
<template>
  <div class="ap-report-tab">
    <el-card shadow="never">
      <template #header>
        <div class="report-header">
          <el-radio-group v-model="reportType">
            <el-radio-button value="statistics">{{
              t('apModule.report.statistics')
            }}</el-radio-button>
            <el-radio-button value="daily">{{ t('apModule.report.daily') }}</el-radio-button>
            <el-radio-button value="monthly">{{ t('apModule.report.monthly') }}</el-radio-button>
            <el-radio-button value="aging">{{ t('apModule.report.aging') }}</el-radio-button>
          </el-radio-group>
          <el-button type="primary" :loading="loading" @click="fetchReport">
            {{ t('common.search') }}
          </el-button>
        </div>
      </template>

      <el-table v-if="rows.length" :data="rows" border stripe max-height="480">
        <el-table-column
          v-for="col in columns"
          :key="col"
          :prop="col"
          :label="col"
          min-width="120"
          align="right"
        >
          <template #default="{ row }">
            {{
              typeof row[col] === 'number'
                ? row[col].toLocaleString('zh-CN', { maximumFractionDigits: 2 })
                : (row[col] ?? '-')
            }}
          </template>
        </el-table-column>
      </el-table>
      <el-empty v-else-if="!loading" :description="t('apModule.report.emptyTip')" />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import {
  getAPStatisticsReport,
  getAPDailyReport,
  getAPMonthlyReport,
  getAPAgingReport,
} from '@/api/ap';

const { t } = useI18n({ useScope: 'global' });

const reportType = ref<'statistics' | 'daily' | 'monthly' | 'aging'>('statistics');
const loading = ref(false);
const rows = ref<Record<string, unknown>[]>([]);

const columns = computed(() => (rows.value.length ? Object.keys(rows.value[0]) : []));

const fetchers = {
  statistics: getAPStatisticsReport,
  daily: getAPDailyReport,
  monthly: getAPMonthlyReport,
  aging: getAPAgingReport,
};

const fetchReport = async () => {
  loading.value = true;
  try {
    const res =
      reportType.value === 'daily'
        ? await getAPDailyReport(new Date().toISOString().split('T')[0])
        : reportType.value === 'monthly'
          ? await getAPMonthlyReport(new Date().getFullYear(), new Date().getMonth() + 1)
          : await fetchers[reportType.value]();
    const d = res.data as unknown as
      | { list?: Record<string, unknown>[]; items?: Record<string, unknown>[] }
      | Record<string, unknown>[]
      | undefined;
    if (d && typeof d === 'object' && !Array.isArray(d)) {
      rows.value = d.list || d.items || [];
    } else {
      rows.value = (d as Record<string, unknown>[]) || [];
    }
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('common.failed'));
    rows.value = [];
  } finally {
    loading.value = false;
  }
};

defineExpose({ refresh: fetchReport });
</script>

<style scoped>
.report-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
