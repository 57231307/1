<!--
  ArReportTab.vue - 应收报表中心
  四类报表查询：统计汇总 / 日报 / 月报 / 账龄分析
  数据源：/ar/reports/*（api/ar.ts 封装）
-->
<template>
  <div class="ar-report-tab">
    <el-card shadow="never">
      <template #header>
        <div class="report-header">
          <el-radio-group v-model="reportType">
            <el-radio-button value="statistics">{{
              t('arModule.report.statistics')
            }}</el-radio-button>
            <el-radio-button value="daily">{{ t('arModule.report.daily') }}</el-radio-button>
            <el-radio-button value="monthly">{{ t('arModule.report.monthly') }}</el-radio-button>
            <el-radio-button value="aging">{{ t('arModule.report.aging') }}</el-radio-button>
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
      <el-empty v-else-if="!loading" :description="t('arModule.report.emptyTip')" />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import {
  getARStatisticsReport,
  getARDailyReport,
  getARMonthlyReport,
  getARAgingReport,
} from '@/api/ar';

const { t } = useI18n({ useScope: 'global' });

const reportType = ref<'statistics' | 'daily' | 'monthly' | 'aging'>('statistics');
const loading = ref(false);
const rows = ref<Record<string, unknown>[]>([]);

const columns = computed(() => (rows.value.length ? Object.keys(rows.value[0]) : []));

const fetchReport = async () => {
  loading.value = true;
  try {
    // 出参形状以定稿 DTO 为唯一事实（backend/src/services/ar_ops/report.rs）：
    // daily/monthly 为裸数组行集，statistics/aging 为单聚合对象（按一行展示）。
    if (reportType.value === 'daily') {
      const res = await getARDailyReport(new Date().toISOString().split('T')[0]);
      rows.value = res.data;
    } else if (reportType.value === 'monthly') {
      const now = new Date();
      const res = await getARMonthlyReport(now.getFullYear(), now.getMonth() + 1);
      rows.value = res.data;
    } else if (reportType.value === 'statistics') {
      const res = await getARStatisticsReport();
      rows.value = [res.data];
    } else {
      const res = await getARAgingReport();
      rows.value = [res.data];
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
