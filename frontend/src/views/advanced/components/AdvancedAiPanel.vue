<script setup lang="ts">
/**
 * AdvancedAiPanel - AI 智能分析 tab 视图组件（纯展示）
 * 任务编号: P13 批 1 B3 I-1（拆分 advanced/index.vue 第 1 个 tab）
 * 包含：销售预测、库存优化建议、异常检测、智能推荐
 * 数据与函数全部由父组件通过 props 传入
 */
// v11 批次 171 P2-1 修复：从 useAi 导入接口类型，替代 any
import { useI18n } from 'vue-i18n'
import type {
  ForecastResult,
  InventoryResult,
  AnomalyItem,
  RecommendationItem,
} from '../composables/useAi'

interface Props {
  forecastPeriod: string
  forecastLoading: boolean
  forecastResult: ForecastResult | null
  runSalesForecast: () => Promise<void>
  inventoryLoading: boolean
  inventoryResult: InventoryResult | null
  runInventoryOptimization: () => Promise<void>
  anomalyType: string
  anomalyLoading: boolean
  anomalyResult: AnomalyItem[] | null
  runAnomalyDetection: () => Promise<void>
  recommendLoading: boolean
  recommendationResult: RecommendationItem[] | null
  getRecommendations: () => Promise<void>
  formatMoney: (amount: number) => string
}

defineProps<Props>()

const { t } = useI18n({ useScope: 'global' })

function priorityLabel(priority: string): string {
  if (priority === 'high') return t('advancedModule.ai.priorityHigh')
  if (priority === 'medium') return t('advancedModule.ai.priorityMedium')
  return t('advancedModule.ai.priorityLow')
}

const emit = defineEmits<{
  (e: 'update:forecastPeriod', value: string): void
  (e: 'update:anomalyType', value: string): void
}>()
</script>

<template>
  <div class="page-header">
    <h2 class="page-title">{{ $t('advancedModule.ai.title') }}</h2>
  </div>

  <el-row :gutter="20">
    <el-col :span="12">
      <el-card shadow="hover" class="mb-20">
        <template #header><div class="card-header">{{ $t('advancedModule.ai.salesForecast') }}</div></template>
        <el-form label-width="100px" aria-label="销售预测表单">
          <el-form-item :label="$t('advancedModule.ai.forecastPeriod')">
            <el-select
              :model-value="forecastPeriod"
              style="width: 100%"
              @update:model-value="(v: string) => emit('update:forecastPeriod', v)"
            >
              <el-option :label="$t('advancedModule.ai.period3m')" value="3m" />
              <el-option :label="$t('advancedModule.ai.period6m')" value="6m" />
              <el-option :label="$t('advancedModule.ai.period12m')" value="12m" />
            </el-select>
          </el-form-item>
          <el-form-item>
            <el-button type="primary" :loading="forecastLoading" @click="runSalesForecast"
              >{{ $t('advancedModule.ai.startForecast') }}</el-button
            >
          </el-form-item>
        </el-form>
        <el-empty v-if="!forecastResult" :description="$t('advancedModule.ai.clickToForecast')" />
        <div v-else>
          <h4>{{ $t('advancedModule.ai.forecastResult') }}</h4>
          <el-divider />
          <el-descriptions :column="2" border>
            <el-descriptions-item :label="$t('advancedModule.ai.forecastSales')">{{
              formatMoney(forecastResult.sales_amount)
            }}</el-descriptions-item>
            <el-descriptions-item :label="$t('advancedModule.ai.forecastOrders')">{{
              forecastResult.order_count
            }}</el-descriptions-item>
            <el-descriptions-item :label="$t('advancedModule.ai.confidence')"
              >{{ forecastResult.confidence }}%</el-descriptions-item
            >
            <el-descriptions-item :label="$t('advancedModule.ai.forecastTrend')">{{
              forecastResult.trend
            }}</el-descriptions-item>
          </el-descriptions>
        </div>
      </el-card>

      <el-card shadow="hover">
        <template #header><div class="card-header">{{ $t('advancedModule.ai.inventoryOpt') }}</div></template>
        <el-button
          type="primary"
          :loading="inventoryLoading"
          @click="runInventoryOptimization"
          >{{ $t('advancedModule.ai.generateSuggestion') }}</el-button
        >
        <el-divider />
        <el-empty v-if="!inventoryResult" :description="$t('advancedModule.ai.clickToGenerate')" />
        <div v-else>
          <el-alert type="success" :title="inventoryResult.summary" show-icon class="mb-10" />
          <el-table :data="inventoryResult.items" stripe aria-label="库存优化建议列表">
            <el-table-column prop="product_name" :label="$t('advancedModule.ai.colProduct')" width="150" />
            <el-table-column prop="suggestion" :label="$t('advancedModule.ai.colSuggestion')" min-width="200" />
            <el-table-column prop="priority" :label="$t('advancedModule.ai.colPriority')" width="100">
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
                  {{ priorityLabel(row.priority) }}
                </el-tag>
              </template>
            </el-table-column>
          </el-table>
        </div>
      </el-card>
    </el-col>

    <el-col :span="12">
      <el-card shadow="hover" class="mb-20">
        <template #header><div class="card-header">{{ $t('advancedModule.ai.anomalyDetection') }}</div></template>
        <el-form label-width="100px" aria-label="异常检测表单">
          <el-form-item :label="$t('advancedModule.ai.dataType')">
            <el-select
              :model-value="anomalyType"
              style="width: 100%"
              @update:model-value="(v: string) => emit('update:anomalyType', v)"
            >
              <el-option :label="$t('advancedModule.ai.dataSales')" value="sales" />
              <el-option :label="$t('advancedModule.ai.dataInventory')" value="inventory" />
              <el-option :label="$t('advancedModule.ai.dataQuality')" value="quality" />
            </el-select>
          </el-form-item>
          <el-form-item>
            <el-button type="primary" :loading="anomalyLoading" @click="runAnomalyDetection"
              >{{ $t('advancedModule.ai.detectAnomaly') }}</el-button
            >
          </el-form-item>
        </el-form>
        <el-empty v-if="!anomalyResult" :description="$t('advancedModule.ai.clickToDetect')" />
        <div v-else>
          <el-table :data="anomalyResult" stripe aria-label="异常检测结果列表">
            <el-table-column prop="item" :label="$t('advancedModule.ai.colItem')" width="150" />
            <el-table-column prop="type" :label="$t('advancedModule.ai.colType')" width="100">
              <template #default="{ row }">
                <el-tag
                  :type="row.severity === 'critical' ? 'danger' : 'warning'"
                  size="small"
                  >{{ row.type }}</el-tag
                >
              </template>
            </el-table-column>
            <el-table-column prop="description" :label="$t('advancedModule.ai.colDesc')" min-width="200" />
            <el-table-column prop="severity" :label="$t('advancedModule.ai.colSeverity')" width="100" />
          </el-table>
        </div>
      </el-card>

      <el-card shadow="hover">
        <template #header><div class="card-header">{{ $t('advancedModule.ai.recommendations') }}</div></template>
        <el-button type="primary" :loading="recommendLoading" @click="getRecommendations"
          >{{ $t('advancedModule.ai.getRecommendations') }}</el-button
        >
        <el-divider />
        <el-empty v-if="!recommendationResult" :description="$t('advancedModule.ai.clickToGetRec')" />
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
