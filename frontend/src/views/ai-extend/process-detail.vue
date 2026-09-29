<script setup lang="ts">
/**
 * P2-4 工艺优化详情
 */
import { onMounted, ref, computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  getProcessOptimization,
  deleteProcessOptimization,
  applyProcessOptimization,
  getProcessOptimizationListByColor,
  SOURCE_LABELS,
  type AiProcessOptimization,
  type ProcessOptCandidate,
} from '@/api/ai-extend';

// 批次 34 v9 P1：接入 i18n，替换硬编码中文 ElMessage
const { t } = useI18n({ useScope: 'global' });

const route = useRoute();
const router = useRouter();
const id = computed(() => Number(route.params.id));
const loading = ref(false);
const model = ref<AiProcessOptimization | null>(null);
const candidates = ref<ProcessOptCandidate[]>([]);

async function load() {
  if (!id.value || Number.isNaN(id.value)) {
    ElMessage.warning(t('aiExtend.process.invalidId'));
    return;
  }
  loading.value = true;
  try {
    const m = await getProcessOptimization(id.value);
    model.value = m;
    if (m.candidates_json) {
      const json = m.candidates_json;
      if (Array.isArray(json)) {
        candidates.value = json as ProcessOptCandidate[];
      }
    }
  } catch (e) {
    ElMessage.error(t('aiExtend.process.loadDetailFailed'));
  } finally {
    loading.value = false;
  }
}

async function handleDelete() {
  if (!model.value) return;
  await ElMessageBox.confirm(t('aiExtend.process.confirmDelete'), t('message.confirmTitle'), {
    type: 'warning',
  });
  try {
    await deleteProcessOptimization(model.value.id);
    ElMessage.success(t('aiExtend.process.deleted'));
    router.replace('/ai-extend/process-optimization');
  } catch (e) {
    ElMessage.error(t('message.deleteFailed'));
  }
}

// 应用工艺优化（登记反馈分与备注，后端落库 applied 状态）
const applying = ref(false);

async function handleApply() {
  if (!model.value) return;
  try {
    const { value } = await ElMessageBox.prompt(
      '反馈分（0-100，可留空）| 反馈备注（可留空），用 | 分隔',
      '应用工艺优化',
      { inputValidator: () => true }
    );
    const [score, remark] = value.split('|').map(s => s.trim());
    applying.value = true;
    await applyProcessOptimization(model.value.id, {
      feedback_score: score ? Number(score) : undefined,
      feedback_remark: remark || undefined,
    });
    ElMessage.success('已应用');
    await load();
  } catch (e) {
    if (e !== 'cancel') ElMessage.error((e as Error).message || '应用失败');
  } finally {
    applying.value = false;
  }
}

// 按色号 + 布类查询历史（列表模式 + 详情页联动查询）
const historyVisible = ref(false);
const historyLoading = ref(false);
const historyItems = ref<AiProcessOptimization[]>([]);

async function handleHistoryQuery() {
  if (!model.value?.color_no) {
    ElMessage.warning('当前记录无色号，无法查询');
    return;
  }
  try {
    const { value } = await ElMessageBox.prompt('请输入布类', '按色号查询历史', {
      inputValue: model.value.fabric_type || '',
      inputValidator: v => !!v.trim() || '布类必填',
    });
    historyLoading.value = true;
    historyVisible.value = true;
    const res = await getProcessOptimizationListByColor({
      color_no: model.value.color_no,
      fabric_type: value.trim(),
      limit: 20,
    });
    historyItems.value = res.items ?? [];
  } catch (e) {
    if (e !== 'cancel') ElMessage.error((e as Error).message || '查询历史失败');
  } finally {
    historyLoading.value = false;
  }
}

onMounted(load);
</script>

