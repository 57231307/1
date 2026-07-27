<template>
  <div class="charts-demo">
    <el-row :gutter="20">
      <el-col :span="24">
        <el-card class="mb-20">
          <template #header>{{ t('componentsDemo.charts.lineExample') }}</template>
          <LineChart
            :x-axis-data="lineXAxis"
            :series="lineSeries"
            :title="t('componentsDemo.charts.lineTitle')"
            :show-area="true"
            height="350px"
          />
        </el-card>
      </el-col>
    </el-row>

    <el-row :gutter="20">
      <el-col :span="12">
        <el-card class="mb-20">
          <template #header>{{ t('componentsDemo.charts.barExample') }}</template>
          <BarChart
            :x-axis-data="barXAxis"
            :series="barSeries"
            :title="t('componentsDemo.charts.barTitle')"
            height="350px"
          />
        </el-card>
      </el-col>
      <el-col :span="12">
        <el-card class="mb-20">
          <template #header>{{ t('componentsDemo.charts.pieExample') }}</template>
          <PieChart :data="pieData" :title="t('componentsDemo.charts.pieTitle')" height="350px" />
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { LineChart, BarChart, PieChart } from '@/components/Charts';

const { t } = useI18n({ useScope: 'global' });

const lineXAxis = [
  t('componentsDemo.charts.months.jan'),
  t('componentsDemo.charts.months.feb'),
  t('componentsDemo.charts.months.mar'),
  t('componentsDemo.charts.months.apr'),
  t('componentsDemo.charts.months.may'),
  t('componentsDemo.charts.months.jun'),
  t('componentsDemo.charts.months.jul'),
  t('componentsDemo.charts.months.aug'),
  t('componentsDemo.charts.months.sep'),
  t('componentsDemo.charts.months.oct'),
  t('componentsDemo.charts.months.nov'),
  t('componentsDemo.charts.months.dec'),
];
const lineSeries = [
  {
    name: t('componentsDemo.charts.lineSeries.sales'),
    data: [120, 132, 101, 134, 90, 230, 210, 180, 200, 250, 280, 300],
  },
  {
    name: t('componentsDemo.charts.lineSeries.profit'),
    data: [60, 72, 51, 74, 40, 130, 110, 90, 100, 125, 140, 150],
  },
];

const barXAxis = [
  t('componentsDemo.charts.depts.sales'),
  t('componentsDemo.charts.depts.marketing'),
  t('componentsDemo.charts.depts.tech'),
  t('componentsDemo.charts.depts.service'),
  t('componentsDemo.charts.depts.finance'),
];
const barSeries = [
  {
    name: 'Q1',
    data: [320, 180, 250, 150, 200],
  },
  {
    name: 'Q2',
    data: [380, 220, 280, 170, 230],
  },
];

const pieData = [
  { name: t('componentsDemo.charts.fabrics.a'), value: 335 },
  { name: t('componentsDemo.charts.fabrics.b'), value: 310 },
  { name: t('componentsDemo.charts.fabrics.c'), value: 274 },
  { name: t('componentsDemo.charts.fabrics.d'), value: 235 },
  { name: t('componentsDemo.charts.fabrics.e'), value: 400 },
];
</script>

<style scoped>
.charts-demo {
  padding: 10px;
}

.mb-20 {
  margin-bottom: 20px;
}
</style>
