<script setup lang="ts">
/**
 * P2-4 质量预测列表 + 创建
 */
import { reactive, ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  createQualityPrediction,
  acknowledgeQualityPrediction,
  deleteQualityPrediction,
  RISK_LEVEL_LABELS,
  RISK_LEVEL_COLORS,
  TREND_LABELS,
  INSPECTION_TYPE_LABELS,
  type AiQualityPrediction,
  type QualityPredRequest,
} from '@/api/ai-extend'
import AiPredictionChart from '@/components/ai/AiPredictionChart.vue'
// 批次 280：接入 useTableApi，消除手写 items/loading/total/page/pageSize/load 重复
import { useTableApi } from '@/composables/useTableApi'

// 批次 34 v9 P1：接入 i18n，替换硬编码中文 ElMessage
const { t } = useI18n({ useScope: 'global' })

const queryFilter = reactive({
  product_id: undefined as number | undefined,
  inspection_type: undefined as string | undefined,
  risk_level: undefined as string | undefined,
  is_acknowledged: undefined as boolean | undefined,
})

// 批次 280：useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
// getQualityPredictionList 返回 PageResult<T>（{ items, total }），useTableApi detectList 会取 obj.items
const {
  data: items,
  loading,
  page,
  pageSize,
  total,
  refresh: load,
  setQueryParam,
} = useTableApi<AiQualityPrediction>({
  url: '/ai/quality-predictions',
  onError: () => ElMessage.error(t('aiExtend.qualityPrediction.loadListFailed')),
})

// 派生下拉选项（从 LABEL 字典生成 OPTIONS 数组，供 el-select 使用）
const INSPECTION_TYPE_OPTIONS = (Object.entries(INSPECTION_TYPE_LABELS) as [string, string][])
  .filter(([k]) => k !== 'all')
  .map(([value, label]) => ({ value, label }))
const RISK_LEVEL_OPTIONS = (Object.entries(RISK_LEVEL_LABELS) as [string, string][])
  .map(([value, label]) => ({ value, label }))

const dialogVisible = ref(false)
const form = reactive<QualityPredRequest>({
  product_id: undefined,
  inspection_type: 'all',
  window_days: 90,
})
const submitting = ref(false)

// 详情抽屉
const detailVisible = ref(false)
const detailModel = ref<AiQualityPrediction | null>(null)

// 批次 280：同步筛选条件到 useTableApi.queryParams 并刷新
function syncQueryParams() {
  setQueryParam('product_id', queryFilter.product_id)
  setQueryParam('inspection_type', queryFilter.inspection_type)
  setQueryParam('risk_level', queryFilter.risk_level)
  setQueryParam('is_acknowledged', queryFilter.is_acknowledged)
}

function handleSearch() {
  syncQueryParams()
  page.value = 1
  load()
}

function openCreate() {
  form.product_id = undefined
  form.inspection_type = 'all'
  form.window_days = 90
  dialogVisible.value = true
}

async function submitCreate() {
  if (form.product_id === undefined || form.product_id === null) {
    ElMessage.warning(t('aiExtend.qualityPrediction.productIdRequired'))
    return
  }
  submitting.value = true
  try {
    const resp = await createQualityPrediction({ ...form })
    ElMessage.success(
      t('aiExtend.qualityPrediction.predictSuccess', {
        risk: RISK_LEVEL_LABELS[resp.response.risk_level],
        score: resp.response.risk_score,
        trend: TREND_LABELS[resp.response.trend as keyof typeof TREND_LABELS] ?? resp.response.trend,
      }),
    )
    dialogVisible.value = false
    page.value = 1
    await load()
    showDetail(items.value[0])
  } catch (e) {
    ElMessage.error(t('message.createFailed'))
  } finally {
    submitting.value = false
  }
}

async function handleAck(row: AiQualityPrediction) {
  try {
    await acknowledgeQualityPrediction(row.id)
    ElMessage.success(t('aiExtend.qualityPrediction.ackSuccess'))
    await load()
  } catch (e) {
    ElMessage.error(t('aiExtend.qualityPrediction.ackFailed'))
  }
}

async function handleDelete(row: AiQualityPrediction) {
  await ElMessageBox.confirm(t('aiExtend.qualityPrediction.confirmDelete', { name: row.product_id ?? t('aiExtend.qualityPrediction.global') }), t('message.confirmTitle'), { type: 'warning' })
  try {
    await deleteQualityPrediction(row.id)
    ElMessage.success(t('aiExtend.qualityPrediction.deleted'))
    await load()
  } catch (e) {
    ElMessage.error(t('message.deleteFailed'))
  }
}

