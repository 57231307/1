<script setup lang="ts">
import { ref } from 'vue'
import { ElTabs, ElTabPane, ElTable, ElTableColumn, ElButton, ElSelect, ElRow, ElCol, ElMessage, ElCard } from 'element-plus'
import { Refresh } from '@element-plus/icons-vue'
import { getBalanceSheet, getProfitStatement, getCashFlowStatement, getTrialBalance, type BalanceSheetItem, type ProfitStatementItem, type CashFlowItem, type ReportData } from '@/api/financeReport'

const activeTab = ref('balance')
const loading = ref(false)
const selectedPeriod = ref('')
const periods = ref<{ label: string; value: string }[]>([])

const balanceSheetData = ref<ReportData>({ period: '', period_name: '', items: [] })
const profitStatementData = ref<ReportData>({ period: '', period_name: '', items: [] })
const cashFlowData = ref<ReportData>({ period: '', period_name: '', items: [] })
const trialBalanceData = ref<ReportData>({ period: '', period_name: '', items: [] })

const selectedYear = ref(new Date().getFullYear())
const selectedMonth = ref(new Date().getMonth() + 1)

const loadPeriods = () => {
  const result: { label: string; value: string }[] = []
  for (let i = 0; i < 12; i++) {
    const date = new Date()
    date.setMonth(date.getMonth() - i)
    const year = date.getFullYear()
    const month = date.getMonth() + 1
    result.push({
      label: `${year}年${month}月`,
      value: `${year}-${month.toString().padStart(2, '0')}`
    })
  }
  periods.value = result
  selectedPeriod.value = result[0].value
}

const loadBalanceSheet = async () => {
  loading.value = true
  try {
    const res: any = await getBalanceSheet({ year: selectedYear.value, month: selectedMonth.value })
    balanceSheetData.value = res.data
  } catch (error) {
    ElMessage.error('加载资产负债表失败')
  } finally {
    loading.value = false
  }
}

const loadProfitStatement = async () => {
  loading.value = true
  try {
    const res: any = await getProfitStatement({ year: selectedYear.value, month: selectedMonth.value })
    profitStatementData.value = res.data
  } catch (error) {
    ElMessage.error('加载利润表失败')
  } finally {
    loading.value = false
  }
}

const loadCashFlow = async () => {
  loading.value = true
  try {
    const res: any = await getCashFlowStatement({ year: selectedYear.value, month: selectedMonth.value })
    cashFlowData.value = res.data
  } catch (error) {
    ElMessage.error('加载现金流量表失败')
  } finally {
    loading.value = false
  }
}

const loadTrialBalance = async () => {
  loading.value = true
  try {
    const res: any = await getTrialBalance({ year: selectedYear.value, month: selectedMonth.value })
    trialBalanceData.value = res.data
  } catch (error) {
    ElMessage.error('加载试算平衡表失败')
  } finally {
    loading.value = false
  }
}

const formatAmount = (amount: number) => {
  return amount.toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 2 })
}

const handleRefresh = () => {
  switch (activeTab.value) {
    case 'balance':
      loadBalanceSheet()
      break
    case 'profit':
      loadProfitStatement()
      break
    case 'cash':
      loadCashFlow()
      break
    case 'trial':
      loadTrialBalance()
      break
  }
}

const handlePeriodChange = () => {
  const [year, month] = selectedPeriod.value.split('-').map(Number)
  selectedYear.value = year
  selectedMonth.value = month
  handleRefresh()
}

loadPeriods()
loadBalanceSheet()
</script>

