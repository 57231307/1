<script setup lang="ts">
/**
 * P2-4 质量预测列表 + 创建
 */
import { onMounted, reactive, ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  listQualityPredictions,
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
import AIPredictionChart from '@/components/ai/AIPredictionChart.vue'

// 批次 34 v9 P1：接入 i18n，替换硬编码中文 ElMessage
const { t } = useI18n({ useScope: 'global' })

const loading = ref(false)
const items = ref<AiQualityPrediction[]>([])
const total = ref(0)
const page = ref(1)
const pageSize = ref(20)

const queryFilter = reactive({
  product_id: undefined as number | undefined,
  inspection_type: undefined as string | undefined,
  risk_level: undefined as string | undefined,
  is_acknowledged: undefined as boolean | undefined,
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

async function load() {
  loading.value = true
  try {
    const res = await listQualityPredictions({
      page: page.value,
      page_size: pageSize.value,
      product_id: queryFilter.product_id,
      inspection_type: queryFilter.inspection_type,
      risk_level: queryFilter.risk_level,
      is_acknowledged: queryFilter.is_acknowledged,
    })
    items.value = res.items
    total.value = res.total
  } catch (e) {
    ElMessage.error(t('aiExtend.qualityPrediction.loadListFailed'))
  } finally {
    loading.value = false
  }
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
      `预测完成：${RISK_LEVEL_LABELS[resp.response.risk_level]}（评分 ${resp.response.risk_score}，趋势 ${TREND_LABELS[resp.response.trend as keyof typeof TREND_LABELS] ?? resp.response.trend}）`,
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
    ElMessage.success('已确认')
    await load()
  } catch (e) {
    ElMessage.error('确认失败')
  }
}

async function handleDelete(row: AiQualityPrediction) {
  await ElMessageBox.confirm(t('aiExtend.qualityPrediction.confirmDelete', { name: row.product_id ?? t('aiExtend.qualityPrediction.global') }), t('message.confirmTitle'), { type: 'warning' })
  try {
    await deleteQualityPrediction(row.id)
    ElMessage.success('已删除')
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
  page.value = 1
  load()
}

const ackOptions = [
  { value: undefined, label: '全部' },
  { value: false, label: '待确认' },
  { value: true, label: '已确认' },
]

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

onMounted(load)
</script>

<template>
  <div class="qual-page">
    <div class="page-header">
      <h2>质量预测历史</h2>
      <div class="header-right">
        <el-button type="primary" @click="openCreate">+ 触发新预测</el-button>
      </div>
    </div>

    <el-card class="filter-card">
      <el-form :inline="true" :model="queryFilter">
        <el-form-item label="产品 ID">
          <el-input-number v-model="queryFilter.product_id" :min="1" controls-position="right" style="width: 140px" />
        </el-form-item>
        <el-form-item label="检验类型">
          <el-select v-model="queryFilter.inspection_type" clearable style="width: 140px">
            <el-option v-for="o in INSPECTION_TYPE_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
          </el-select>
        </el-form-item>
        <el-form-item label="风险等级">
          <el-select v-model="queryFilter.risk_level" clearable style="width: 140px">
            <el-option v-for="o in RISK_LEVEL_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
          </el-select>
        </el-form-item>
        <el-form-item label="确认状态">
          <el-select v-model="queryFilter.is_acknowledged" clearable style="width: 140px">
            <el-option v-for="o in ackOptions" :key="String(o.value)" :label="o.label" :value="o.value" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="() => { page = 1; load() }">查询</el-button>
          <el-button @click="resetFilter">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card>
      <el-table v-loading="loading" :data="items" stripe border>
        <el-table-column prop="id" label="ID" width="70" />
        <el-table-column prop="product_id" label="产品 ID" width="90">
          <template #default="{ row }">{{ row.product_id ?? '全局' }}</template>
        </el-table-column>
        <el-table-column prop="inspection_type" label="检验类型" width="100">
          <template #default="{ row }">{{ INSPECTION_TYPE_LABELS[row.inspection_type] ?? row.inspection_type }}</template>
        </el-table-column>
        <el-table-column prop="total_inspections" label="检验总数" width="100" align="right" />
        <el-table-column prop="avg_qualification_rate" label="平均合格率" width="120">
          <template #default="{ row }">{{ Number(row.avg_qualification_rate).toFixed(1) }}%</template>
        </el-table-column>
        <el-table-column prop="risk_level" label="风险等级" width="100">
          <template #default="{ row }">
            <el-tag
              :style="{ background: RISK_LEVEL_COLORS[row.risk_level], color: '#fff', border: 'none' }"
              size="small"
            >
              {{ RISK_LEVEL_LABELS[row.risk_level] }} · {{ row.risk_score }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="trend" label="趋势" width="100">
          <template #default="{ row }">{{ TREND_LABELS[row.trend] ?? row.trend }} ({{ Number(row.trend_rate).toFixed(1) }}pp)</template>
        </el-table-column>
        <el-table-column prop="confidence" label="置信度" width="100">
          <template #default="{ row }">{{ Number(row.confidence).toFixed(2) }}</template>
        </el-table-column>
        <el-table-column prop="is_acknowledged" label="确认" width="80">
          <template #default="{ row }">
            <el-tag :type="row.is_acknowledged ? 'success' : 'warning'" size="small">
              {{ row.is_acknowledged ? '已确认' : '待确认' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_at" label="时间" min-width="160">
          <template #default="{ row }">{{ new Date(row.created_at).toLocaleString('zh-CN') }}</template>
        </el-table-column>
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <el-button size="small" @click="showDetail(row)">详情</el-button>
            <el-button v-permission="'ai_quality_prediction:approve'" v-if="!row.is_acknowledged" type="success" size="small" @click="handleAck(row)">确认</el-button>
            <el-button v-permission="'ai_quality_prediction:delete'" type="danger" size="small" @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
      <el-pagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :total="total"
        :page-sizes="[10, 20, 50, 100]"
        layout="total, sizes, prev, pager, next, jumper"
        @current-change="load"
        @size-change="() => { page = 1; load() }"
        style="margin-top: 16px; justify-content: flex-end"
      />
    </el-card>

    <!-- 创建弹窗 -->
    <el-dialog v-model="dialogVisible" title="触发 AI 质量预测" width="540px">
      <el-form :model="form" label-width="100px">
        <el-form-item label="产品 ID" required>
          <el-input-number v-model="form.product_id" :min="1" controls-position="right" style="width: 100%" />
        </el-form-item>
        <el-form-item label="检验类型">
          <el-select v-model="form.inspection_type" style="width: 100%">
            <el-option label="全部" value="all" />
            <el-option label="来料" value="incoming" />
            <el-option label="过程" value="inprocess" />
            <el-option label="成品" value="final" />
            <el-option label="出货" value="outgoing" />
          </el-select>
        </el-form-item>
        <el-form-item label="时间窗（天）">
          <el-input-number v-model="form.window_days" :min="7" :max="365" />
          <span class="hint">建议 30-180 天</span>
        </el-form-item>
        <el-alert
          title="历史检验数据 ≥ 5 条时走趋势分析；不足时走保守默认（合格率 95% / 置信度 0.3）"
          type="info"
          :closable="false"
          show-icon
        />
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="submitCreate">生成预测</el-button>
      </template>
    </el-dialog>

    <!-- 详情抽屉 -->
    <el-drawer v-model="detailVisible" title="质量预测详情" size="60%">
      <template v-if="detailModel">
        <div class="detail-section">
          <el-descriptions :column="2" border>
            <el-descriptions-item label="产品 ID">{{ detailModel.product_id ?? '全局' }}</el-descriptions-item>
            <el-descriptions-item label="检验类型">
              {{ INSPECTION_TYPE_LABELS[detailModel.inspection_type] ?? detailModel.inspection_type }}
            </el-descriptions-item>
            <el-descriptions-item label="时间窗">{{ detailModel.window_days }} 天</el-descriptions-item>
            <el-descriptions-item label="检验总数">{{ detailModel.total_inspections }}</el-descriptions-item>
            <el-descriptions-item label="平均合格率">
              {{ Number(detailModel.avg_qualification_rate).toFixed(1) }}%
            </el-descriptions-item>
            <el-descriptions-item label="置信度">
              <el-progress :percentage="Math.round(Number(detailModel.confidence) * 100)" :stroke-width="10" />
            </el-descriptions-item>
            <el-descriptions-item label="风险等级">
              <el-tag
                :style="{ background: RISK_LEVEL_COLORS[detailModel.risk_level], color: '#fff', border: 'none' }"
              >
                {{ RISK_LEVEL_LABELS[detailModel.risk_level] }} · {{ detailModel.risk_score }}
              </el-tag>
            </el-descriptions-item>
            <el-descriptions-item label="趋势">
              {{ TREND_LABELS[detailModel.trend] }} ({{ Number(detailModel.trend_rate).toFixed(1) }}pp)
            </el-descriptions-item>
          </el-descriptions>
        </div>

        <div class="detail-section">
          <div class="detail-section-title">趋势图</div>
          <AIPredictionChart
            :period-breakdown="detailPeriods"
            :risk-score="detailModel.risk_score"
            :risk-level="detailModel.risk_level as 'low' | 'medium' | 'high'"
            :trend="detailModel.trend as 'up' | 'flat' | 'down' | 'nodata'"
          />
        </div>

        <div v-if="detailIssues.length" class="detail-section">
          <div class="detail-section-title">主要问题归因</div>
          <el-table :data="detailIssues" size="small" border>
            <el-table-column prop="issue" label="问题类型" />
            <el-table-column prop="count" label="次数" width="100" />
            <el-table-column prop="percentage" label="占比" width="200">
              <template #default="{ row }">
                <el-progress :percentage="row.percentage" :stroke-width="8" />
              </template>
            </el-table-column>
          </el-table>
        </div>

        <div v-if="detailRecommendations.length" class="detail-section">
          <div class="detail-section-title">建议措施</div>
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
