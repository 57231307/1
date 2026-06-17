<script setup lang="ts">
/**
 * AI 工艺优化参数展示弹窗（P2-4）
 * 展示推荐参数 / 置信度 / 相似案例 / 反馈打分
 */
import { computed, ref, watch } from 'vue'
import type { ProcessOptResponse, AiProcessOptimization } from '@/api/ai-extend'
import { ElMessage } from 'element-plus'
import { applyProcessOptimization } from '@/api/ai-extend'

interface Props {
  visible: boolean
  optimization: AiProcessOptimization | null
  response: ProcessOptResponse | null
}
const props = defineProps<Props>()
const emit = defineEmits<{
  (e: 'update:visible', v: boolean): void
  (e: 'applied'): void
}>()

const dialogVisible = computed({
  get: () => props.visible,
  set: (v) => emit('update:visible', v),
})

const feedbackScore = ref<number>(0)
const feedbackRemark = ref<string>('')
const submitting = ref(false)

watch(
  () => props.visible,
  (v) => {
    if (v) {
      feedbackScore.value = props.optimization?.feedback_score ?? 0
      feedbackRemark.value = props.optimization?.feedback_remark ?? ''
    }
  },
)

const sourceLabel = computed(() => {
  if (!props.response) return ''
  return props.response.source === 'knn' ? 'k-NN 加权' : '典型参数表'
})

const candidates = computed(() => props.response?.candidates ?? [])

async function handleApply() {
  if (!props.optimization) return
  if (feedbackScore.value > 0 && (feedbackScore.value < 1 || feedbackScore.value > 5)) {
    ElMessage.warning('反馈评分须在 1-5 之间')
    return
  }
  submitting.value = true
  try {
    await applyProcessOptimization(props.optimization.id, {
      feedback_score: feedbackScore.value > 0 ? feedbackScore.value : undefined,
      feedback_remark: feedbackRemark.value || undefined,
    })
    ElMessage.success('已记录应用反馈')
    emit('applied')
    dialogVisible.value = false
  } catch (e) {
    ElMessage.error('应用失败，请稍后重试')
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <el-dialog
    v-model="dialogVisible"
    title="AI 工艺优化推荐"
    width="780px"
    :close-on-click-modal="false"
  >
    <template v-if="response && optimization">
      <div class="ai-opt-dialog">
        <!-- 推荐参数 -->
        <div class="section">
          <div class="section-title">推荐参数</div>
          <el-descriptions :column="4" border>
            <el-descriptions-item label="温度 (°C)">
              <span class="primary-value">
                {{ response.recommended_params.temperature.toFixed(1) }}
              </span>
            </el-descriptions-item>
            <el-descriptions-item label="时间 (分钟)">
              <span class="primary-value">
                {{ response.recommended_params.time_minutes }}
              </span>
            </el-descriptions-item>
            <el-descriptions-item label="pH 值">
              <span class="primary-value">
                {{ response.recommended_params.ph_value.toFixed(1) }}
              </span>
            </el-descriptions-item>
            <el-descriptions-item label="浴比 (1:X)">
              <span class="primary-value">
                {{ response.recommended_params.liquor_ratio.toFixed(1) }}
              </span>
            </el-descriptions-item>
          </el-descriptions>
        </div>

        <!-- 元数据 -->
        <div class="section">
          <el-row :gutter="16">
            <el-col :span="8">
              <el-statistic title="置信度" :value="response.confidence" :precision="3" />
            </el-col>
            <el-col :span="8">
              <el-statistic title="相似案例" :value="response.similar_cases" />
            </el-col>
            <el-col :span="8">
              <el-statistic title="推荐来源" :value="sourceLabel" />
            </el-col>
          </el-row>
        </div>

        <!-- 原因 -->
        <div class="section">
          <el-alert :title="response.reason" type="info" :closable="false" show-icon />
        </div>

        <!-- 候选案例 -->
        <div v-if="candidates.length" class="section">
          <div class="section-title">相似历史案例（最多 10 条）</div>
          <el-table :data="candidates" size="small" border>
            <el-table-column prop="case_id" label="案例 ID" width="90" />
            <el-table-column prop="color_no" label="色号" width="110" />
            <el-table-column prop="fabric_type" label="布类" width="100" />
            <el-table-column prop="similarity" label="相似度" width="100">
              <template #default="{ row }">
                <el-progress :percentage="Math.round(row.similarity * 100)" :stroke-width="6" />
              </template>
            </el-table-column>
            <el-table-column prop="temperature" label="温度" width="70" />
            <el-table-column prop="time_minutes" label="时间" width="70" />
            <el-table-column prop="ph_value" label="pH" width="70" />
            <el-table-column prop="liquor_ratio" label="浴比" />
          </el-table>
        </div>

        <!-- 反馈 -->
        <div class="section">
          <div class="section-title">应用反馈</div>
          <el-form label-width="100px" size="default">
            <el-form-item label="评分（1-5）">
              <el-rate v-model="feedbackScore" :max="5" />
            </el-form-item>
            <el-form-item label="备注">
              <el-input
                v-model="feedbackRemark"
                type="textarea"
                :rows="2"
                placeholder="如：工艺稳定 / 客户反映色牢度 / 浴比偏高"
                maxlength="200"
                show-word-limit
              />
            </el-form-item>
          </el-form>
        </div>
      </div>
    </template>

    <template #footer>
      <el-button @click="dialogVisible = false">关闭</el-button>
      <el-button type="primary" :loading="submitting" @click="handleApply">
        {{ optimization?.is_applied ? '更新反馈' : '标记为已应用' }}
      </el-button>
    </template>
  </el-dialog>
</template>

<style scoped>
.ai-opt-dialog {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.section {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.section-title {
  font-size: 14px;
  font-weight: 600;
  color: #303133;
}
.primary-value {
  font-size: 20px;
  font-weight: 700;
  color: #409eff;
}
</style>
