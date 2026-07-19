<script setup lang="ts">
/**
 * P2-4 工艺优化详情
 */
import { onMounted, ref, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import { getProcessOptimization, deleteProcessOptimization, SOURCE_LABELS, type AiProcessOptimization, type ProcessOptCandidate } from '@/api/ai-extend'

// 批次 34 v9 P1：接入 i18n，替换硬编码中文 ElMessage
const { t } = useI18n({ useScope: 'global' })

const route = useRoute()
const router = useRouter()
const id = computed(() => Number(route.params.id))
const loading = ref(false)
const model = ref<AiProcessOptimization | null>(null)
const candidates = ref<ProcessOptCandidate[]>([])

async function load() {
  if (!id.value || Number.isNaN(id.value)) {
    ElMessage.warning(t('aiExtend.process.invalidId'))
    return
  }
  loading.value = true
  try {
    const m = await getProcessOptimization(id.value)
    model.value = m
    if (m.candidates_json) {
      const json = m.candidates_json
      if (Array.isArray(json)) {
        candidates.value = json as ProcessOptCandidate[]
      }
    }
  } catch (e) {
    ElMessage.error(t('aiExtend.process.loadDetailFailed'))
  } finally {
    loading.value = false
  }
}

async function handleDelete() {
  if (!model.value) return
  await ElMessageBox.confirm(t('aiExtend.process.confirmDelete'), t('message.confirmTitle'), { type: 'warning' })
  try {
    await deleteProcessOptimization(model.value.id)
    ElMessage.success('已删除')
    router.replace('/ai-extend/process-optimization')
  } catch (e) {
    ElMessage.error(t('message.deleteFailed'))
  }
}

onMounted(load)
</script>

<template>
  <div v-loading="loading" class="proc-detail">
    <div class="page-header">
      <h2>工艺优化详情</h2>
      <div class="header-right">
        <el-button @click="router.back()">返回</el-button>
        <el-button v-permission="'ai_process_optimization:delete'" v-if="model" type="danger" @click="handleDelete">删除记录</el-button>
      </div>
    </div>

    <template v-if="model">
      <el-row :gutter="16">
        <el-col :span="14">
          <el-card>
            <template #header>
              <div class="card-header">推荐参数（应用至生产）</div>
            </template>
            <el-descriptions :column="2" border>
              <el-descriptions-item label="色号">{{ model.color_no }}</el-descriptions-item>
              <el-descriptions-item label="色名">{{ model.color_name || '—' }}</el-descriptions-item>
              <el-descriptions-item label="布类">{{ model.fabric_type }}</el-descriptions-item>
              <el-descriptions-item label="染料类型">{{ model.dye_type || '—' }}</el-descriptions-item>
              <el-descriptions-item label="推荐温度">
                <span class="primary-value">{{ model.recommended_temperature }} °C</span>
              </el-descriptions-item>
              <el-descriptions-item label="推荐时间">
                <span class="primary-value">{{ model.recommended_time_minutes }} 分钟</span>
              </el-descriptions-item>
              <el-descriptions-item label="推荐 pH">
                <span class="primary-value">{{ model.recommended_ph_value }}</span>
              </el-descriptions-item>
              <el-descriptions-item label="推荐浴比">
                <span class="primary-value">1 : {{ model.recommended_liquor_ratio }}</span>
              </el-descriptions-item>
              <el-descriptions-item label="推荐来源">
                <el-tag>{{ SOURCE_LABELS[model.source] || model.source }}</el-tag>
              </el-descriptions-item>
              <el-descriptions-item label="置信度">
                <el-progress :percentage="Math.round(Number(model.confidence) * 100)" :stroke-width="10" />
              </el-descriptions-item>
            </el-descriptions>

            <el-alert
              v-if="model.reason"
              :title="model.reason"
              type="info"
              :closable="false"
              show-icon
              style="margin-top: 12px"
            />
          </el-card>
        </el-col>

        <el-col :span="10">
          <el-card>
            <template #header>
              <div class="card-header">应用与反馈</div>
            </template>
            <el-descriptions :column="1" border>
              <el-descriptions-item label="应用状态">
                <el-tag :type="model.is_applied ? 'success' : 'info'">
                  {{ model.is_applied ? '已应用' : '未应用' }}
                </el-tag>
              </el-descriptions-item>
              <el-descriptions-item label="应用时间">
                {{ model.applied_at ? new Date(model.applied_at).toLocaleString('zh-CN') : '—' }}
              </el-descriptions-item>
              <el-descriptions-item label="反馈评分">
                <el-rate v-if="model.feedback_score" :model-value="model.feedback_score" disabled :max="5" />
                <span v-else>—</span>
              </el-descriptions-item>
              <el-descriptions-item label="反馈备注">{{ model.feedback_remark || '—' }}</el-descriptions-item>
              <el-descriptions-item label="相似案例数">{{ model.similar_cases }}</el-descriptions-item>
              <el-descriptions-item label="请求 ID">
                <span class="mono">{{ model.request_id }}</span>
              </el-descriptions-item>
              <el-descriptions-item label="创建时间">
                {{ new Date(model.created_at).toLocaleString('zh-CN') }}
              </el-descriptions-item>
            </el-descriptions>
          </el-card>
        </el-col>
      </el-row>

      <el-card v-if="candidates.length">
        <template #header>
          <div class="card-header">相似历史案例（最多 10 条）</div>
        </template>
        <el-table :data="candidates" size="small" border aria-label="相似候选案例列表">
          <el-table-column prop="case_id" label="案例 ID" width="100" />
          <el-table-column prop="color_no" label="色号" width="120" />
          <el-table-column prop="fabric_type" label="布类" width="100" />
          <el-table-column prop="similarity" label="相似度" width="200">
            <template #default="{ row }">
              <el-progress :percentage="Math.round(row.similarity * 100)" :stroke-width="8" />
            </template>
          </el-table-column>
          <el-table-column prop="temperature" label="温度" width="80" />
          <el-table-column prop="time_minutes" label="时间" width="80" />
          <el-table-column prop="ph_value" label="pH" width="80" />
          <el-table-column prop="liquor_ratio" label="浴比" />
        </el-table>
      </el-card>
    </template>
  </div>
</template>

<style scoped>
.proc-detail {
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
.primary-value {
  font-size: 18px;
  font-weight: 700;
  color: #409eff;
}
.mono {
  font-family: 'SFMono-Regular', Consolas, monospace;
  font-size: 12px;
}
.card-header {
  font-weight: 600;
}
</style>
