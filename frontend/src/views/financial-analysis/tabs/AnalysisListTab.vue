<!--
  AnalysisListTab.vue - 财务分析 Tab
  来源：原 financial-analysis/index.vue 主体内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="analysis-list-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('financialAnalysis.analysisListTab.pageTitle') }}</h2>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form
        :inline="true"
        :model="queryForm"
        :aria-label="t('financialAnalysis.analysisListTab.ariaLabelFilterForm')"
      >
        <el-form-item :label="t('financialAnalysis.analysisListTab.labelReportType')">
          <el-select
            v-model="queryForm.reportType"
            :placeholder="t('financialAnalysis.analysisListTab.placeholderReportType')"
            style="width: 180px"
          >
            <el-option
              :label="t('financialAnalysis.analysisListTab.optionProfitability')"
              value="profitability"
            />
            <el-option
              :label="t('financialAnalysis.analysisListTab.optionSolvency')"
              value="solvency"
            />
            <el-option
              :label="t('financialAnalysis.analysisListTab.optionOperation')"
              value="operation"
            />
            <el-option
              :label="t('financialAnalysis.analysisListTab.optionDevelopment')"
              value="development"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('financialAnalysis.analysisListTab.labelPeriod')">
          <el-date-picker
            v-model="queryForm.period"
            type="month"
            :placeholder="t('financialAnalysis.analysisListTab.placeholderMonth')"
            value-format="YYYY-MM"
            style="width: 160px"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleAnalyze">{{
            t('financialAnalysis.analysisListTab.buttonAnalyze')
          }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover">
      <template #header>
        <div class="card-header">
          <div>
            <span>{{ t('financialAnalysis.analysisListTab.cardTitle') }}</span>
            <el-button type="primary" size="small" @click="openCreateDialog">
              <el-icon><Plus /></el-icon>{{ t('financialAnalysis.analysisListTab.buttonCreate') }}
            </el-button>
            <el-button type="warning" size="small" plain @click="indicatorVisible = true">
              新增指标
            </el-button>
            <el-button
              type="info"
              size="small"
              plain
              @click="
                () => {
                  trendVisible = true;
                  handleTrendQuery();
                }
              "
            >
              财务趋势
            </el-button>
          </div>
        </div>
      </template>
      <el-table
        v-loading="loading"
        :data="reports"
        stripe
        :aria-label="t('financialAnalysis.analysisListTab.ariaLabelList')"
      >
        <el-table-column
          prop="name"
          :label="t('financialAnalysis.analysisListTab.columnReportName')"
          min-width="180"
        />
        <el-table-column
          prop="indicator_type"
          :label="t('financialAnalysis.analysisListTab.columnType')"
          width="120"
        >
          <template #default="{ row }">
            <el-tag size="small">{{ getReportTypeLabel(row.indicator_type) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="period"
          :label="t('financialAnalysis.analysisListTab.columnPeriod')"
          width="120"
        />
        <el-table-column
          prop="status"
          :label="t('financialAnalysis.analysisListTab.columnStatus')"
          width="100"
        >
          <template #default="{ row }">
            <el-tag :type="financialStatusTagType(row.status)">
              {{ t(financialStatusLabelKey(row.status)) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="executed_at"
          :label="t('financialAnalysis.analysisListTab.columnExecutedAt')"
          width="180"
        />
        <el-table-column
          :label="t('financialAnalysis.analysisListTab.columnActions')"
          width="240"
          fixed="right"
        >
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="executeReport(row)">{{
              t('financialAnalysis.analysisListTab.buttonExecute')
            }}</el-button>
            <el-button type="warning" link size="small" @click="openParamExec(row)">{{
              t('financialAnalysis.analysisListTab.buttonParamExec')
            }}</el-button>
            <el-button type="success" link size="small" @click="viewReport(row)">{{
              t('financialAnalysis.analysisListTab.buttonView')
            }}</el-button>
            <el-button
              v-permission="'financial_report:update'"
              type="warning"
              link
              size="small"
              @click="editReport(row)"
              >{{ t('financialAnalysis.analysisListTab.buttonEdit') }}</el-button
            >
            <el-button
              v-permission="'financial_report:delete'"
              type="danger"
              link
              size="small"
              @click="deleteReport(row)"
              >{{ t('financialAnalysis.analysisListTab.buttonDelete') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog
      v-model="dialogVisible"
      :title="
        form.id
          ? t('financialAnalysis.analysisListTab.dialogTitleEdit')
          : t('financialAnalysis.analysisListTab.dialogTitleCreate')
      "
      width="500px"
      :aria-label="
        form.id
          ? t('financialAnalysis.analysisListTab.ariaLabelDialogEdit')
          : t('financialAnalysis.analysisListTab.ariaLabelDialogCreate')
      "
    >
      <el-form
        ref="formRef"
        :model="form"
        :rules="rules"
        label-width="100px"
        :aria-label="t('financialAnalysis.analysisListTab.ariaLabelForm')"
      >
        <el-form-item :label="t('financialAnalysis.analysisListTab.labelReportName')" prop="name">
          <el-input
            v-model="form.name"
            :placeholder="t('financialAnalysis.analysisListTab.placeholderReportName')"
          />
        </el-form-item>
        <el-form-item
          :label="t('financialAnalysis.analysisListTab.labelFormReportType')"
          prop="report_type"
        >
          <el-select
            v-model="form.report_type"
            :placeholder="t('financialAnalysis.analysisListTab.placeholderSelectType')"
            style="width: 100%"
          >
            <el-option
              :label="t('financialAnalysis.analysisListTab.optionProfitability')"
              value="profitability"
            />
            <el-option
              :label="t('financialAnalysis.analysisListTab.optionSolvency')"
              value="solvency"
            />
            <el-option
              :label="t('financialAnalysis.analysisListTab.optionOperation')"
              value="operation"
            />
            <el-option
              :label="t('financialAnalysis.analysisListTab.optionDevelopment')"
              value="development"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('financialAnalysis.analysisListTab.labelFormPeriod')">
          <el-date-picker
            v-model="form.period"
            type="month"
            :placeholder="t('financialAnalysis.analysisListTab.placeholderMonth')"
            value-format="YYYY-MM"
            style="width: 100%"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{
          t('financialAnalysis.analysisListTab.buttonCancel')
        }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
          t('financialAnalysis.analysisListTab.buttonConfirm')
        }}</el-button>
      </template>
    </el-dialog>

    <!-- 带参执行对话框（executeReportWithParams） -->
    <el-dialog v-model="paramExecVisible" title="带参数执行报表" width="480">
      <el-form label-width="90px">
        <el-form-item label="报表">
          <el-input :model-value="paramExecRow?.name || ''" disabled />
        </el-form-item>
        <el-form-item :label="t('financialAnalysis.analysisListTab.labelPeriod')">
          <!-- 后端仅读 query.period（YYYY-MM）；此前的自由 JSON 参数框发过去会被整体丢弃 -->
          <el-date-picker
            v-model="paramExecPeriod"
            type="month"
            value-format="YYYY-MM"
            :clearable="true"
            style="width: 100%"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="paramExecVisible = false">取消</el-button>
        <el-button type="primary" :loading="paramExecSaving" @click="handleParamExec">
          执行
        </el-button>
      </template>
    </el-dialog>

    <!-- 新增财务指标（createFinancialIndicator） -->
    <el-dialog v-model="indicatorVisible" title="新增财务指标" width="520">
      <el-form :model="indicatorForm" label-width="100px">
        <el-form-item label="指标名称" required>
          <el-input v-model="indicatorForm.name" />
        </el-form-item>
        <el-form-item label="计算公式" required>
          <el-input v-model="indicatorForm.formula" type="textarea" :rows="2" />
        </el-form-item>
        <el-form-item label="分类">
          <el-input v-model="indicatorForm.indicator_type" placeholder="如：ratio" />
        </el-form-item>
        <el-form-item label="单位">
          <el-input v-model="indicatorForm.unit" placeholder="如：% / 元" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="indicatorVisible = false">取消</el-button>
        <el-button type="primary" :loading="indicatorSaving" @click="handleSaveIndicator">
          保存
        </el-button>
      </template>
    </el-dialog>

    <!-- 财务趋势查询（getFinancialTrends） -->
    <el-dialog v-model="trendVisible" title="财务趋势查询" width="640">
      <div class="toolbar" style="margin-bottom: 8px">
        <el-input v-model="trendForm.indicator_id" placeholder="指标ID" style="width: 150px" />
        <el-input v-model="trendForm.start_date" placeholder="开始日期" style="width: 140px" />
        <el-input v-model="trendForm.end_date" placeholder="结束日期" style="width: 140px" />
        <el-button type="primary" :loading="trendLoading" @click="handleTrendQuery">
          查询
        </el-button>
      </div>
      <el-table v-if="trendRows.length" :data="trendRows" border size="small" max-height="300">
        <el-table-column
          v-for="col in trendCols"
          :key="col"
          :prop="col"
          :label="col"
          min-width="120"
          show-overflow-tooltip
        />
      </el-table>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import {
  getReportList,
  createReport,
  updateReport,
  deleteReport as deleteReportApi,
  executeFinancialReport,
  getFinancialReport,
  executeReportWithParams,
  createFinancialIndicator,
  getFinancialTrends,
  type FinancialReport,
  type CreateReportRequest,
} from '@/api/financial-analysis';
import { financialStatusLabelKey, financialStatusTagType } from '@/utils/financial-status';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

const loading = ref(false);
const submitLoading = ref(false);
const dialogVisible = ref(false);
const reports = ref<FinancialReport[]>([]);
const formRef = ref<FormInstance>();

const queryForm = reactive({
  reportType: '',
  period: new Date().toISOString().slice(0, 7),
});

const form = reactive({
  id: undefined as number | undefined,
  name: '',
  report_type: 'profitability',
  period: new Date().toISOString().slice(0, 7),
});

const rules: FormRules = {
  name: [
    {
      required: true,
      message: t('financialAnalysis.analysisListTab.validateReportNameRequired'),
      trigger: 'blur',
    },
  ],
  report_type: [
    {
      required: true,
      message: t('financialAnalysis.analysisListTab.validateReportTypeRequired'),
      trigger: 'change',
    },
  ],
};

/** 报表类型 → i18n 标签（语言切换响应） */
const getReportTypeLabel = (type?: string) => {
  switch (type) {
    case 'profitability':
      return t('financialAnalysis.analysisListTab.optionProfitability');
    case 'solvency':
      return t('financialAnalysis.analysisListTab.optionSolvency');
    case 'operation':
      return t('financialAnalysis.analysisListTab.optionOperation');
    case 'development':
      return t('financialAnalysis.analysisListTab.optionDevelopment');
    default:
      return type || '-';
  }
};

const fetchReports = async () => {
  loading.value = true;
  try {
    const res = await getReportList({ page: 1, page_size: 100 });
    reports.value = res.data.items;
  } catch (e) {
    const err = e as Error;
    logger.error(t('financialAnalysis.analysisListTab.logFetchFailed'), err);
    ElMessage.error(err.message || t('financialAnalysis.analysisListTab.messageFetchFailed'));
  } finally {
    loading.value = false;
  }
};

const handleAnalyze = () => {
  fetchReports();
};

const openCreateDialog = () => {
  form.id = undefined;
  form.name = '';
  form.report_type = 'profitability';
  form.period = new Date().toISOString().slice(0, 7);
  dialogVisible.value = true;
};

const editReport = (row: FinancialReport) => {
  form.id = row.id;
  form.name = row.name;
  form.report_type = row.indicator_type;
  form.period = row.period || new Date().toISOString().slice(0, 7);
  dialogVisible.value = true;
};

const handleSubmit = async () => {
  if (!formRef.value) return;
  await formRef.value.validate(async valid => {
    if (!valid) return;
    submitLoading.value = true;
    try {
      if (form.id) {
        await updateReport(form.id, { name: form.name, report_type: form.report_type });
        ElMessage.success(t('financialAnalysis.analysisListTab.messageUpdateSuccess'));
      } else {
        const payload: CreateReportRequest = {
          name: form.name,
          report_type: form.report_type,
          period_start: `${form.period}-01`,
          period_end: `${form.period}-01`,
        };
        await createReport(payload);
        ElMessage.success(t('financialAnalysis.analysisListTab.messageCreateSuccess'));
      }
      dialogVisible.value = false;
      fetchReports();
    } catch (e) {
      const err = e as Error;
      ElMessage.error(err.message || t('financialAnalysis.analysisListTab.messageOperationFailed'));
    } finally {
      submitLoading.value = false;
    }
  });
};

const executeReport = async (row: FinancialReport) => {
  if (row.id === undefined) return;
  try {
    await executeFinancialReport(row.id);
    ElMessage.success(t('financialAnalysis.analysisListTab.messageExecuteSuccess'));
    fetchReports();
  } catch (e) {
    const err = e as Error;
    ElMessage.error(err.message || t('financialAnalysis.analysisListTab.messageExecuteFailed'));
  }
};

// ===== 带参执行（executeReportWithParams） =====
const paramExecVisible = ref(false);
const paramExecSaving = ref(false);
const paramExecRow = ref<FinancialReport | null>(null);
const paramExecPeriod = ref('');

const openParamExec = (row: FinancialReport) => {
  paramExecRow.value = row;
  paramExecPeriod.value = '';
  paramExecVisible.value = true;
};

const handleParamExec = async () => {
  if (!paramExecRow.value?.id) return;
  paramExecSaving.value = true;
  try {
    await executeReportWithParams(paramExecRow.value.id, paramExecPeriod.value || undefined);
    ElMessage.success(t('financialAnalysis.analysisListTab.messageExecuteSuccess'));
    paramExecVisible.value = false;
    fetchReports();
  } catch (e) {
    ElMessage.error(
      (e as Error).message || t('financialAnalysis.analysisListTab.messageExecuteFailed')
    );
  } finally {
    paramExecSaving.value = false;
  }
};

// ===== 新增财务指标（createFinancialIndicator） =====
const indicatorVisible = ref(false);
const indicatorSaving = ref(false);
const indicatorForm = reactive({
  name: '',
  formula: '',
  indicator_type: '',
  unit: '',
});

const handleSaveIndicator = async () => {
  if (!indicatorForm.name || !indicatorForm.formula) {
    ElMessage.warning('请填写指标名称与计算公式');
    return;
  }
  indicatorSaving.value = true;
  try {
    await createFinancialIndicator({
      name: indicatorForm.name,
      formula: indicatorForm.formula,
      indicator_type: indicatorForm.indicator_type || undefined,
      unit: indicatorForm.unit || undefined,
    });
    ElMessage.success(t('common.success'));
    indicatorVisible.value = false;
  } catch (e) {
    ElMessage.error((e as Error).message || t('common.failed'));
  } finally {
    indicatorSaving.value = false;
  }
};

// ===== 财务趋势查询（getFinancialTrends） =====
const trendVisible = ref(false);
const trendLoading = ref(false);
const trendRows = ref<Array<Record<string, unknown>>>([]);
const trendCols = ref<string[]>([]);
const trendForm = reactive({ indicator_id: '', start_date: '', end_date: '' });

const handleTrendQuery = async () => {
  const indicatorId = Number(trendForm.indicator_id);
  if (!indicatorId) {
    trendRows.value = [];
    trendCols.value = [];
    return;
  }
  trendLoading.value = true;
  try {
    const res = await getFinancialTrends({
      indicator_id: indicatorId,
      start_date: trendForm.start_date || undefined,
      end_date: trendForm.end_date || undefined,
    });
    const trends = res.data.items;
    trendRows.value = trends as unknown as Array<Record<string, unknown>>;
    trendCols.value = trendRows.value.length ? Object.keys(trendRows.value[0]).slice(0, 8) : [];
  } catch (e) {
    ElMessage.error((e as Error).message || t('common.failed'));
  } finally {
    trendLoading.value = false;
  }
};

// 批次 157b P1-1 修复：展示报表详情（现接入 getFinancialReport 按 ID 回源最新数据）
const viewReport = async (row: FinancialReport) => {
  if (row.id === undefined) return;
  try {
    const res = await getFinancialReport(row.id);
    if (res.data) {
      const detail = res.data;
      const lines = [
        t('financialAnalysis.analysisListTab.detailReportName', {
          value: detail.name || '-',
        }),
        t('financialAnalysis.analysisListTab.detailReportType', {
          value: getReportTypeLabel(detail.indicator_type),
        }),
        t('financialAnalysis.analysisListTab.detailPeriod', { value: detail.period || '-' }),
        t('financialAnalysis.analysisListTab.detailStatus', {
          value: t(financialStatusLabelKey(detail.status)),
        }),
        t('financialAnalysis.analysisListTab.detailExecutedAt', {
          value: detail.executed_at || '-',
        }),
        t('financialAnalysis.analysisListTab.detailCreatedAt', { value: detail.created_at || '-' }),
        t('financialAnalysis.analysisListTab.detailUpdatedAt', { value: detail.updated_at || '-' }),
      ];
      await ElMessageBox.alert(
        lines.join('\n'),
        t('financialAnalysis.analysisListTab.dialogTitleDetail'),
        {
          confirmButtonText: t('financialAnalysis.analysisListTab.buttonClose'),
        }
      );
    }
  } catch (err) {
    ElMessage.error(
      (err as Error).message || t('financialAnalysis.analysisListTab.messageFetchFailed')
    );
  }
};

const deleteReport = async (row: FinancialReport) => {
  if (row.id === undefined) return;
  try {
    await ElMessageBox.confirm(
      t('financialAnalysis.analysisListTab.confirmDeleteMessage', { name: row.name }),
      t('financialAnalysis.analysisListTab.dialogTitleDeleteConfirm'),
      { type: 'warning' }
    );
    await deleteReportApi(row.id);
    ElMessage.success(t('financialAnalysis.analysisListTab.messageDeleteSuccess'));
    fetchReports();
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error;
      ElMessage.error(err.message || t('financialAnalysis.analysisListTab.messageDeleteFailed'));
    }
  }
};

onMounted(() => {
  fetchReports();
});
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
