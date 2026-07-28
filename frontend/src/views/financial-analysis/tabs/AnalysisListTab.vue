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
          <span>{{ t('financialAnalysis.analysisListTab.cardTitle') }}</span>
          <el-button type="primary" size="small" @click="openCreateDialog">
            <el-icon><Plus /></el-icon>{{ t('financialAnalysis.analysisListTab.buttonCreate') }}
          </el-button>
        </div>
      </template>
      <el-table
        v-loading="loading"
        :data="reports"
        stripe
        :aria-label="t('financialAnalysis.analysisListTab.ariaLabelList')"
      >
        <el-table-column
          prop="reportName"
          :label="t('financialAnalysis.analysisListTab.columnReportName')"
          min-width="180"
        />
        <el-table-column
          prop="reportType"
          :label="t('financialAnalysis.analysisListTab.columnType')"
          width="120"
        >
          <template #default="{ row }">
            <el-tag size="small">{{ getReportTypeLabel(row.reportType) }}</el-tag>
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
            <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="executedAt"
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
        <el-form-item
          :label="t('financialAnalysis.analysisListTab.labelReportName')"
          prop="reportName"
        >
          <el-input
            v-model="form.reportName"
            :placeholder="t('financialAnalysis.analysisListTab.placeholderReportName')"
          />
        </el-form-item>
        <el-form-item
          :label="t('financialAnalysis.analysisListTab.labelFormReportType')"
          prop="reportType"
        >
          <el-select
            v-model="form.reportType"
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
  type FinancialReport,
} from '@/api/financial-analysis';
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

const form = reactive<Partial<FinancialReport>>({
  id: undefined,
  reportName: '',
  reportType: 'profitability',
  period: new Date().toISOString().slice(0, 7),
});

const rules: FormRules = {
  reportName: [
    {
      required: true,
      message: t('financialAnalysis.analysisListTab.validateReportNameRequired'),
      trigger: 'blur',
    },
  ],
  reportType: [
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

/** 报表状态 → i18n 标签（语言切换响应） */
const getStatusLabel = (status?: string) => {
  switch (status) {
    case 'draft':
      return t('financialAnalysis.analysisListTab.statusDraft');
    case 'executed':
      return t('financialAnalysis.analysisListTab.statusExecuted');
    case 'failed':
      return t('financialAnalysis.analysisListTab.statusFailed');
    default:
      return status || '-';
  }
};

const getStatusType = (status?: string) => {
  const map: Record<string, string> = {
    draft: 'info',
    executed: 'success',
    failed: 'danger',
  };
  return map[status || ''] || 'info';
};

const fetchReports = async () => {
  loading.value = true;
  try {
    const res = await getReportList(queryForm);
    const d = (res as { data?: unknown }).data as
      | {
          list?: FinancialReport[];
          items?: FinancialReport[];
          data?: FinancialReport[];
          total?: number;
        }
      | FinancialReport[];
    if (Array.isArray(d)) {
      reports.value = d;
    } else {
      reports.value = d?.list || d?.items || [];
    }
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
  form.reportName = '';
  form.reportType = 'profitability';
  form.period = new Date().toISOString().slice(0, 7);
  dialogVisible.value = true;
};

const editReport = (row: FinancialReport) => {
  Object.assign(form, row);
  dialogVisible.value = true;
};

const handleSubmit = async () => {
  if (!formRef.value) return;
  await formRef.value.validate(async valid => {
    if (!valid) return;
    submitLoading.value = true;
    try {
      if (form.id) {
        await updateReport(form.id, form);
        ElMessage.success(t('financialAnalysis.analysisListTab.messageUpdateSuccess'));
      } else {
        await createReport(form);
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

// 批次 157b P1-1 修复：展示报表详情（无独立 getReport API，使用行数据展示）
const viewReport = async (row: FinancialReport) => {
  const lines = [
    t('financialAnalysis.analysisListTab.detailReportName', { value: row.reportName || '-' }),
    t('financialAnalysis.analysisListTab.detailReportType', {
      value: getReportTypeLabel(row.reportType),
    }),
    t('financialAnalysis.analysisListTab.detailPeriod', { value: row.period || '-' }),
    t('financialAnalysis.analysisListTab.detailStatus', { value: getStatusLabel(row.status) }),
    t('financialAnalysis.analysisListTab.detailExecutedAt', { value: row.executedAt || '-' }),
    t('financialAnalysis.analysisListTab.detailCreatedAt', { value: row.createdAt || '-' }),
    t('financialAnalysis.analysisListTab.detailUpdatedAt', { value: row.updatedAt || '-' }),
  ];
  await ElMessageBox.alert(
    lines.join('\n'),
    t('financialAnalysis.analysisListTab.dialogTitleDetail'),
    {
      confirmButtonText: t('financialAnalysis.analysisListTab.buttonClose'),
    }
  );
};

const deleteReport = async (row: FinancialReport) => {
  if (row.id === undefined) return;
  try {
    await ElMessageBox.confirm(
      t('financialAnalysis.analysisListTab.confirmDeleteMessage', { name: row.reportName }),
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
