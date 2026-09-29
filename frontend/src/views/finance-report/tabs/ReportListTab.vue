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
            <el-button
              v-permission="'finance_report.print'"
              type="success"
              link
              @click="handlePrint"
            >
              <el-icon><Printer /></el-icon>{{ t('financeReport.reportListTab.buttonPrint') }}
            </el-button>
            <el-button
              v-permission="'finance_report.export'"
              type="primary"
              link
              @click="handleExport"
            >
              <el-icon><Download /></el-icon>{{ t('financeReport.reportListTab.buttonExport') }}
            </el-button>
          </div>
        </div>
      </template>

      <el-empty v-if="!hasData" :description="t('financeReport.reportListTab.emptyDescription')" />

      <!-- 资产负债表：资产/负债/所有者权益三组明细 + 合计数 -->
      <div v-else-if="balanceSheet" class="report-content">
        <div class="report-summary">
          <span
            >{{ t('financeReport.reportListTab.labelReportDate')
            }}{{ balanceSheet.report_date }}</span
          >
        </div>
        <div class="section-block">
          <h3 class="section-title">{{ t('financeReport.reportListTab.sectionAssets') }}</h3>
          <el-table :data="balanceSheet.assets" stripe border>
            <el-table-column
              :label="t('financeReport.reportListTab.colName')"
              prop="name"
              min-width="160"
            />
            <el-table-column
              :label="t('financeReport.reportListTab.colDescription')"
              prop="description"
              min-width="160"
            />
            <el-table-column
              :label="t('financeReport.reportListTab.colAmount')"
              prop="amount"
              align="right"
              width="160"
            >
              <template #default="{ row }">{{ formatAmount(row.amount) }}</template>
            </el-table-column>
          </el-table>
          <div class="total-line">
            {{ t('financeReport.reportListTab.labelTotalAssets') }}
            {{ formatAmount(balanceSheet.total_assets) }}
          </div>
        </div>
        <div class="section-block">
          <h3 class="section-title">{{ t('financeReport.reportListTab.sectionLiabilities') }}</h3>
          <el-table :data="balanceSheet.liabilities" stripe border>
            <el-table-column
              :label="t('financeReport.reportListTab.colName')"
              prop="name"
              min-width="160"
            />
            <el-table-column
              :label="t('financeReport.reportListTab.colDescription')"
              prop="description"
              min-width="160"
            />
            <el-table-column
              :label="t('financeReport.reportListTab.colAmount')"
              prop="amount"
              align="right"
              width="160"
            >
              <template #default="{ row }">{{ formatAmount(row.amount) }}</template>
            </el-table-column>
          </el-table>
          <div class="total-line">
            {{ t('financeReport.reportListTab.labelTotalLiabilities') }}
            {{ formatAmount(balanceSheet.total_liabilities) }}
          </div>
        </div>
        <div class="section-block">
          <h3 class="section-title">{{ t('financeReport.reportListTab.sectionEquity') }}</h3>
          <el-table :data="balanceSheet.equity" stripe border>
            <el-table-column
              :label="t('financeReport.reportListTab.colName')"
              prop="name"
              min-width="160"
            />
            <el-table-column
              :label="t('financeReport.reportListTab.colDescription')"
              prop="description"
              min-width="160"
            />
            <el-table-column
              :label="t('financeReport.reportListTab.colAmount')"
              prop="amount"
              align="right"
              width="160"
            >
              <template #default="{ row }">{{ formatAmount(row.amount) }}</template>
            </el-table-column>
          </el-table>
          <div class="total-line">
            {{ t('financeReport.reportListTab.labelTotalEquity') }}
            {{ formatAmount(balanceSheet.total_equity) }}
          </div>
        </div>
      </div>

      <!-- 利润表 -->
      <div v-else-if="incomeStatement" class="report-content">
        <div class="report-summary">
          <span
            >{{ t('financeReport.reportListTab.labelPeriodRange')
            }}{{ incomeStatement.period_start }} ~ {{ incomeStatement.period_end }}</span
          >
        </div>
        <div class="section-block">
          <h3 class="section-title">{{ t('financeReport.reportListTab.sectionRevenue') }}</h3>
          <el-table :data="incomeStatement.revenue" stripe border>
            <el-table-column
              :label="t('financeReport.reportListTab.colName')"
              prop="name"
              min-width="160"
            />
            <el-table-column
              :label="t('financeReport.reportListTab.colDescription')"
              prop="description"
              min-width="160"
            />
            <el-table-column
              :label="t('financeReport.reportListTab.colAmount')"
              prop="amount"
              align="right"
              width="160"
            >
              <template #default="{ row }">{{ formatAmount(row.amount) }}</template>
            </el-table-column>
          </el-table>
        </div>
        <div class="section-block">
          <h3 class="section-title">
            {{ t('financeReport.reportListTab.sectionOperatingExpenses') }}
          </h3>
          <el-table :data="incomeStatement.operating_expenses" stripe border>
            <el-table-column
              :label="t('financeReport.reportListTab.colName')"
              prop="name"
              min-width="160"
            />
            <el-table-column
              :label="t('financeReport.reportListTab.colDescription')"
              prop="description"
              min-width="160"
            />
            <el-table-column
              :label="t('financeReport.reportListTab.colAmount')"
              prop="amount"
              align="right"
              width="160"
            >
              <template #default="{ row }">{{ formatAmount(row.amount) }}</template>
            </el-table-column>
          </el-table>
        </div>
        <el-descriptions :column="2" border class="metrics">
          <el-descriptions-item :label="t('financeReport.reportListTab.labelTotalRevenue')">{{
            formatAmount(incomeStatement.total_revenue)
          }}</el-descriptions-item>
          <el-descriptions-item :label="t('financeReport.reportListTab.labelCostOfGoodsSold')">{{
            formatAmount(incomeStatement.cost_of_goods_sold)
          }}</el-descriptions-item>
          <el-descriptions-item :label="t('financeReport.reportListTab.labelGrossProfit')">{{
            formatAmount(incomeStatement.gross_profit)
          }}</el-descriptions-item>
          <el-descriptions-item
            :label="t('financeReport.reportListTab.labelTotalOperatingExpenses')"
            >{{ formatAmount(incomeStatement.total_operating_expenses) }}</el-descriptions-item
          >
          <el-descriptions-item :label="t('financeReport.reportListTab.labelOperatingIncome')">{{
            formatAmount(incomeStatement.operating_income)
          }}</el-descriptions-item>
          <el-descriptions-item :label="t('financeReport.reportListTab.labelOtherIncome')">{{
            formatAmount(incomeStatement.other_income)
          }}</el-descriptions-item>
          <el-descriptions-item :label="t('financeReport.reportListTab.labelOtherExpenses')">{{
            formatAmount(incomeStatement.other_expenses)
          }}</el-descriptions-item>
          <el-descriptions-item :label="t('financeReport.reportListTab.labelNetIncome')">{{
            formatAmount(incomeStatement.net_income)
          }}</el-descriptions-item>
        </el-descriptions>
      </div>

      <!-- 现金流量表 -->
      <div v-else-if="cashFlow" class="report-content">
        <div class="report-summary">
          <span
            >{{ t('financeReport.reportListTab.labelPeriodRange') }}{{ cashFlow.period_start }} ~
            {{ cashFlow.period_end }}</span
          >
        </div>
        <div class="section-block">
          <h3 class="section-title">
            {{ t('financeReport.reportListTab.sectionOperatingActivities') }}
          </h3>
          <el-table :data="cashFlow.operating_activities" stripe border>
            <el-table-column
              :label="t('financeReport.reportListTab.colName')"
              prop="name"
              min-width="160"
            />
            <el-table-column
              :label="t('financeReport.reportListTab.colDescription')"
              prop="description"
              min-width="160"
            />
            <el-table-column
              :label="t('financeReport.reportListTab.colAmount')"
              prop="amount"
              align="right"
              width="160"
            >
              <template #default="{ row }">{{ formatAmount(row.amount) }}</template>
            </el-table-column>
          </el-table>
        </div>
        <div class="section-block">
          <h3 class="section-title">
            {{ t('financeReport.reportListTab.sectionInvestingActivities') }}
          </h3>
          <el-table :data="cashFlow.investing_activities" stripe border>
            <el-table-column
              :label="t('financeReport.reportListTab.colName')"
              prop="name"
              min-width="160"
            />
            <el-table-column
              :label="t('financeReport.reportListTab.colDescription')"
              prop="description"
              min-width="160"
            />
            <el-table-column
              :label="t('financeReport.reportListTab.colAmount')"
              prop="amount"
              align="right"
              width="160"
            >
              <template #default="{ row }">{{ formatAmount(row.amount) }}</template>
            </el-table-column>
          </el-table>
        </div>
        <div class="section-block">
          <h3 class="section-title">
            {{ t('financeReport.reportListTab.sectionFinancingActivities') }}
          </h3>
          <el-table :data="cashFlow.financing_activities" stripe border>
            <el-table-column
              :label="t('financeReport.reportListTab.colName')"
              prop="name"
              min-width="160"
            />
            <el-table-column
              :label="t('financeReport.reportListTab.colDescription')"
              prop="description"
              min-width="160"
            />
            <el-table-column
              :label="t('financeReport.reportListTab.colAmount')"
              prop="amount"
              align="right"
              width="160"
            >
              <template #default="{ row }">{{ formatAmount(row.amount) }}</template>
            </el-table-column>
          </el-table>
        </div>
        <el-descriptions :column="2" border class="metrics">
          <el-descriptions-item :label="t('financeReport.reportListTab.labelNetCashOperations')">{{
            formatAmount(cashFlow.net_cash_from_operations)
          }}</el-descriptions-item>
          <el-descriptions-item :label="t('financeReport.reportListTab.labelNetCashInvesting')">{{
            formatAmount(cashFlow.net_cash_from_investing)
          }}</el-descriptions-item>
          <el-descriptions-item :label="t('financeReport.reportListTab.labelNetCashFinancing')">{{
            formatAmount(cashFlow.net_cash_from_financing)
          }}</el-descriptions-item>
          <el-descriptions-item :label="t('financeReport.reportListTab.labelNetChangeInCash')">{{
            formatAmount(cashFlow.net_change_in_cash)
          }}</el-descriptions-item>
          <el-descriptions-item :label="t('financeReport.reportListTab.labelBeginningCash')">{{
            formatAmount(cashFlow.beginning_cash)
          }}</el-descriptions-item>
          <el-descriptions-item :label="t('financeReport.reportListTab.labelEndingCash')">{{
            formatAmount(cashFlow.ending_cash)
          }}</el-descriptions-item>
        </el-descriptions>
      </div>

      <!-- 试算平衡表（科目余额表） -->
      <div v-else-if="trialBalance" class="report-content">
        <div class="report-summary">
          <span
            >{{ t('financeReport.reportListTab.labelPeriodPrefix') }}{{ trialBalance.period }}</span
          >
        </div>
        <el-table :data="trialBalance.entries" stripe border>
          <el-table-column
            :label="t('financeReport.reportListTab.colSubjectCode')"
            prop="subject_code"
            width="120"
          />
          <el-table-column
            :label="t('financeReport.reportListTab.colSubjectName')"
            prop="subject_name"
            min-width="160"
          />
          <el-table-column
            :label="t('financeReport.reportListTab.colLevel')"
            prop="level"
            align="center"
            width="80"
          />
          <el-table-column
            :label="t('financeReport.reportListTab.colInitialDebit')"
            prop="initial_debit"
            align="right"
            width="140"
          >
            <template #default="{ row }">{{ formatAmount(row.initial_debit) }}</template>
          </el-table-column>
          <el-table-column
            :label="t('financeReport.reportListTab.colInitialCredit')"
            prop="initial_credit"
            align="right"
            width="140"
          >
            <template #default="{ row }">{{ formatAmount(row.initial_credit) }}</template>
          </el-table-column>
          <el-table-column
            :label="t('financeReport.reportListTab.colPeriodDebit')"
            prop="period_debit"
            align="right"
            width="140"
          >
            <template #default="{ row }">{{ formatAmount(row.period_debit) }}</template>
          </el-table-column>
          <el-table-column
            :label="t('financeReport.reportListTab.colPeriodCredit')"
            prop="period_credit"
            align="right"
            width="140"
          >
            <template #default="{ row }">{{ formatAmount(row.period_credit) }}</template>
          </el-table-column>
          <el-table-column
            :label="t('financeReport.reportListTab.colEndingDebit')"
            prop="ending_debit"
            align="right"
            width="140"
          >
            <template #default="{ row }">{{ formatAmount(row.ending_debit) }}</template>
          </el-table-column>
          <el-table-column
            :label="t('financeReport.reportListTab.colEndingCredit')"
            prop="ending_credit"
            align="right"
            width="140"
          >
            <template #default="{ row }">{{ formatAmount(row.ending_credit) }}</template>
          </el-table-column>
        </el-table>
        <el-descriptions :column="3" border class="metrics">
          <el-descriptions-item :label="t('financeReport.reportListTab.labelTotalInitialDebit')">{{
            formatAmount(trialBalance.total_initial_debit)
          }}</el-descriptions-item>
          <el-descriptions-item :label="t('financeReport.reportListTab.labelTotalInitialCredit')">{{
            formatAmount(trialBalance.total_initial_credit)
          }}</el-descriptions-item>
          <el-descriptions-item :label="t('financeReport.reportListTab.labelPeriodBalance')">{{
            formatBalanceCheck(trialBalance.total_period_debit, trialBalance.total_period_credit)
          }}</el-descriptions-item>
        </el-descriptions>
      </div>

      <!-- 总分类账 -->
      <div v-else-if="generalLedger" class="report-content">
        <div class="report-summary">
          <span
            >{{ t('financeReport.reportListTab.labelSubject') }}{{ generalLedger.subject_code }}
            {{ generalLedger.subject_name }}</span
          >
          <span
            >{{ t('financeReport.reportListTab.labelPeriodRange')
            }}{{ generalLedger.period_start }} ~ {{ generalLedger.period_end }}</span
          >
        </div>
        <el-table :data="generalLedger.entries" stripe border>
          <el-table-column
            :label="t('financeReport.reportListTab.colDate')"
            prop="voucher_date"
            width="120"
          />
          <el-table-column
            :label="t('financeReport.reportListTab.colVoucherNo')"
            prop="voucher_no"
            width="140"
          />
          <el-table-column
            :label="t('financeReport.reportListTab.colLineNo')"
            prop="line_no"
            align="center"
            width="80"
          />
          <el-table-column
            :label="t('financeReport.reportListTab.colSummary')"
            prop="summary"
            min-width="160"
          />
          <el-table-column
            :label="t('financeReport.reportListTab.colDebitAmount')"
            prop="debit"
            align="right"
            width="140"
          >
            <template #default="{ row }">{{ formatAmount(row.debit) }}</template>
          </el-table-column>
          <el-table-column
            :label="t('financeReport.reportListTab.colCreditAmount')"
            prop="credit"
            align="right"
            width="140"
          >
            <template #default="{ row }">{{ formatAmount(row.credit) }}</template>
          </el-table-column>
          <el-table-column
            :label="t('financeReport.reportListTab.colDirection')"
            prop="direction"
            align="center"
            width="80"
          >
            <template #default="{ row }">{{ formatDirection(row.direction) }}</template>
          </el-table-column>
          <el-table-column
            :label="t('financeReport.reportListTab.colBalance')"
            prop="balance"
            align="right"
            width="140"
          >
            <template #default="{ row }">{{ formatAmount(row.balance) }}</template>
          </el-table-column>
        </el-table>
        <el-descriptions :column="2" border class="metrics">
          <el-descriptions-item :label="t('financeReport.reportListTab.labelOpeningBalance')">{{
            formatAmount(generalLedger.opening_balance)
          }}</el-descriptions-item>
          <el-descriptions-item :label="t('financeReport.reportListTab.labelClosingBalance')">{{
            formatAmount(generalLedger.closing_balance)
          }}</el-descriptions-item>
          <el-descriptions-item :label="t('financeReport.reportListTab.labelTotalDebit')">{{
            formatAmount(generalLedger.total_debit)
          }}</el-descriptions-item>
          <el-descriptions-item :label="t('financeReport.reportListTab.labelTotalCredit')">{{
            formatAmount(generalLedger.total_credit)
          }}</el-descriptions-item>
        </el-descriptions>
      </div>

      <!-- 明细分类账 -->
      <div v-else-if="subsidiaryLedger" class="report-content">
        <div class="report-summary">
          <span
            >{{ t('financeReport.reportListTab.labelDimensionType')
            }}{{ subsidiaryLedger.dimension_type }}</span
          >
          <span
            >{{ t('financeReport.reportListTab.labelDimensionValue')
            }}{{ subsidiaryLedger.dimension_value }}</span
          >
          <span
            >{{ t('financeReport.reportListTab.labelPeriodRange')
            }}{{ subsidiaryLedger.period_start }} ~ {{ subsidiaryLedger.period_end }}</span
          >
        </div>
        <el-table :data="subsidiaryLedger.entries" stripe border>
          <el-table-column
            :label="t('financeReport.reportListTab.colBusinessDate')"
            prop="business_date"
            width="120"
          />
          <el-table-column
            :label="t('financeReport.reportListTab.colBusinessNo')"
            prop="business_no"
            width="160"
          />
          <el-table-column
            :label="t('financeReport.reportListTab.colBusinessType')"
            prop="business_type"
            width="120"
          />
          <el-table-column
            :label="t('financeReport.reportListTab.colSubjectCode')"
            prop="subject_code"
            width="120"
          />
          <el-table-column
            :label="t('financeReport.reportListTab.colSubjectName')"
            prop="subject_name"
            min-width="140"
          />
          <el-table-column
            :label="t('financeReport.reportListTab.colSummary')"
            prop="summary"
            min-width="160"
          />
          <el-table-column
            :label="t('financeReport.reportListTab.colDebitAmount')"
            prop="debit"
            align="right"
            width="140"
          >
            <template #default="{ row }">{{ formatAmount(row.debit) }}</template>
          </el-table-column>
          <el-table-column
            :label="t('financeReport.reportListTab.colCreditAmount')"
            prop="credit"
            align="right"
            width="140"
          >
            <template #default="{ row }">{{ formatAmount(row.credit) }}</template>
          </el-table-column>
        </el-table>
        <el-descriptions :column="2" border class="metrics">
          <el-descriptions-item :label="t('financeReport.reportListTab.labelTotalDebit')">{{
            formatAmount(subsidiaryLedger.total_debit)
          }}</el-descriptions-item>
          <el-descriptions-item :label="t('financeReport.reportListTab.labelTotalCredit')">{{
            formatAmount(subsidiaryLedger.total_credit)
          }}</el-descriptions-item>
        </el-descriptions>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { Download, Printer } from '@element-plus/icons-vue';
