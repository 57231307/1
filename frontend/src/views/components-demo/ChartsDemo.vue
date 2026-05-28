<template>
  <div class="charts-demo">
    <el-row :gutter="20">
      <el-col :span="24">
        <el-card class="mb-20">
          <template #header>折线图示例</template>
          <LineChart
            :x-axis-data="lineXAxis"
            :series="lineSeries"
            title="月度销售趋势"
            :show-area="true"
            height="350px"
          />
        </el-card>
      </el-col>
    </el-row>

    <el-row :gutter="20">
      <el-col :span="12">
        <el-card class="mb-20">
          <template #header>柱状图示例</template>
          <BarChart
            :x-axis-data="barXAxis"
            :series="barSeries"
            title="各部门业绩对比"
            height="350px"
          />
        </el-card>
      </el-col>
      <el-col :span="12">
        <el-card class="mb-20">
          <template #header>饼图示例</template>
          <PieChart :data="pieData" title="产品销售占比" height="350px" />
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { LineChart, BarChart, PieChart } from '@/components/Charts'

const lineXAxis = [
  '1月',
  '2月',
  '3月',
  '4月',
  '5月',
  '6月',
  '7月',
  '8月',
  '9月',
  '10月',
  '11月',
  '12月',
]
const lineSeries = [
  {
    name: '销售额',
    data: [120, 132, 101, 134, 90, 230, 210, 180, 200, 250, 280, 300],
  },
  {
    name: '利润',
    data: [60, 72, 51, 74, 40, 130, 110, 90, 100, 125, 140, 150],
  },
]

const barXAxis = ['销售部', '市场部', '技术部', '客服部', '财务部']
const barSeries = [
  {
    name: 'Q1',
    data: [320, 180, 250, 150, 200],
  },
  {
    name: 'Q2',
    data: [380, 220, 280, 170, 230],
  },
]

const pieData = [
  { name: '面料A', value: 335 },
  { name: '面料B', value: 310 },
  { name: '面料C', value: 274 },
  { name: '面料D', value: 235 },
  { name: '面料E', value: 400 },
]
</script>

<style scoped>
.charts-demo {
  padding: 10px;
}

.mb-20 {
  margin-bottom: 20px;
}
</style>