function showDetail(row: AiQualityPrediction) {
  detailModel.value = row
  detailVisible.value = true
}

function resetFilter() {
  queryFilter.product_id = undefined
  queryFilter.inspection_type = undefined
  queryFilter.risk_level = undefined
  queryFilter.is_acknowledged = undefined
  syncQueryParams()
  page.value = 1
  load()
}

const ackOptions = computed(() => [
  { value: undefined, label: t('aiExtend.qualityPrediction.ackAll') },
  { value: false, label: t('aiExtend.qualityPrediction.ackPending') },
  { value: true, label: t('aiExtend.qualityPrediction.acknowledged') },
])

const detailPeriods = computed(() => {
  if (!detailModel.value) return [] as { period: string; inspections: number; avg_qualification_rate: number }[]
  const json = detailModel.value.period_breakdown_json
  if (Array.isArray(json)) return json as { period: string; inspections: number; avg_qualification_rate: number }[]
  return []
})

const detailIssues = computed(() => {
  if (!detailModel.value) return [] as { issue: string; count: number; percentage: number }[]
  const json = detailModel.value.top_issues_json
  if (Array.isArray(json)) return json as { issue: string; count: number; percentage: number }[]
  return []
})

const detailRecommendations = computed(() => {
  if (!detailModel.value) return [] as string[]
  const json = detailModel.value.recommendations_json
  if (Array.isArray(json)) return json as string[]
  return []
})
</script>