import {
  getBalanceSheet,
  getProfitStatement,
  getCashFlowStatement,
  getTrialBalance,
  getGeneralLedger,
  getSubsidiaryLedger,
  type BalanceSheet,
  type IncomeStatement,
  type CashFlowStatement,
  type TrialBalance,
  type GeneralLedger,
  type SubsidiaryLedger,
} from '@/api/finance-report';
import { logger } from '@/utils/logger';
import { exportFromBackend } from '@/utils/export';

const { t } = useI18n({ useScope: 'global' });

const loading = ref(false);

// 每种报表的后端载荷是各自独立的单对象 DTO，分别持有，切换报表类型时清空其余。
const balanceSheet = ref<BalanceSheet | null>(null);
const incomeStatement = ref<IncomeStatement | null>(null);
const cashFlow = ref<CashFlowStatement | null>(null);
const trialBalance = ref<TrialBalance | null>(null);
const generalLedger = ref<GeneralLedger | null>(null);
const subsidiaryLedger = ref<SubsidiaryLedger | null>(null);

const hasData = computed(
  () =>
    !!balanceSheet.value ||
    !!incomeStatement.value ||
    !!cashFlow.value ||
    !!trialBalance.value ||
    !!generalLedger.value ||
    !!subsidiaryLedger.value
);

