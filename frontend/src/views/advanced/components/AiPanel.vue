<script setup lang="ts">
/**
 * AiPanel - AI 智能分析 tab 视图组件（纯展示）
 * 任务编号: P13 批 1 B3 I-1（拆分 advanced/index.vue 第 1 个 tab）
 * 包含：销售预测、库存优化建议、异常检测、智能推荐
 * 数据与函数全部由父组件通过 props 传入
 */
interface Props {
  forecastPeriod: string
  forecastLoading: boolean
  forecastResult: any
  runSalesForecast: () => Promise<void>
  inventoryLoading: boolean
  inventoryResult: any
  runInventoryOptimization: () => Promise<void>
  anomalyType: string
  anomalyLoading: boolean
  anomalyResult: any
  runAnomalyDetection: () => Promise<void>
  recommendLoading: boolean
  recommendationResult: any
  getRecommendations: () => Promise<void>
  formatMoney: (amount: number) => string
}

defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:forecastPeriod', value: string): void
  (e: 'update:anomalyType', value: string): void
}>()
</script>

<template>
  <div class="page-header">
    <h2 class="page-title">AI 智能分析</h2>
  </div>

  <el-row :gutter="20">
    <el-col :span="12">
      <el-card shadow="hover" class="mb-20">
        <template #header><div class="card-header">销售预测</div></template>
        <el-form label-width="100px">
          <el-form-item label="预测周期">
            <el-select v-model="forecastPeriod" style="width: 100%">
              <el-option label="未来 3 个月" value="3m" />
              <el-option label="未来 6 个月" value="6m" />
              <el-option label="未来 12 个月" value="12m" />
            </el-select>
          </el-form-item>
          <el-form-item>
            <el-button type="primary" :loading="forecastLoading" @click="runSalesForecast"
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
        <template #header><div class="card-header">库存优化建议</div></template>
        <el-button
          type="primary"
          :loading="inventoryLoading"
          @click="runInventoryOptimization"
          >生成建议</el-button
        >
        <el-divider />
        <el-empty v-if="!inventoryResult" description="点击生成优化建议" />
        <div v-else>
          <el-alert type="success" :title="inventoryResult.summary" show-icon class="mb-10" />
          <el-table :data="inventoryResult.items" stripe>
            <el-table-column prop="product_name" label="产品" width="150" />
            <el-table-column prop="suggestion" label="建议" min-width="200" />
            <el-table-column prop="priority" label="优先级" width="100">
              <template #default="{ row }">
                <el-tag
                  :type="
                    row.priority === 'high'
                      ? 'danger'
                      : row.priority === 'medium'
                        ? 'warning'
                        : 'info'
                  "
                  size="small"
                >
                  {{
                    row.priority === 'high' ? '高' : row.priority === 'medium' ? '中' : '低'
                  }}
                </el-tag>
              </template>
            </el-table-column>
          </el-table>
        </div>
      </el-card>
    </el-col>

    <el-col :span="12">
      <el-card shadow="hover" class="mb-20">
        <template #header><div class="card-header">异常检测</div></template>
        <el-form label-width="100px">
          <el-form-item label="数据类型">
            <el-select
              :model-value="anomalyType"
              style="width: 100%"
              @update:model-value="(v: string) => emit('update:anomalyType', v)"
            >
              <el-option label="销售数据" value="sales" />
              <el-option label="库存数据" value="inventory" />
              <el-option label="质量数据" value="quality" />
            </el-select>
          </el-form-item>
          <el-form-item>
            <el-button type="primary" :loading="anomalyLoading" @click="runAnomalyDetection"
              >检测异常</el-button
            >
          </el-form-item>
        </el-form>
        <el-empty v-if="!anomalyResult" description="点击开始检测" />
        <div v-else>
          <el-table :data="anomalyResult" stripe>
            <el-table-column prop="item" label="检测项" width="150" />
            <el-table-column prop="type" label="类型" width="100">
              <template #default="{ row }">
                <el-tag
                  :type="row.severity === 'critical' ? 'danger' : 'warning'"
                  size="small"
                  >{{ row.type }}</el-tag
                >
              </template>
            </el-table-column>
            <el-table-column prop="description" label="描述" min-width="200" />
            <el-table-column prop="severity" label="严重程度" width="100" />
          </el-table>
        </div>
      </el-card>

      <el-card shadow="hover">
        <template #header><div class="card-header">智能推荐</div></template>
        <el-button type="primary" :loading="recommendLoading" @click="getRecommendations"
          >获取推荐</el-button
        >
        <el-divider />
        <el-empty v-if="!recommendationResult" description="点击获取推荐" />
        <div v-else>
          <el-timeline>
            <el-timeline-item
              v-for="(rec, i) in recommendationResult"
              :key="i"
              :type="rec.type === 'suggestion' ? 'primary' : 'success'"
              :timestamp="rec.created_at"
            >
              {{ rec.content }}
            </el-timeline-item>
          </el-timeline>
        </div>
      </el-card>
    </el-col>
  </el-row>
</template>
