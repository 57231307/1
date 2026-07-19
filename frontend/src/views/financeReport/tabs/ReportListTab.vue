<!--
  ReportListTab.vue - 财务报表 Tab
  来源：原 financeReport/index.vue 主体内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="report-list-tab">
    <div class="page-header">
      <h2 class="page-title">财务报表</h2>
      <div>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>导出
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryForm" aria-label="财务报表筛选表单">
        <el-form-item label="报表类型">
          <el-select
            v-model="queryForm.report_type"
            placeholder="选择报表类型"
            style="width: 180px"
          >
            <el-option label="资产负债表" value="balance_sheet" />
            <el-option label="利润表" value="income_statement" />
            <el-option label="现金流量表" value="cash_flow" />
            <el-option label="科目余额表" value="trial_balance" />
            <el-option label="总分类账" value="general_ledger" />
            <el-option label="明细分类账" value="subsidiary_ledger" />
          </el-select>
        </el-form-item>
        <el-form-item label="会计期间">
          <el-date-picker
            v-model="queryForm.period"
            type="month"
            placeholder="选择月份"
            value-format="YYYY-MM"
            style="width: 160px"
          />
        </el-form-item>
        <el-form-item v-if="queryForm.report_type === 'general_ledger'" label="科目编码">
          <el-input v-model="queryForm.subject_code" placeholder="如 1001" style="width: 140px" />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleGenerate">生成报表</el-button>
          <el-button @click="handleReset">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card v-loading="loading" shadow="hover" class="report-card">
      <template #header>
        <div class="card-header">
          <span>{{ getReportTypeLabel(queryForm.report_type) }} - {{ queryForm.period }}</span>
          <div>
            <el-button type="success" link @click="handlePrint">
              <el-icon><Printer /></el-icon>打印
            </el-button>
            <el-button type="primary" link @click="handleExport">
              <el-icon><Download /></el-icon>导出
            </el-button>
          </div>
        </div>
      </template>

      <el-empty v-if="!reportData" description="请选择报表类型与会计期间后点击生成报表" />
      <div v-else class="report-content">
        <div class="report-summary">
          <span>期间：{{ reportData.period_name || reportData.period }}</span>
          <span v-if="reportData.total != null">合计：¥{{ reportData.total.toFixed(2) }}</span>
        </div>
        <el-table :data="reportData.items || []" stripe border aria-label="财务报表列表">
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

const loading = ref(false)
const reportData = ref<ReportData | null>(null)

const queryForm = reactive({
  report_type: 'balance_sheet',
  period: new Date().toISOString().slice(0, 7),
  subject_code: '',
})

const getReportTypeLabel = (type: string) => {
  const map: Record<string, string> = {
    balance_sheet: '资产负债表',
    income_statement: '利润表',
    cash_flow: '现金流量表',
    trial_balance: '科目余额表',
    general_ledger: '总分类账',
    subsidiary_ledger: '明细分类账',
  }
  return map[type] || '报表'
}

const reportColumns = computed(() => {
  if (!reportData.value?.items?.length) {
    return []
  }
  const item = reportData.value.items[0]
  const cols: {
    key: string
    label: string
    align?: 'left' | 'right' | 'center'
    width?: number
    formatter?: (val: unknown) => string
  }[] = []
  if ('code' in item) cols.push({ key: 'code', label: '编码', width: 100 })
  if ('name' in item) cols.push({ key: 'name', label: '名称', width: 200 })
  if ('level' in item) cols.push({ key: 'level', label: '级次', width: 80, align: 'center' })
  if ('debit_amount' in item) {
    cols.push({
      key: 'debit_amount',
      label: '借方金额',
      align: 'right',
      width: 140,
      formatter: formatAmount,
    })
  }
  if ('credit_amount' in item) {
    cols.push({
      key: 'credit_amount',
      label: '贷方金额',
      align: 'right',
      width: 140,
      formatter: formatAmount,
    })
  }
  if ('balance' in item) {
    cols.push({
      key: 'balance',
      label: '余额',
      align: 'right',
      width: 140,
      formatter: formatAmount,
    })
  }
  if ('amount' in item) {
    cols.push({ key: 'amount', label: '金额', align: 'right', width: 140, formatter: formatAmount })
  }
  if ('inflow' in item) {
    cols.push({ key: 'inflow', label: '流入', align: 'right', width: 140, formatter: formatAmount })
  }
  if ('outflow' in item) {
    cols.push({
      key: 'outflow',
      label: '流出',
      align: 'right',
      width: 140,
      formatter: formatAmount,
    })
  }
  if ('net_flow' in item) {
    cols.push({
      key: 'net_flow',
      label: '净流量',
      align: 'right',
      width: 140,
      formatter: formatAmount,
    })
  }
  if ('date' in item) cols.push({ key: 'date', label: '日期', width: 120 })
  if ('voucher_no' in item) cols.push({ key: 'voucher_no', label: '凭证号', width: 120 })
  if ('summary' in item) cols.push({ key: 'summary', label: '摘要', width: 200 })
  if ('direction' in item) {
    cols.push({
      key: 'direction',
      label: '方向',
      width: 80,
      align: 'center',
      formatter: (v: unknown) => (v === 'debit' ? '借' : '贷'),
    })
  }
  return cols
})

const formatAmount = (val: unknown) => {
  const num = Number(val) || 0
  return `¥${num.toFixed(2)}`
}

const handleGenerate = async () => {
  if (!queryForm.period) {
    ElMessage.warning('请选择会计期间')
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
          ElMessage.warning('请输入科目编码')
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
      ElMessage.info('该期间暂无报表数据')
    }
  } catch (e) {
    const err = e as Error
    logger.error('生成报表失败', err)
    ElMessage.error(err.message || '生成报表失败')
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
    ElMessage.warning('请先生成报表')
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