const queryForm = reactive({
  report_type: 'balance_sheet',
  period: new Date().toISOString().slice(0, 7),
  subject_code: '',
});

/** 报表类型 → 国际化标签（语言切换时响应式刷新） */
const getReportTypeLabel = (type: string) => {
  const map: Record<string, string> = {
    balance_sheet: t('financeReport.reportListTab.optionBalanceSheet'),
    income_statement: t('financeReport.reportListTab.optionIncomeStatement'),
    cash_flow: t('financeReport.reportListTab.optionCashFlow'),
    trial_balance: t('financeReport.reportListTab.optionTrialBalance'),
    general_ledger: t('financeReport.reportListTab.optionGeneralLedger'),
    subsidiary_ledger: t('financeReport.reportListTab.optionSubsidiaryLedger'),
  };
  return map[type] || t('financeReport.reportListTab.labelReport');
};

/** 金额格式化（¥ 前缀 + 2 位小数；后端 Decimal 序列化为字符串时按数值归一） */
const formatAmount = (val: number | string | null | undefined) => {
  const num = Number(val);
  return `¥${(Number.isFinite(num) ? num : 0).toFixed(2)}`;
};

/** 借贷平衡校验展示 */
const formatBalanceCheck = (debit: number, credit: number) => {
  const balanced = Number(debit) === Number(credit);
  return balanced
    ? t('financeReport.reportListTab.balanceBalanced')
    : t('financeReport.reportListTab.balanceUnbalanced');
};

