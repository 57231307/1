<script setup lang="ts">
/**
 * P2-4 AI 分析深化 - 概览看板
 */
import { onMounted, ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { DataAnalysis, Document, MagicStick } from '@element-plus/icons-vue'
import { getAiSummary, getAiHealth, RISK_LEVEL_LABELS, RISK_LEVEL_COLORS, TREND_LABELS, SOURCE_LABELS } from '@/api/ai-extend'
import type { AiSummary } from '@/api/ai-extend'

// 批次 34 v9 P1：接入 i18n，替换硬编码中文 ElMessage
const { t } = useI18n({ useScope: 'global' })

const router = useRouter()
const summary = ref<AiSummary | null>(null)
const loading = ref(false)
const health = ref<{ status: string; version: string } | null>(null)

async function load() {
  loading.value = true
  try {
    const [s, h] = await Promise.all([getAiSummary(), getAiHealth()])
    summary.value = s
    health.value = h
  } catch (e) {
    ElMessage.error(t('message.loadFailed'))
  } finally {
    loading.value = false
  }
}

onMounted(load)

const applyRateText = computed(() => {
  if (!summary.value) return '—'
  return `${(summary.value.process_optimization.apply_rate * 100).toFixed(1)}%`
})
const applyRateColor = computed(() => {
  if (!summary.value) return '#909399'
  const r = summary.value.process_optimization.apply_rate
  if (r >= 0.6) return '#67c23a'
  if (r >= 0.3) return '#e6a23c'
  return '#f56c6c'
})
</script>

<template>
  <div class="ai-overview">
    <div class="page-header">
      <h2>AI 分析深化看板</h2>
      <div class="header-right">
        <el-tag v-if="health" :type="health.status === 'ok' ? 'success' : 'danger'" size="small">
          服务 {{ health.status === 'ok' ? '正常' : '异常' }} · {{ health.version }}
        </el-tag>
        <el-button @click="load" :loading="loading" type="primary" plain>刷新</el-button>
      </div>
    </div>

    <el-row v-if="summary" :gutter="16" class="kpi-row">
      <el-col :span="6">
        <el-card shadow="hover" class="kpi-card">
          <div class="kpi-label">工艺优化历史</div>
          <div class="kpi-value">{{ summary.process_optimization.total }}</div>
          <div class="kpi-extra">k-NN 推荐 {{ summary.process_optimization.knn_recommended }} 条</div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover" class="kpi-card">
          <div class="kpi-label">应用率</div>
          <div class="kpi-value" :style="{ color: applyRateColor }">{{ applyRateText }}</div>
          <div class="kpi-extra">已应用 {{ summary.process_optimization.applied }} 条</div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover" class="kpi-card">
          <div class="kpi-label">质量预测历史</div>
          <div class="kpi-value">{{ summary.quality_prediction.total }}</div>
          <div class="kpi-extra">高风险 {{ summary.quality_prediction.high_risk }} 条</div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover" class="kpi-card">
          <div class="kpi-label">待确认预警</div>
          <div class="kpi-value" :style="{ color: summary.quality_prediction.unacknowledged > 0 ? '#f56c6c' : '#67c23a' }">
            {{ summary.quality_prediction.unacknowledged }}
          </div>
          <div class="kpi-extra">建议 24h 内确认</div>
        </el-card>
      </el-col>
    </el-row>

    <el-row :gutter="16" class="actions-row">
      <el-col :span="8">
        <el-card shadow="hover" class="action-card" @click="router.push('/ai-extend/process-optimization')">
          <div class="action-icon" style="background: #ecf5ff; color: #409eff;">
            <el-icon :size="32"><MagicStick /></el-icon>
          </div>
          <div class="action-body">
            <div class="action-title">工艺优化</div>
            <div class="action-desc">基于 k-NN 算法自动推荐染色温度 / 时间 / pH / 浴比</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card shadow="hover" class="action-card" @click="router.push('/ai-extend/quality-prediction')">
          <div class="action-icon" style="background: #fef0f0; color: #f56c6c;">
            <el-icon :size="32"><DataAnalysis /></el-icon>
          </div>
          <div class="action-body">
            <div class="action-title">质量预测</div>
            <div class="action-desc">基于历史趋势自动预测风险等级与建议措施</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card shadow="hover" class="action-card" @click="router.push('/ai-extend/process-detail/1')">
          <div class="action-icon" style="background: #f0f9eb; color: #67c23a;">
            <el-icon :size="32"><Document /></el-icon>
          </div>
          <div class="action-body">
            <div class="action-title">历史记录</div>
            <div class="action-desc">查看全部工艺优化与质量预测历史，支持应用反馈</div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-row v-if="summary" :gutter="16" class="latest-row">
      <el-col :span="12">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">最新工艺优化</div>
          </template>
          <el-table :data="summary.latest_process_optimizations" size="small" max-height="300" aria-label="最新工艺优化列表">
            <el-table-column prop="color_no" label="色号" width="100" />
            <el-table-column prop="fabric_type" label="布类" width="100" />
            <el-table-column prop="source" label="来源" width="100">
              <template #default="{ row }">{{ SOURCE_LABELS[row.source] || row.source }}</template>
            </el-table-column>
            <el-table-column prop="confidence" label="置信度" width="100">
              <template #default="{ row }">{{ Number(row.confidence).toFixed(2) }}</template>
            </el-table-column>
            <el-table-column prop="is_applied" label="应用" width="70">
              <template #default="{ row }">
                <el-tag :type="row.is_applied ? 'success' : 'info'" size="small">
                  {{ row.is_applied ? '已应用' : '未应用' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="created_at" label="时间" min-width="160" />
          </el-table>
        </el-card>
      </el-col>
      <el-col :span="12">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">最新质量预测</div>
          </template>
          <el-table :data="summary.latest_quality_predictions" size="small" max-height="300" aria-label="最新质量预测列表">
            <el-table-column prop="product_id" label="产品 ID" width="80" />
            <el-table-column prop="risk_level" label="风险" width="90">
              <template #default="{ row }">
                <el-tag
                  :style="{ background: RISK_LEVEL_COLORS[row.risk_level], color: '#fff', border: 'none' }"
                  size="small"
                >
                  {{ RISK_LEVEL_LABELS[row.risk_level] }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="risk_score" label="评分" width="70" />
            <el-table-column prop="trend" label="趋势" width="80">
              <template #default="{ row }">{{ TREND_LABELS[row.trend] || row.trend }}</template>
            </el-table-column>
            <el-table-column prop="is_acknowledged" label="确认" width="80">
              <template #default="{ row }">
                <el-tag :type="row.is_acknowledged ? 'success' : 'warning'" size="small">
                  {{ row.is_acknowledged ? '已确认' : '待确认' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="created_at" label="时间" min-width="160" />
          </el-table>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<style scoped>
.ai-overview {
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
  color: #303133;
}
.header-right {
  display: flex;
  gap: 8px;
  align-items: center;
}
.kpi-row {
  margin-bottom: 0;
}
.kpi-card {
  text-align: center;
}
.kpi-label {
  font-size: 13px;
  color: #909399;
}
.kpi-value {
  font-size: 32px;
  font-weight: 700;
  color: #303133;
  margin: 8px 0;
}
.kpi-extra {
  font-size: 12px;
  color: #909399;
}
.actions-row {
  margin-bottom: 0;
}
.action-card {
  display: flex;
  align-items: center;
  gap: 12px;
  cursor: pointer;
  transition: transform 0.2s;
}
.action-card:hover {
  transform: translateY(-2px);
}
.action-icon {
  width: 56px;
  height: 56px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
}
.action-body {
  flex: 1;
}
.action-title {
  font-size: 15px;
  font-weight: 600;
  color: #303133;
  margin-bottom: 4px;
}
.action-desc {
  font-size: 12px;
  color: #909399;
}
.latest-row {
  margin-bottom: 0;
}
.card-header {
  font-weight: 600;
}
</style>