<template>
  <div class="qual-page">
    <div class="page-header">
      <h2>{{ $t('aiExtend.qualityPrediction.listTitle') }}</h2>
      <div class="header-right">
        <el-button type="primary" @click="openCreate">{{ $t('aiExtend.qualityPrediction.newPredict') }}</el-button>
      </div>
    </div>

    <el-card class="filter-card">
      <el-form :inline="true" :model="queryFilter" aria-label="AI 质量预测筛选表单">
        <el-form-item :label="$t('aiExtend.qualityPrediction.colProductId')">
          <el-input-number v-model="queryFilter.product_id" :min="1" controls-position="right" style="width: 140px" />
        </el-form-item>
        <el-form-item :label="$t('aiExtend.qualityPrediction.colInspectionType')">
          <el-select v-model="queryFilter.inspection_type" clearable style="width: 140px">
            <el-option v-for="o in INSPECTION_TYPE_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('aiExtend.qualityPrediction.colRiskLevel')">
          <el-select v-model="queryFilter.risk_level" clearable style="width: 140px">
            <el-option v-for="o in RISK_LEVEL_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('aiExtend.qualityPrediction.colAckStatus')">
          <el-select v-model="queryFilter.is_acknowledged" clearable style="width: 140px">
            <el-option v-for="o in ackOptions" :key="String(o.value)" :label="o.label" :value="o.value" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleSearch">{{ $t('aiExtend.qualityPrediction.query') }}</el-button>
          <el-button @click="resetFilter">{{ $t('aiExtend.qualityPrediction.reset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card>
      <el-table v-loading="loading" :data="items" stripe border aria-label="AI 质量预测列表">
        <el-table-column prop="id" label="ID" width="70" />
        <el-table-column prop="product_id" :label="$t('aiExtend.qualityPrediction.colProductId')" width="90">
          <template #default="{ row }">{{ row.product_id ?? $t('aiExtend.qualityPrediction.global') }}</template>
        </el-table-column>
        <el-table-column prop="inspection_type" :label="$t('aiExtend.qualityPrediction.colInspectionType')" width="100">
          <template #default="{ row }">{{ INSPECTION_TYPE_LABELS[row.inspection_type] ?? row.inspection_type }}</template>
        </el-table-column>
        <el-table-column prop="total_inspections" :label="$t('aiExtend.qualityPrediction.colTotalInspections')" width="100" align="right" />
        <el-table-column prop="avg_qualification_rate" :label="$t('aiExtend.qualityPrediction.colAvgRate')" width="120">
          <template #default="{ row }">{{ Number(row.avg_qualification_rate).toFixed(1) }}%</template>
        </el-table-column>
        <el-table-column prop="risk_level" :label="$t('aiExtend.qualityPrediction.colRiskLevel')" width="100">
          <template #default="{ row }">
            <el-tag
              :style="{ background: RISK_LEVEL_COLORS[row.risk_level], color: '#fff', border: 'none' }"
              size="small"
            >
              {{ RISK_LEVEL_LABELS[row.risk_level] }} · {{ row.risk_score }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="trend" :label="$t('aiExtend.qualityPrediction.colTrend')" width="100">
          <template #default="{ row }">{{ TREND_LABELS[row.trend] ?? row.trend }} ({{ Number(row.trend_rate).toFixed(1) }}pp)</template>
        </el-table-column>
        <el-table-column prop="confidence" :label="$t('aiExtend.qualityPrediction.colConfidence')" width="100">
          <template #default="{ row }">{{ Number(row.confidence).toFixed(2) }}</template>
        </el-table-column>
        <el-table-column prop="is_acknowledged" :label="$t('aiExtend.qualityPrediction.colAck')" width="80">
          <template #default="{ row }">
            <el-tag :type="row.is_acknowledged ? 'success' : 'warning'" size="small">
              {{ row.is_acknowledged ? $t('aiExtend.qualityPrediction.acknowledged') : $t('aiExtend.qualityPrediction.ackPending') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_at" :label="$t('aiExtend.qualityPrediction.colTime')" min-width="160">
          <template #default="{ row }">{{ new Date(row.created_at).toLocaleString('zh-CN') }}</template>
        </el-table-column>
        <el-table-column :label="$t('aiExtend.qualityPrediction.colAction')" width="200" fixed="right">
          <template #default="{ row }">
            <el-button size="small" @click="showDetail(row)">{{ $t('aiExtend.qualityPrediction.detail') }}</el-button>
            <el-button v-permission="'ai_quality_prediction:approve'" v-if="!row.is_acknowledged" type="success" size="small" @click="handleAck(row)">{{ $t('aiExtend.qualityPrediction.ack') }}</el-button>
            <el-button v-permission="'ai_quality_prediction:delete'" type="danger" size="small" @click="handleDelete(row)">{{ $t('aiExtend.qualityPrediction.delete') }}</el-button>
          </template>
        </el-table-column>
      </el-table>
      <el-pagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :total="total"
        :page-sizes="[10, 20, 50, 100]"
        layout="total, sizes, prev, pager, next, jumper"
        aria-label="AI 质量预测列表分页"
        style="margin-top: 16px; justify-content: flex-end"
      />
    </el-card>

    <!-- 创建弹窗 -->
    <el-dialog v-model="dialogVisible" :title="$t('aiExtend.qualityPrediction.createDialogTitle')" width="540px" aria-label="触发 AI 质量预测对话框">
      <el-form :model="form" label-width="100px" aria-label="AI 质量预测表单">
        <el-form-item :label="$t('aiExtend.qualityPrediction.colProductId')" required>
          <el-input-number v-model="form.product_id" :min="1" controls-position="right" style="width: 100%" />
        </el-form-item>
        <el-form-item :label="$t('aiExtend.qualityPrediction.colInspectionType')">
          <el-select v-model="form.inspection_type" style="width: 100%">
            <el-option :label="$t('aiExtend.qualityPrediction.typeAll')" value="all" />
            <el-option :label="$t('aiExtend.qualityPrediction.typeIncoming')" value="incoming" />
            <el-option :label="$t('aiExtend.qualityPrediction.typeInprocess')" value="inprocess" />
            <el-option :label="$t('aiExtend.qualityPrediction.typeFinal')" value="final" />
            <el-option :label="$t('aiExtend.qualityPrediction.typeOutgoing')" value="outgoing" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('aiExtend.qualityPrediction.timeWindow')">
          <el-input-number v-model="form.window_days" :min="7" :max="365" />
          <span class="hint">{{ $t('aiExtend.qualityPrediction.timeWindowHint') }}</span>
        </el-form-item>
        <el-alert
          :title="$t('aiExtend.qualityPrediction.alertText')"
          type="info"
          :closable="false"
          show-icon
        />
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ $t('aiExtend.qualityPrediction.cancel') }}</el-button>
        <el-button type="primary" :loading="submitting" @click="submitCreate">{{ $t('aiExtend.qualityPrediction.generate') }}</el-button>
      </template>
    </el-dialog>

    <!-- 详情抽屉 -->
    <el-drawer v-model="detailVisible" :title="$t('aiExtend.qualityPrediction.detailTitle')" size="60%">
      <template v-if="detailModel">
        <div class="detail-section">
          <el-descriptions :column="2" border>
            <el-descriptions-item :label="$t('aiExtend.qualityPrediction.colProductId')">{{ detailModel.product_id ?? $t('aiExtend.qualityPrediction.global') }}</el-descriptions-item>
            <el-descriptions-item :label="$t('aiExtend.qualityPrediction.colInspectionType')">
              {{ INSPECTION_TYPE_LABELS[detailModel.inspection_type] ?? detailModel.inspection_type }}
            </el-descriptions-item>
            <el-descriptions-item :label="$t('aiExtend.qualityPrediction.detailTimeWindow')">{{ detailModel.window_days }} {{ $t('aiExtend.qualityPrediction.detailUnitDays') }}</el-descriptions-item>
            <el-descriptions-item :label="$t('aiExtend.qualityPrediction.colTotalInspections')">{{ detailModel.total_inspections }}</el-descriptions-item>
            <el-descriptions-item :label="$t('aiExtend.qualityPrediction.colAvgRate')">
              {{ Number(detailModel.avg_qualification_rate).toFixed(1) }}%
            </el-descriptions-item>
            <el-descriptions-item :label="$t('aiExtend.qualityPrediction.colConfidence')">
              <el-progress :percentage="Math.round(Number(detailModel.confidence) * 100)" :stroke-width="10" />
            </el-descriptions-item>
            <el-descriptions-item :label="$t('aiExtend.qualityPrediction.colRiskLevel')">
              <el-tag
                :style="{ background: RISK_LEVEL_COLORS[detailModel.risk_level], color: '#fff', border: 'none' }"
              >
                {{ RISK_LEVEL_LABELS[detailModel.risk_level] }} · {{ detailModel.risk_score }}
              </el-tag>
            </el-descriptions-item>
            <el-descriptions-item :label="$t('aiExtend.qualityPrediction.colTrend')">
              {{ TREND_LABELS[detailModel.trend] }} ({{ Number(detailModel.trend_rate).toFixed(1) }}pp)
            </el-descriptions-item>
          </el-descriptions>
        </div>

        <div class="detail-section">
          <div class="detail-section-title">{{ $t('aiExtend.qualityPrediction.trendChart') }}</div>
          <AiPredictionChart
            :period-breakdown="detailPeriods"
            :risk-score="detailModel.risk_score"
            :risk-level="detailModel.risk_level as 'low' | 'medium' | 'high'"
            :trend="detailModel.trend as 'up' | 'flat' | 'down' | 'nodata'"
          />
        </div>

        <div v-if="detailIssues.length" class="detail-section">
          <div class="detail-section-title">{{ $t('aiExtend.qualityPrediction.topIssues') }}</div>
          <el-table :data="detailIssues" size="small" border aria-label="主要问题归因列表">
            <el-table-column prop="issue" :label="$t('aiExtend.qualityPrediction.colIssue')" />
            <el-table-column prop="count" :label="$t('aiExtend.qualityPrediction.colCount')" width="100" />
            <el-table-column prop="percentage" :label="$t('aiExtend.qualityPrediction.colPercentage')" width="200">
              <template #default="{ row }">
                <el-progress :percentage="row.percentage" :stroke-width="8" />
              </template>
            </el-table-column>
          </el-table>
        </div>

        <div v-if="detailRecommendations.length" class="detail-section">
          <div class="detail-section-title">{{ $t('aiExtend.qualityPrediction.recommendations') }}</div>
          <el-alert
            v-for="(rec, idx) in detailRecommendations"
            :key="idx"
            :title="rec"
            type="warning"
            :closable="false"
            show-icon
            style="margin-bottom: 8px"
          />
        </div>
      </template>
    </el-drawer>
  </div>
</template>

<style scoped>
.qual-page {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.page-header h2 {
  margin: 0;
  font-size: 20px;
}
.header-right {
  display: flex;
  gap: 8px;
}
.filter-card {
  margin-bottom: 0;
}
.hint {
  margin-left: 8px;
  font-size: 12px;
  color: #909399;
}
.detail-section {
  margin-bottom: 20px;
}
.detail-section-title {
  font-size: 14px;
  font-weight: 600;
  margin-bottom: 8px;
  color: #303133;
}
</style>
