<!--
  质量检查组件
  - 异常列表
  - 上报异常（GB/T 26377 色差 + ISO 105 色牢度）
  - 解决异常
-->
<template>
  <div class="quality-check">
    <div class="action-bar">
      <el-button type="primary" @click="showReportDialog">
        <el-icon><Plus /></el-icon>
        {{ t('common.qualityCheck.reportIssue') }}
      </el-button>
    </div>

    <el-table
      :data="issues"
      border
      stripe
      :empty-text="t('common.qualityCheck.emptyText')"
      :aria-label="t('common.qualityCheck.tableAriaLabel')"
    >
      <el-table-column
        prop="issue_type"
        :label="t('common.qualityCheck.colIssueType')"
        width="140"
      />
      <el-table-column :label="t('common.qualityCheck.colSeverity')" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="ISSUE_SEVERITY_COLORS[row.severity] || 'info'">
            {{ getSeverityLabel(row.severity) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column
        prop="description"
        :label="t('common.qualityCheck.colDescription')"
        min-width="200"
        show-overflow-tooltip
      />
      <el-table-column :label="t('common.qualityCheck.colDiscoveredAt')" width="170">
        <template #default="{ row }">
          {{ formatDate(row.discovered_at) }}
        </template>
      </el-table-column>
      <el-table-column :label="t('common.qualityCheck.colStatus')" width="100" align="center">
        <template #default="{ row }">
          <el-tag
            :type="
              row.status === 'open' ? 'danger' : row.status === 'resolved' ? 'success' : 'info'
            "
          >
            {{ getStatusLabel(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column
        prop="resolution"
        :label="t('common.qualityCheck.colResolution')"
        min-width="200"
        show-overflow-tooltip
      />
      <el-table-column :label="t('common.qualityCheck.colOperation')" width="120" fixed="right">
        <template #default="{ row }">
          <el-button
            v-if="row.status === 'open' || row.status === 'investigating'"
            size="small"
            type="success"
            link
            @click="handleResolve(row)"
          >
            {{ t('common.qualityCheck.resolve') }}
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- 上报异常对话框 -->
    <el-dialog
      v-model="reportVisible"
      :title="t('common.qualityCheck.reportDialogTitle')"
      :aria-label="t('common.qualityCheck.reportDialogAriaLabel')"
      width="540px"
    >
      <el-form
        ref="reportFormRef"
        :model="reportForm"
        :rules="reportRules"
        label-width="100px"
        :aria-label="t('common.qualityCheck.reportFormAriaLabel')"
      >
        <el-form-item :label="t('common.qualityCheck.formIssueType')" prop="issue_type">
          <el-select
            v-model="reportForm.issue_type"
            :placeholder="t('common.qualityCheck.formIssueTypePlaceholder')"
          >
            <el-option :label="t('common.qualityCheck.issueType.colorDiff')" value="color_diff" />
            <el-option
              :label="t('common.qualityCheck.issueType.colorFastness')"
              value="color_fastness"
            />
            <el-option :label="t('common.qualityCheck.issueType.spec')" value="spec" />
            <el-option :label="t('common.qualityCheck.issueType.damage')" value="damage" />
            <el-option :label="t('common.qualityCheck.issueType.other')" value="other" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('common.qualityCheck.formSeverity')" prop="severity">
          <el-radio-group v-model="reportForm.severity">
            <el-radio-button label="low">{{
              t('common.qualityCheck.severity.low')
            }}</el-radio-button>
            <el-radio-button label="medium">{{
              t('common.qualityCheck.severity.medium')
            }}</el-radio-button>
            <el-radio-button label="high">{{
              t('common.qualityCheck.severity.high')
            }}</el-radio-button>
            <el-radio-button label="critical">{{
              t('common.qualityCheck.severity.critical')
            }}</el-radio-button>
          </el-radio-group>
        </el-form-item>
        <el-form-item
          v-if="reportForm.issue_type === 'color_diff'"
          :label="t('common.qualityCheck.colorDeltaE')"
        >
          <el-input-number v-model="reportForm.color_delta_e" :min="0" :precision="2" :step="0.5" />
          <span style="margin-left: 8px; color: #909399; font-size: 12px">
            {{ t('common.qualityCheck.colorDeltaEHint') }}
          </span>
        </el-form-item>
        <el-form-item
          v-if="reportForm.issue_type === 'color_fastness'"
          :label="t('common.qualityCheck.colorFastnessGrade')"
        >
          <el-select
            v-model="reportForm.color_fastness_grade"
            :placeholder="t('common.qualityCheck.colorFastnessGradePlaceholder')"
          >
            <el-option
              v-for="i in [1, 2, 3, 4, 5]"
              :key="i"
              :label="t('common.qualityCheck.grade', { n: i })"
              :value="i"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('common.qualityCheck.formDescription')" prop="description">
          <el-input v-model="reportForm.description" type="textarea" :rows="3" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="reportVisible = false">{{ t('common.qualityCheck.cancel') }}</el-button>
        <el-button type="primary" :loading="submitting" @click="handleReportSubmit">{{
          t('common.qualityCheck.submit')
        }}</el-button>
      </template>
    </el-dialog>

    <!-- 解决异常对话框 -->
    <el-dialog
      v-model="resolveVisible"
      :title="t('common.qualityCheck.resolveDialogTitle')"
      width="500px"
      :aria-label="t('common.qualityCheck.resolveDialogAriaLabel')"
    >
      <el-form
        :model="resolveForm"
        label-width="80px"
        :aria-label="t('common.qualityCheck.resolveFormAriaLabel')"
      >
        <el-form-item :label="t('common.qualityCheck.formResolution')" required>
          <el-input v-model="resolveForm.resolution" type="textarea" :rows="3" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="resolveVisible = false">{{ t('common.qualityCheck.cancel') }}</el-button>
        <el-button type="primary" :loading="submitting" @click="handleResolveSubmit">{{
          t('common.qualityCheck.confirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { ElMessage } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import { useI18n } from 'vue-i18n';
import {
  reportQualityIssue,
  resolveQualityIssue,
  ISSUE_SEVERITY_COLORS,
  type QualityIssue,
} from '@/api/custom-order';

const { t } = useI18n({ useScope: 'global' });

// v11 批次 181 P2-1 修复：使用 API 导出的 QualityIssue 类型，替代本地定义

const props = defineProps<{
  orderId: number;
  issues: QualityIssue[];
}>();

const emit = defineEmits<{ (e: 'refresh'): void }>();

const reportVisible = ref(false);
const resolveVisible = ref(false);
const submitting = ref(false);
const reportFormRef = ref();
const currentIssue = ref<QualityIssue | null>(null);

const reportForm = ref({
  issue_type: '',
  severity: 'medium',
  description: '',
  color_delta_e: undefined as number | undefined,
  color_fastness_grade: undefined as number | undefined,
});

const resolveForm = ref({ resolution: '' });

const reportRules = computed(() => ({
  issue_type: [
    { required: true, message: t('common.qualityCheck.rule.issueTypeRequired'), trigger: 'change' },
  ],
  severity: [
    { required: true, message: t('common.qualityCheck.rule.severityRequired'), trigger: 'change' },
  ],
  description: [
    { required: true, message: t('common.qualityCheck.rule.descriptionRequired'), trigger: 'blur' },
  ],
}));

/** 严重度标签映射（响应式 t() 求值，替代导入的 ISSUE_SEVERITY 中文常量） */
function getSeverityLabel(s: string): string {
  const known = ['low', 'medium', 'high', 'critical'];
  if (!known.includes(s)) return s;
  return t(`common.qualityCheck.severity.${s}`);
}

/** 状态标签映射（响应式 t() 求值，替代直接显示 row.status 原值） */
function getStatusLabel(s: string): string {
  const known = ['open', 'investigating', 'resolved'];
  if (!known.includes(s)) return s;
  return t(`common.qualityCheck.status.${s}`);
}

function formatDate(d: string | undefined) {
  if (!d) return '-';
  return new Date(d).toLocaleString('zh-CN');
}

function showReportDialog() {
  reportForm.value = {
    issue_type: '',
    severity: 'medium',
    description: '',
    color_delta_e: undefined,
    color_fastness_grade: undefined,
  };
  reportVisible.value = true;
}

async function handleReportSubmit() {
  if (!reportFormRef.value) return;
  try {
    await reportFormRef.value.validate();
  } catch {
    return;
  }
  submitting.value = true;
  try {
    await reportQualityIssue(props.orderId, reportForm.value);
    ElMessage.success(t('common.qualityCheck.reportSuccess'));
    reportVisible.value = false;
    emit('refresh');
  } catch (e: unknown) {
    // v11 批次 180 P2-1 修复：catch (e: any) 改为 catch (e: unknown) + 类型守卫
    const errMsg = e instanceof Error ? e.message : String(e);
    ElMessage.error(errMsg || t('common.qualityCheck.reportFailed'));
  } finally {
    submitting.value = false;
  }
}

function handleResolve(row: QualityIssue) {
  currentIssue.value = row;
  resolveForm.value = { resolution: '' };
  resolveVisible.value = true;
}

async function handleResolveSubmit() {
  if (!resolveForm.value.resolution) {
    ElMessage.warning(t('common.qualityCheck.pleaseInputResolution'));
    return;
  }
  if (!currentIssue.value) return;
  submitting.value = true;
  try {
    await resolveQualityIssue(currentIssue.value.id, {
      resolution: resolveForm.value.resolution,
      operator_id: 1,
    });
    ElMessage.success(t('common.qualityCheck.resolveSuccess'));
    resolveVisible.value = false;
    emit('refresh');
  } catch (e: unknown) {
    // v11 批次 180 P2-1 修复：catch (e: any) 改为 catch (e: unknown) + 类型守卫
    const errMsg = e instanceof Error ? e.message : String(e);
    ElMessage.error(errMsg || t('common.qualityCheck.resolveFailed'));
  } finally {
    submitting.value = false;
  }
}
</script>

<style scoped>
.quality-check {
  padding: 8px 0;
}
.action-bar {
  margin-bottom: 12px;
}
</style>