<template>
  <div class="app-container">
    <div class="filter-bar">
      <ElRow :gutter="20" align="middle">
        <ElCol :span="4">
          <ElSelect
            v-model="selectedPeriod"
            placeholder="选择期间"
            @change="handlePeriodChange"
          >
            <ElOption v-for="p in periods" :key="p.value" :label="p.label" :value="p.value" />
          </ElSelect>
        </ElCol>
        <ElCol :span="12" />
        <ElCol :span="8" class="filter-right">
          <ElButton type="primary" @click="handleRefresh"><Refresh /> 刷新数据</ElButton>
        </ElCol>
      </ElRow>
    </div>

    <ElTabs v-model="activeTab" type="card" @tab-change="handleRefresh">
      <ElTabPane label="资产负债表" name="balance">
        <ElCard title="资产负债表" class="report-card">
          <div class="report-header">
            <span class="report-title">资产负债表</span>
            <span class="report-period">{{ balanceSheetData.period_name }}</span>
          </div>
          <ElTable :data="balanceSheetData.items" border :loading="loading" style="width: 100%">
            <ElTableColumn prop="name" label="项目" width="300">
              <template #default="scope">
                <span :style="{ paddingLeft: `${(scope.row.level - 1) * 20}px` }">
                  {{ scope.row.name }}
                </span>
              </template>
            </ElTableColumn>
            <ElTableColumn prop="code" label="科目编码" width="120" />
            <ElTableColumn label="期初余额" width="180" align="right">
              <template #default="scope">
                {{ formatAmount((scope.row as BalanceSheetItem).balance || 0) }}
              </template>
            </ElTableColumn>
            <ElTableColumn label="期末余额" width="180" align="right">
              <template #default="scope">
                {{ formatAmount((scope.row as BalanceSheetItem).debit_amount || 0) }}
              </template>
            </ElTableColumn>
          </ElTable>
          <div class="report-summary">
            <div class="summary-item">
              <span class="label">资产总计:</span>
              <span class="value">{{ formatAmount(balanceSheetData.total || 0) }}</span>
            </div>
          </div>
        </ElCard>
      </ElTabPane>

      <ElTabPane label="利润表" name="profit">
        <ElCard title="利润表" class="report-card">
          <div class="report-header">
            <span class="report-title">利润表</span>
            <span class="report-period">{{ profitStatementData.period_name }}</span>
          </div>
          <ElTable :data="profitStatementData.items" border :loading="loading" style="width: 100%">
            <ElTableColumn prop="name" label="项目" width="300">
              <template #default="scope">
                <span :style="{ paddingLeft: `${(scope.row.level - 1) * 20}px` }">
                  {{ scope.row.name }}
                </span>
              </template>
            </ElTableColumn>
            <ElTableColumn prop="code" label="科目编码" width="120" />
            <ElTableColumn label="本期金额" width="200" align="right">
              <template #default="scope">
                {{ formatAmount((scope.row as ProfitStatementItem).amount || 0) }}
              </template>
            </ElTableColumn>
            <ElTableColumn label="累计金额" width="200" align="right">
              <template #default="scope">
                {{ formatAmount((scope.row as ProfitStatementItem).amount || 0) }}
              </template>
            </ElTableColumn>
          </ElTable>
          <div class="report-summary">
            <div class="summary-item">
              <span class="label">净利润:</span>
              <span class="value profit">{{ formatAmount(profitStatementData.total || 0) }}</span>
            </div>
          </div>
        </ElCard>
      </ElTabPane>

      <ElTabPane label="现金流量表" name="cash">
        <ElCard title="现金流量表" class="report-card">
          <div class="report-header">
            <span class="report-title">现金流量表</span>
            <span class="report-period">{{ cashFlowData.period_name }}</span>
          </div>
          <ElTable :data="cashFlowData.items" border :loading="loading" style="width: 100%">
            <ElTableColumn prop="name" label="项目" width="350">
              <template #default="scope">
                <span :style="{ paddingLeft: `${(scope.row.level - 1) * 20}px` }">
                  {{ scope.row.name }}
                </span>
              </template>
            </ElTableColumn>
            <ElTableColumn prop="code" label="科目编码" width="120" />
            <ElTableColumn label="现金流入" width="180" align="right">
              <template #default="scope">
                {{ formatAmount((scope.row as CashFlowItem).inflow || 0) }}
              </template>
            </ElTableColumn>
            <ElTableColumn label="现金流出" width="180" align="right">
              <template #default="scope">
                {{ formatAmount((scope.row as CashFlowItem).outflow || 0) }}
              </template>
            </ElTableColumn>
            <ElTableColumn label="净流量" width="180" align="right">
              <template #default="scope">
                <span :class="{ 'positive': (scope.row as CashFlowItem).net_flow >= 0, 'negative': (scope.row as CashFlowItem).net_flow < 0 }">
                  {{ formatAmount((scope.row as CashFlowItem).net_flow || 0) }}
                </span>
              </template>
            </ElTableColumn>
          </ElTable>
          <div class="report-summary">
            <div class="summary-item">
              <span class="label">现金净增加额:</span>
              <span :class="['value', cashFlowData.total && cashFlowData.total >= 0 ? 'positive' : 'negative']">
                {{ formatAmount(cashFlowData.total || 0) }}
              </span>
            </div>
          </div>
        </ElCard>
      </ElTabPane>

      <ElTabPane label="试算平衡表" name="trial">
        <ElCard title="试算平衡表" class="report-card">
          <div class="report-header">
            <span class="report-title">试算平衡表</span>
            <span class="report-period">{{ trialBalanceData.period_name }}</span>
          </div>
          <ElTable :data="trialBalanceData.items" border :loading="loading" style="width: 100%">
            <ElTableColumn prop="code" label="科目编码" width="120" />
            <ElTableColumn prop="name" label="科目名称" width="250" />
            <ElTableColumn label="期初借方" width="150" align="right">
              <template #default="scope">{{ formatAmount(scope.row.begin_debit || 0) }}</template>
            </ElTableColumn>
            <ElTableColumn label="期初贷方" width="150" align="right">
              <template #default="scope">{{ formatAmount(scope.row.begin_credit || 0) }}</template>
            </ElTableColumn>
            <ElTableColumn label="本期借方" width="150" align="right">
              <template #default="scope">{{ formatAmount(scope.row.debit_amount || 0) }}</template>
            </ElTableColumn>
            <ElTableColumn label="本期贷方" width="150" align="right">
              <template #default="scope">{{ formatAmount(scope.row.credit_amount || 0) }}</template>
            </ElTableColumn>
            <ElTableColumn label="期末借方" width="150" align="right">
              <template #default="scope">{{ formatAmount(scope.row.end_debit || 0) }}</template>
            </ElTableColumn>
            <ElTableColumn label="期末贷方" width="150" align="right">
              <template #default="scope">{{ formatAmount(scope.row.end_credit || 0) }}</template>
            </ElTableColumn>
          </ElTable>
        </ElCard>
      </ElTabPane>
    </ElTabs>
  </div>
</template>

<style scoped>
.app-container {
  padding: 20px;
}

.filter-bar {
  margin-bottom: 20px;
  padding: 15px;
  background: #fff;
  border-radius: 8px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.08);
}

.filter-right {
  text-align: right;
}

.report-card {
  margin-bottom: 20px;
}

.report-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 15px;
  padding-bottom: 15px;
  border-bottom: 1px solid #ebeef5;
}

.report-title {
  font-size: 18px;
  font-weight: bold;
}

.report-period {
  color: #909399;
}

.report-summary {
  display: flex;
  justify-content: flex-end;
  margin-top: 20px;
  padding-top: 15px;
  border-top: 2px solid #ebeef5;
}

.summary-item {
  margin-left: 30px;
}

.summary-item .label {
  margin-right: 10px;
  font-weight: bold;
}

.summary-item .value {
  font-weight: bold;
  font-size: 18px;
  color: #1f2937;
}

.summary-item .value.profit {
  color: #67c23a;
}

.summary-item .value.positive {
  color: #67c23a;
}

.summary-item .value.negative {
  color: #f56c6c;
}

:deep(.el-table .positive) {
  color: #67c23a;
}

:deep(.el-table .negative) {
  color: #f56c6c;
}
</style>