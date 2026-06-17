<!--
  AdvancedAiTab.vue - 高级 AI 分析 Tab
  来源：原 advanced/index.vue 中 ai tab
  拆分日期：2026-06-17 P1-3-Batch-6
-->
<template>
  <div>
    <div class="page-header">
      <h2 class="page-title">AI 智能分析</h2>
    </div>

    <el-row :gutter="20">
      <el-col :span="12">
        <el-card shadow="hover" class="mb-20">
          <template #header>
            <div class="card-header">销售预测</div>
          </template>
          <el-form label-width="100px">
            <el-form-item label="预测周期">
              <el-select v-model="forecastPeriod" style="width: 100%">
                <el-option label="未来 3 个月" value="3m" />
                <el-option label="未来 6 个月" value="6m" />
                <el-option label="未来 12 个月" value="12m" />
              </el-select>
            </el-form-item>
            <el-form-item>
              <el-button type="primary" :loading="forecastLoading" @click="emit('run-forecast')"
                >开始预测</el-button
              >
            </el-form-item>
          </el-form>
          <el-empty v-if="!forecastResult" description="点击开始预测" />
          <div v-else>
            <h4>预测结果</h4>
            <el-divider />
            <el-descriptions :column="2" border>
              <el-descriptions-item label="预测销售额">{{
                formatMoney(forecastResult.sales_amount)
              }}</el-descriptions-item>
              <el-descriptions-item label="预测订单数">{{
                forecastResult.order_count
              }}</el-descriptions-item>
              <el-descriptions-item label="置信度"
                >{{ forecastResult.confidence }}%</el-descriptions-item
              >
              <el-descriptions-item label="预测趋势">{{
                forecastResult.trend
              }}</el-descriptions-item>
            </el-descriptions>
          </div>
        </el-card>

        <el-card shadow="hover">
          <template #header>
            <div class="card-header">库存优化建议</div>
          </template>
          <el-button type="primary" :loading="optimizeLoading" @click="emit('optimize-inventory')"
            >生成优化方案</el-button
          >
          <div v-if="optimizeResult" class="mt-20">
            <h4>优化建议</h4>
            <el-divider />
            <el-alert
              v-for="(item, idx) in optimizeResult.suggestions"
              :key="idx"
              :title="item.title"
              :type="item.level"
              :description="item.description"
              show-icon
              :closable="false"
              class="mb-10"
            />
          </div>
        </el-card>
      </el-col>

      <el-col :span="12">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">客户画像</div>
          </template>
          <el-button type="primary" :loading="profileLoading" @click="emit('run-profile')"
            >生成客户画像</el-button
          >
          <div v-if="profileResult" class="mt-20">
            <el-descriptions :column="1" border>
              <el-descriptions-item label="客户总数">{{
                profileResult.total_customers
              }}</el-descriptions-item>
              <el-descriptions-item label="VIP 客户">{{
                profileResult.vip_count
              }}</el-descriptions-item>
              <el-descriptions-item label="活跃客户">{{
                profileResult.active_count
              }}</el-descriptions-item>
              <el-descriptions-item label="流失风险">{{
                profileResult.churn_risk
              }}</el-descriptions-item>
            </el-descriptions>
          </div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  forecastPeriod: string
  forecastLoading: boolean
  forecastResult: any
  optimizeLoading: boolean
  optimizeResult: any
  profileLoading: boolean
  profileResult: any
  formatMoney: (amount: number) => string
}>()

defineEmits<{
  'run-forecast': []
  'optimize-inventory': []
  'run-profile': []
}>()
</script>