<template>
  <div v-loading="loading" class="proc-detail">
    <div class="page-header">
      <h2>{{ $t('aiExtend.process.detailTitle') }}</h2>
      <div class="header-right">
        <el-button @click="router.back()">{{ $t('aiExtend.process.back') }}</el-button>
        <el-button v-if="model" type="primary" :loading="applying" @click="handleApply"
          >应用</el-button
        >
        <el-button v-if="model" plain @click="handleHistoryQuery">按色号查历史</el-button>
        <el-button
          v-if="model"
          v-permission="'ai_process_optimization:delete'"
          type="danger"
          @click="handleDelete"
          >{{ $t('aiExtend.process.deleteRecord') }}</el-button
        >
      </div>
    </div>
    <!-- 空态：记录不存在或加载失败时给出可见提示（避免整页空白） -->
    <el-empty v-if="!loading && !model" :description="$t('aiExtend.process.recordNotFound')" />

    <template v-if="model">
      <el-row :gutter="16">
        <el-col :span="14">
          <el-card>
            <template #header>
              <div class="card-header">{{ $t('aiExtend.process.recommendParams') }}</div>
            </template>
            <el-descriptions :column="2" border>
              <el-descriptions-item :label="$t('aiExtend.process.colColorNo')">{{
                model.color_no
              }}</el-descriptions-item>
              <el-descriptions-item :label="$t('aiExtend.process.colColorName')">{{
                model.color_name || '—'
              }}</el-descriptions-item>
              <el-descriptions-item :label="$t('aiExtend.process.colFabricType')">{{
                model.fabric_type
              }}</el-descriptions-item>
              <el-descriptions-item :label="$t('aiExtend.process.colDyeType')">{{
                model.dye_type || '—'
              }}</el-descriptions-item>
              <el-descriptions-item :label="$t('aiExtend.process.recTemperature')">
                <span class="primary-value">{{ model.recommended_temperature }} °C</span>
              </el-descriptions-item>
              <el-descriptions-item :label="$t('aiExtend.process.recTime')">
                <span class="primary-value"
                  >{{ model.recommended_time_minutes }}
                  {{ $t('aiExtend.process.unitMinutes') }}</span
                >
              </el-descriptions-item>
              <el-descriptions-item :label="$t('aiExtend.process.recPh')">
                <span class="primary-value">{{ model.recommended_ph_value }}</span>
              </el-descriptions-item>
              <el-descriptions-item :label="$t('aiExtend.process.recLiquorRatio')">
                <span class="primary-value">1 : {{ model.recommended_liquor_ratio }}</span>
              </el-descriptions-item>
              <el-descriptions-item :label="$t('aiExtend.process.recSource')">
                <el-tag>{{ SOURCE_LABELS[model.source] || model.source }}</el-tag>
              </el-descriptions-item>
              <el-descriptions-item :label="$t('aiExtend.process.confidence')">
                <el-progress
                  :percentage="Math.round(Number(model.confidence) * 100)"
                  :stroke-width="10"
                />
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
              <div class="card-header">{{ $t('aiExtend.process.appAndFeedback') }}</div>
            </template>
            <el-descriptions :column="1" border>
              <el-descriptions-item :label="$t('aiExtend.process.applyStatus')">
                <el-tag :type="model.is_applied ? 'success' : 'info'">
                  {{
                    model.is_applied
                      ? $t('aiExtend.process.applied')
                      : $t('aiExtend.process.notApplied')
                  }}
                </el-tag>
              </el-descriptions-item>
              <el-descriptions-item :label="$t('aiExtend.process.applyTime')">
                {{ model.applied_at ? new Date(model.applied_at).toLocaleString('zh-CN') : '—' }}
              </el-descriptions-item>
              <el-descriptions-item :label="$t('aiExtend.process.feedbackScore')">
                <el-rate
                  v-if="model.feedback_score"
                  :model-value="model.feedback_score"
                  disabled
                  :max="5"
                />
                <span v-else>—</span>
              </el-descriptions-item>
              <el-descriptions-item :label="$t('aiExtend.process.feedbackRemark')">{{
                model.feedback_remark || '—'
              }}</el-descriptions-item>
              <el-descriptions-item :label="$t('aiExtend.process.similarCases')">{{
                model.similar_cases
              }}</el-descriptions-item>
              <el-descriptions-item :label="$t('aiExtend.process.requestId')">
                <span class="mono">{{ model.request_id }}</span>
              </el-descriptions-item>
              <el-descriptions-item :label="$t('aiExtend.process.createdAt')">
                {{ new Date(model.created_at).toLocaleString('zh-CN') }}
              </el-descriptions-item>
            </el-descriptions>
          </el-card>
        </el-col>
      </el-row>

      <el-card v-if="candidates.length">
        <template #header>
          <div class="card-header">{{ $t('aiExtend.process.similarHistory') }}</div>
        </template>
        <el-table
          :data="candidates"
          size="small"
          border
          :aria-label="t('aiExtend.process.ariaSimilarHistory')"
        >
          <el-table-column prop="case_id" :label="$t('aiExtend.process.colCaseId')" width="100" />
          <el-table-column prop="color_no" :label="$t('aiExtend.process.colColorNo')" width="120" />
          <el-table-column
            prop="fabric_type"
            :label="$t('aiExtend.process.colFabricType')"
            width="100"
          />
          <el-table-column
            prop="similarity"
            :label="$t('aiExtend.process.colSimilarity')"
            width="200"
          >
            <template #default="{ row }">
              <el-progress :percentage="Math.round(row.similarity * 100)" :stroke-width="8" />
            </template>
          </el-table-column>
          <el-table-column
            prop="temperature"
            :label="$t('aiExtend.process.colTemperature')"
            width="80"
          />
          <el-table-column prop="time_minutes" :label="$t('aiExtend.process.colTime')" width="80" />
          <el-table-column prop="ph_value" :label="$t('aiExtend.process.colPh')" width="80" />
          <el-table-column prop="liquor_ratio" :label="$t('aiExtend.process.colLiquorRatio')" />
        </el-table>
      </el-card>
    </template>

    <el-dialog
      v-model="historyVisible"
      title="同色号历史记录"
      width="720"
      v-loading="historyLoading"
    >
      <el-table :data="historyItems" border size="small" max-height="360">
        <el-table-column prop="id" label="ID" width="70" />
        <el-table-column prop="color_no" label="色号" width="110" />
        <el-table-column prop="fabric_type" label="布类" width="110" />
        <el-table-column prop="dye_type" label="染色类型" width="110" />
        <el-table-column prop="status" label="状态" width="100" />
        <el-table-column prop="applied_at" label="应用时间" min-width="150">
          <template #default="{ row }">{{ row.applied_at || '-' }}</template>
        </el-table-column>
      </el-table>
      <template #footer>
        <el-button @click="historyVisible = false">关闭</el-button>
      </template>
    </el-dialog>
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