/** 借贷方向格式化（后端写入大写 token DEBIT/CREDIT，比较点逐字符对齐） */
const formatDirection = (v: string) => {
  if (v === 'DEBIT') return t('financeReport.reportListTab.directionDebit');
  if (v === 'CREDIT') return t('financeReport.reportListTab.directionCredit');
  return v;
};

const clearReports = () => {
  balanceSheet.value = null;
  incomeStatement.value = null;
  cashFlow.value = null;
  trialBalance.value = null;
  generalLedger.value = null;
  subsidiaryLedger.value = null;
};

const handleGenerate = async () => {
  if (!queryForm.period) {
    ElMessage.warning(t('financeReport.reportListTab.messageSelectPeriod'));
    return;
  }
  loading.value = true;
  clearReports();
  try {
    const params = { period: queryForm.period };
    switch (queryForm.report_type) {
      case 'balance_sheet': {
        const res = await getBalanceSheet();
        balanceSheet.value = res.data;
        break;
      }
      case 'income_statement': {
        const res = await getProfitStatement(params);
        incomeStatement.value = res.data;
        break;
      }
      case 'cash_flow': {
        const res = await getCashFlowStatement(params);
        cashFlow.value = res.data;
        break;
      }
      case 'trial_balance': {
        const res = await getTrialBalance(params);
        trialBalance.value = res.data;
        break;
      }
      case 'general_ledger': {
        if (!queryForm.subject_code) {
          ElMessage.warning(t('financeReport.reportListTab.messageInputSubjectCode'));
          loading.value = false;
          return;
        }
        const res = await getGeneralLedger(queryForm.subject_code, params);
        generalLedger.value = res.data;
        break;
      }
      case 'subsidiary_ledger': {
        const res = await getSubsidiaryLedger(undefined, undefined, params);
        subsidiaryLedger.value = res.data;
        break;
      }
      default:
        break;
    }
    if (!hasData.value) {
      ElMessage.info(t('financeReport.reportListTab.messageNoData'));
    }
  } catch (e) {
    const err = e as Error;
    logger.error(t('financeReport.reportListTab.messageGenerateFailed'), err);
    ElMessage.error(err.message || t('financeReport.reportListTab.messageGenerateFailed'));
  } finally {
    loading.value = false;
  }
};

