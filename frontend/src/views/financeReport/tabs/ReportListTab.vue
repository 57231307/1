<!--
  ReportListTab.vue - 财务报表 Tab
  来源：原 financeReport/index.vue 主体内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="report-list-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('financeReport.reportListTab.pageTitle') }}</h2>
      <div>
        <el-button v-permission="'finance_report.export'" @click="handleExport">
          <el-icon><Download /></el-icon>{{ t('financeReport.reportListTab.buttonExport') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form
        :inline="true"
        :model="queryForm"
        :aria-label="t('financeReport.reportListTab.filterAriaLabel')"
      >
        <el-form-item :label="t('financeReport.reportListTab.labelReportType')">
          <el-select
            v-model="queryForm.report_type"
            :placeholder="t('financeReport.reportListTab.placeholderReportType')"
            style="width: 180px"
          >
            <el-option
              :label="t('financeReport.reportListTab.optionBalanceSheet')"
              value="balance_sheet"
            />
            <el-option
              :label="t('financeReport.reportListTab.optionIncomeStatement')"
              value="income_statement"
            />
            <el-option :label="t('financeReport.reportListTab.optionCashFlow')" value="cash_flow" />
            <el-option
              :label="t('financeReport.reportListTab.optionTrialBalance')"
              value="trial_balance"
            />
            <el-option
              :label="t('financeReport.reportListTab.optionGeneralLedger')"
              value="general_ledger"
            />
            <el-option
              :label="t('financeReport.reportListTab.optionSubsidiaryLedger')"
              value="subsidiary_ledger"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('financeReport.reportListTab.labelPeriod')">
          <el-date-picker
            v-model="queryForm.period"
            type="month"
            :placeholder="t('financeReport.reportListTab.placeholderPeriod')"
            value-format="YYYY-MM"
            style="width: 160px"
          />
        </el-form-item>
        <el-form-item
          v-if="queryForm.report_type === 'general_ledger'"
          :label="t('financeReport.reportListTab.labelSubjectCode')"
        >
          <el-input
            v-model="queryForm.subject_code"
            :placeholder="t('financeReport.reportListTab.placeholderSubjectCode')"
            style="width: 140px"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleGenerate">{{
            t('financeReport.reportListTab.buttonGenerate')
          }}</el-button>
          <el-button @click="handleReset">{{
            t('financeReport.reportListTab.buttonReset')
          }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card v-loading="loading" shadow="hover" class="report-card">
      <template #header>
        <div class="card-header">
          <span>{{ getReportTypeLabel(queryForm.report_type) }} - {{ queryForm.period }}</span>
          <div>
            <el-button v-permission="'finance_report.print'" type="success" link @click="handlePrint">
              <el-icon><Printer /></el-icon>{{ t('financeReport.reportListTab.buttonPrint') }}
            </el-button>
            <el-button v-permission="'finance_report.export'" type="primary" link @click="handleExport">
              <el-icon><Download /></el-icon>{{ t('financeReport.reportListTab.buttonExport') }}
            </el-button>
          </div>
        </div>
      </template>

      <el-empty
        v-if="!reportData"
        :description="t('financeReport.reportListTab.emptyDescription')"
      />
      <div v-else class="report-content">
        <div class="report-summary">
          <span
            >{{ t('financeReport.reportListTab.labelPeriodPrefix')
            }}{{ reportData.period_name || reportData.period }}</span
          >
          <span v-if="reportData.total != null"
            >{{ t('financeReport.reportListTab.labelTotalPrefix') }}¥{{
              reportData.total.toFixed(2)
            }}</span
          >
        </div>
        <el-table
          :data="reportData.items || []"
          stripe
          border
          :aria-label="t('financeReport.reportListTab.tableAriaLabel')"
        >
          <el-table-column
            v-for="col in reportColumns"
            :key="col.key"
            :prop="col.key"
            :label="col.label"
            :align="col.align || 'left'"
            :width="col.width"
          >
            <template v-if="col.formatter" #default="{ row }">
              {{ col.formatter(row[col.key]) }}
            </template>
          </el-table-column>
        </el-table>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { Download, Printer } from '@element-plus/icons-vue'
import {
  getBalanceSheet,
  getProfitStatement,
  getCashFlowStatement,
  getTrialBalance,
  getGeneralLedger,
  getSubsidiaryLedger,
  type ReportData,
} from '@/api/financeReport'
import { logger } from '@/utils/logger'
import { exportToExcel } from '@/utils/export'

const { t } = useI18n({ useScope: 'global' })

const loading = ref(false)
const reportData = ref<ReportData | null>(null)

const queryForm = reactive({
  report_type: 'balance_sheet',
  period: new Date().toISOString().slice(0, 7),
  subject_code: '',
})

/** 报表类型 → 国际化标签（语言切换时响应式刷新） */
const getReportTypeLabel = (type: string) => {
  const map: Record<string, string> = {
    balance_sheet: t('financeReport.reportListTab.optionBalanceSheet'),
    income_statement: t('financeReport.reportListTab.optionIncomeStatement'),
    cash_flow: t('financeReport.reportListTab.optionCashFlow'),
    trial_balance: t('financeReport.reportListTab.optionTrialBalance'),
    general_ledger: t('financeReport.reportListTab.optionGeneralLedger'),
    subsidiary_ledger: t('financeReport.reportListTab.optionSubsidiaryLedger'),
  }
  return map[type] || t('financeReport.reportListTab.labelReport')
}

type ColAlign = 'left' | 'right' | 'center'
interface ColDef {
  key: string
  label: string
  width: number
  align?: ColAlign
  formatter?: (v: unknown) => string
}

/** 金额格式化（¥ 前缀 + 2 位小数） */
const formatAmount = (val: unknown) => {
  const num = Number(val) || 0
  return `¥${num.toFixed(2)}`
}

/** 借贷方向格式化 */
const formatDirection = (v: unknown) =>
  v === 'debit'
    ? t('financeReport.reportListTab.directionDebit')
    : t('financeReport.reportListTab.directionCredit')

/** 根据首行数据字段动态构建列定义（≤50 行） */
const buildReportColumns = (item: Record<string, unknown>): ColDef[] => {
  const cols: ColDef[] = []
  const add = (
    key: string,
    label: string,
    width: number,
    align?: ColAlign,
    formatter?: (v: unknown) => string
  ) => {
    if (key in item) cols.push({ key, label, width, align, formatter })
  }
  add('code', t('financeReport.reportListTab.colCode'), 100)
  add('name', t('financeReport.reportListTab.colName'), 200)
  add('level', t('financeReport.reportListTab.colLevel'), 80, 'center')
  add('debit_amount', t('financeReport.reportListTab.colDebitAmount'), 140, 'right', formatAmount)
  add('credit_amount', t('financeReport.reportListTab.colCreditAmount'), 140, 'right', formatAmount)
  add('balance', t('financeReport.reportListTab.colBalance'), 140, 'right', formatAmount)
  add('amount', t('financeReport.reportListTab.colAmount'), 140, 'right', formatAmount)
  add('inflow', t('financeReport.reportListTab.colInflow'), 140, 'right', formatAmount)
  add('outflow', t('financeReport.reportListTab.colOutflow'), 140, 'right', formatAmount)
  add('net_flow', t('financeReport.reportListTab.colNetFlow'), 140, 'right', formatAmount)
  add('date', t('financeReport.reportListTab.colDate'), 120)
  add('voucher_no', t('financeReport.reportListTab.colVoucherNo'), 120)
  add('summary', t('financeReport.reportListTab.colSummary'), 200)
  add('direction', t('financeReport.reportListTab.colDirection'), 80, 'center', formatDirection)
  return cols
}

const reportColumns = computed(() => {
  const items = reportData.value?.items
  if (!items?.length) return []
  return buildReportColumns(items[0] as Record<string, unknown>)
})

const handleGenerate = async () => {
  if (!queryForm.period) {
    ElMessage.warning(t('financeReport.reportListTab.messageSelectPeriod'))
    return
  }
  loading.value = true
  try {
    let res: { data?: ReportData }
    const params = { period: queryForm.period }
    switch (queryForm.report_type) {
      case 'balance_sheet':
        res = await getBalanceSheet(params)
        break
      case 'income_statement':
        res = await getProfitStatement(params)
        break
      case 'cash_flow':
        res = await getCashFlowStatement(params)
        break
      case 'trial_balance':
        res = await getTrialBalance(params)
        break
      case 'general_ledger':
        if (!queryForm.subject_code) {
          ElMessage.warning(t('financeReport.reportListTab.messageInputSubjectCode'))
          loading.value = false
          return
        }
        res = await getGeneralLedger(queryForm.subject_code, params)
        break
      case 'subsidiary_ledger':
        res = await getSubsidiaryLedger(undefined, undefined, params)
        break
      default:
        res = { data: undefined }
    }
    reportData.value = res?.data || null
    if (!reportData.value) {
      ElMessage.info(t('financeReport.reportListTab.messageNoData'))
    }
  } catch (e) {
    const err = e as Error
    logger.error(t('financeReport.reportListTab.messageGenerateFailed'), err)
    ElMessage.error(err.message || t('financeReport.reportListTab.messageGenerateFailed'))
  } finally {
    loading.value = false
  }
}

const handleReset = () => {
  queryForm.report_type = 'balance_sheet'
  queryForm.period = new Date().toISOString().slice(0, 7)
  queryForm.subject_code = ''
  reportData.value = null
}

const handlePrint = () => {
  window.print()
}

const handleExport = () => {
  if (!reportData.value?.items?.length) {
    ElMessage.warning(t('financeReport.reportListTab.messageGenerateFirst'))
    return
  }
  const items = reportData.value.items
  const cols = reportColumns.value
  exportToExcel({
    filename: `${getReportTypeLabel(queryForm.report_type)}_${queryForm.period}`,
    format: 'excel',
    data: items.map((item): Record<string, unknown> => ({ ...(item as Record<string, unknown>) })),
    columns: cols.map(c => ({
      key: c.key,
      title: c.label,
    })),
  })
}
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.report-summary {
  display: flex;
  gap: 24px;
  margin-bottom: 16px;
  padding: 12px 16px;
  background: #f5f7fa;
  border-radius: 4px;
  font-weight: 500;
}
.report-content {
  padding: 8px 0;
}
</style>