const handleReset = () => {
  queryForm.report_type = 'balance_sheet';
  queryForm.period = new Date().toISOString().slice(0, 7);
  queryForm.subject_code = '';
  clearReports();
};

const handlePrint = () => {
  window.print();
};

const handleExport = () => {
  if (!hasData.value) {
    ElMessage.warning(t('financeReport.reportListTab.messageGenerateFirst'));
    return;
  }
  const reportType = queryForm.report_type;
  const filename = `${getReportTypeLabel(reportType)}_${queryForm.period}`;

  // 所有报表类型统一使用后端导出
  const exportApiMap: Record<string, string> = {
    trial_balance: '/finance/reports/trial-balance/export',
    balance_sheet: '/finance/reports/balance-sheet/export',
    income_statement: '/finance/reports/income-statement/export',
    cash_flow: '/finance/reports/cash-flow/export',
    general_ledger: '/finance/reports/general-ledger/export',
    subsidiary_ledger: '/finance/reports/subsidiary-ledger/export',
  };

  const apiPath = exportApiMap[reportType];
  if (apiPath) {
    const params: Record<string, string> = {};
    if (queryForm.period) params.period = queryForm.period;
    if (reportType === 'general_ledger' || reportType === 'subsidiary_ledger') {
      // 总账和明细账需要额外参数
      if (queryForm.subject_code) params.subject_code = queryForm.subject_code;
    }
    exportFromBackend(apiPath, params, filename);
  } else {
    ElMessage.warning(t('financeReport.reportListTab.unsupportedExportType'));
  }
};
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
.section-block {
  margin-bottom: 20px;
}
.section-title {
  margin: 0 0 8px;
  font-size: 15px;
  font-weight: 600;
  color: #303133;
}
.total-line {
  margin-top: 8px;
  text-align: right;
  font-weight: 600;
  color: #303133;
}
.metrics {
  margin-top: 12px;
}
</style>
