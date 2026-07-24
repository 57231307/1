<!--
  FundTab.vue - 资金账户 Tab
  来源：原 ar/index.vue 中 资金账户 tab 内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="fund-tab">
    <div class="page-header">
      <h2 class="page-title">{{ $t('arModule.fund.title') }}</h2>
      <el-button type="primary" @click="openFundDialog()">
        <el-icon><Plus /></el-icon>
        {{ $t('arModule.fund.create') }}
      </el-button>
    </div>

    <el-row :gutter="20" class="fund-summary">
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="summary-item">
            <div class="summary-label">{{ $t('arModule.fund.totalBalance') }}</div>
            <div class="summary-value">{{ formatMoney(totalBalance) }}</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="summary-item">
            <div class="summary-label">{{ $t('arModule.fund.totalFrozen') }}</div>
            <div class="summary-value text-orange">{{ formatMoney(totalFrozen) }}</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="summary-item">
            <div class="summary-label">{{ $t('arModule.fund.totalAvailable') }}</div>
            <div class="summary-value text-green">{{ formatMoney(totalAvailable) }}</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="summary-item">
            <div class="summary-label">{{ $t('arModule.fund.accountCount') }}</div>
            <div class="summary-value">{{ funds.length }}</div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover" class="mt-20">
      <el-table v-loading="fundLoading" :data="funds" stripe :aria-label="$t('arModule.fund.listAria')">
        <el-table-column prop="account_code" :label="$t('arModule.fund.accountCode')" width="120" />
        <el-table-column prop="account_name" :label="$t('arModule.fund.accountName')" min-width="150" />
        <el-table-column prop="account_type" :label="$t('arModule.fund.accountType')" width="100">
          <template #default="{ row }">
            {{ getAccountTypeLabel(row.account_type) }}
          </template>
        </el-table-column>
        <el-table-column :label="$t('arModule.fund.balance')" width="140" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.balance) }}
          </template>
        </el-table-column>
        <el-table-column :label="$t('arModule.fund.frozenBalance')" width="120" align="right">
          <template #default="{ row }">
            <span class="text-orange">{{ formatMoney(row.frozen_balance) }}</span>
          </template>
        </el-table-column>
        <el-table-column :label="$t('arModule.fund.availableBalance')" width="140" align="right">
          <template #default="{ row }">
            <span class="text-green">{{ formatMoney(row.available_balance) }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="bank_name" :label="$t('arModule.fund.bankName')" width="150" />
        <el-table-column prop="bank_account" :label="$t('arModule.fund.bankAccount')" width="180" />
        <el-table-column prop="status" :label="$t('common.status')" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{ row.status === 'active' ? $t('arModule.fund.statusActive') : $t('arModule.fund.statusFrozen') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="$t('common.operation')" width="200" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="depositFund(row)">{{ $t('arModule.fund.deposit') }}</el-button>
            <el-button type="primary" link size="small" @click="withdrawFund(row)">{{ $t('arModule.fund.withdraw') }}</el-button>
            <el-button type="warning" link size="small" @click="freezeFund(row)">{{ $t('arModule.fund.freeze') }}</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="fundDialogVisible" :title="$t('arModule.fund.createTitle')" width="500px" :aria-label="$t('arModule.fund.createAria')">
      <el-form ref="fundFormRef" :model="fundForm" :rules="fundRules" label-width="80px" :aria-label="$t('arModule.fund.formAria')">
        <el-form-item :label="$t('arModule.fund.accountCode')" prop="account_code">
          <el-input v-model="fundForm.account_code" :placeholder="$t('arModule.fund.accountCodePlaceholder')" />
        </el-form-item>
        <el-form-item :label="$t('arModule.fund.accountName')" prop="account_name">
          <el-input v-model="fundForm.account_name" :placeholder="$t('arModule.fund.accountNamePlaceholder')" />
        </el-form-item>
        <el-form-item :label="$t('arModule.fund.accountType')" prop="account_type">
          <el-select v-model="fundForm.account_type" :placeholder="$t('arModule.fund.typePlaceholder')" style="width: 100%">
            <el-option :label="$t('arModule.fund.typeBank')" value="bank" />
            <el-option :label="$t('arModule.fund.typeCash')" value="cash" />
            <el-option :label="$t('arModule.fund.typeAlipay')" value="alipay" />
            <el-option :label="$t('arModule.fund.typeWechat')" value="wechat" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('arModule.fund.bankName')">
          <el-input v-model="fundForm.bank_name" :placeholder="$t('arModule.fund.bankNamePlaceholder')" />
        </el-form-item>
        <el-form-item :label="$t('arModule.fund.bankAccount')">
          <el-input v-model="fundForm.bank_account" :placeholder="$t('arModule.fund.bankAccountPlaceholder')" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="fundDialogVisible = false">{{ $t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="fundSubmitLoading" @click="submitFund">{{ $t('common.confirm') }}</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="fundOperationDialogVisible" :title="fundOperationTitle" width="400px" :aria-label="$t('arModule.fund.opAria')">
      <el-form label-width="80px" :aria-label="$t('arModule.fund.opFormAria')">
        <el-form-item :label="$t('arModule.fund.amount')">
          <el-input-number
            v-model="fundOperationAmount"
            :min="0"
            :precision="2"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item v-if="fundOperationType === 'freeze'" :label="$t('arModule.fund.reason')">
          <el-input v-model="fundOperationReason" type="textarea" :placeholder="$t('arModule.fund.reasonPlaceholder')" />
        </el-form-item>
        <el-form-item :label="$t('arModule.fund.remark')">
          <el-input v-model="fundOperationRemark" :placeholder="$t('arModule.fund.remarkPlaceholder')" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="fundOperationDialogVisible = false">{{ $t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="fundOperationLoading" @click="submitFundOperation"
          >{{ $t('common.confirm') }}</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import type { FormInstance, FormRules } from 'element-plus'
import {
  getFundAccountList,
  createFundAccount,
  depositFund as depositFundApi,
  withdrawFund as withdrawFundApi,
  freezeFund as freezeFundApi,
  type FundAccount,
} from '@/api/fund'

const { t } = useI18n({ useScope: 'global' })

const funds = ref<FundAccount[]>([])
const fundLoading = ref(false)
const fundSubmitLoading = ref(false)
const fundOperationLoading = ref(false)
const fundDialogVisible = ref(false)
const fundOperationDialogVisible = ref(false)
const fundFormRef = ref<FormInstance>()

const fundOperationType = ref('')
const fundOperationTitle = ref('')
const fundOperationAmount = ref(0)
const fundOperationReason = ref('')
const fundOperationRemark = ref('')
const currentFundAccount = ref<FundAccount | null>(null)

const fundForm = reactive({
  account_code: '',
  account_name: '',
  account_type: 'bank',
  bank_name: '',
  bank_account: '',
})

const fundRules: FormRules = {
  account_code: [{ required: true, message: t('arModule.fund.accountCodeRequired'), trigger: 'blur' }],
  account_name: [{ required: true, message: t('arModule.fund.accountNameRequired'), trigger: 'blur' }],
  account_type: [{ required: true, message: t('arModule.fund.accountTypeRequired'), trigger: 'change' }],
}

const formatMoney = (amount: number) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const getAccountTypeLabel = (type: string) => {
  const keyMap: Record<string, string> = {
    bank: 'arModule.fund.typeBank',
    cash: 'arModule.fund.typeCash',
    alipay: 'arModule.fund.typeAlipay',
    wechat: 'arModule.fund.typeWechat',
  }
  const key = keyMap[type]
  return key ? t(key) : type
}

const totalBalance = computed(() => funds.value.reduce((sum, f) => sum + (f.balance || 0), 0))
const totalFrozen = computed(() => funds.value.reduce((sum, f) => sum + (f.frozen_balance || 0), 0))
const totalAvailable = computed(() =>
  funds.value.reduce((sum, f) => sum + (f.available_balance || 0), 0)
)

const fetchFunds = async () => {
  fundLoading.value = true
  try {
    const res = await getFundAccountList()
    const d = res.data as
      | { list?: FundAccount[]; items?: FundAccount[]; data?: FundAccount[] }
      | FundAccount[]
    funds.value = Array.isArray(d) ? d : d?.list || d?.items || []
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || t('arModule.fund.fetchListFailed'))
  } finally {
    fundLoading.value = false
  }
}

const openFundDialog = () => {
  fundFormRef.value?.resetFields()
  fundForm.account_code = ''
  fundForm.account_name = ''
  fundForm.account_type = 'bank'
  fundForm.bank_name = ''
  fundForm.bank_account = ''
  fundDialogVisible.value = true
}

const submitFund = async () => {
  const valid = await fundFormRef.value?.validate()
  if (!valid) return

  fundSubmitLoading.value = true
  try {
    await createFundAccount(fundForm)
    ElMessage.success(t('common.success'))
    fundDialogVisible.value = false
    fetchFunds()
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || t('common.failed'))
  } finally {
    fundSubmitLoading.value = false
  }
}

const depositFund = (row: FundAccount) => {
  currentFundAccount.value = row
  fundOperationType.value = 'deposit'
  fundOperationTitle.value = t('arModule.fund.depositTitle')
  fundOperationAmount.value = 0
  fundOperationRemark.value = ''
  fundOperationDialogVisible.value = true
}

const withdrawFund = (row: FundAccount) => {
  currentFundAccount.value = row
  fundOperationType.value = 'withdraw'
  fundOperationTitle.value = t('arModule.fund.withdrawTitle')
  fundOperationAmount.value = 0
  fundOperationRemark.value = ''
  fundOperationDialogVisible.value = true
}

const freezeFund = (row: FundAccount) => {
  currentFundAccount.value = row
  fundOperationType.value = 'freeze'
  fundOperationTitle.value = t('arModule.fund.freezeTitle')
  fundOperationAmount.value = 0
  fundOperationReason.value = ''
  fundOperationRemark.value = ''
  fundOperationDialogVisible.value = true
}

const submitFundOperation = async () => {
  if (fundOperationAmount.value <= 0) {
    ElMessage.warning(t('arModule.fund.invalidAmount'))
    return
  }

  if (!currentFundAccount.value) return

  fundOperationLoading.value = true
  try {
    if (fundOperationType.value === 'deposit') {
      await depositFundApi(
        currentFundAccount.value.id,
        fundOperationAmount.value,
        fundOperationRemark.value
      )
      ElMessage.success(t('arModule.fund.depositSuccess'))
    } else if (fundOperationType.value === 'withdraw') {
      await withdrawFundApi(
        currentFundAccount.value.id,
        fundOperationAmount.value,
        fundOperationRemark.value
      )
      ElMessage.success(t('arModule.fund.withdrawSuccess'))
    } else if (fundOperationType.value === 'freeze') {
      if (!fundOperationReason.value) {
        ElMessage.warning(t('arModule.fund.reasonRequired'))
        return
      }
      await freezeFundApi(
        currentFundAccount.value.id,
        fundOperationAmount.value,
        fundOperationReason.value
      )
      ElMessage.success(t('arModule.fund.freezeSuccess'))
    }
    fundOperationDialogVisible.value = false
    fetchFunds()
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || t('common.failed'))
  } finally {
    fundOperationLoading.value = false
  }
}

onMounted(() => {
  fetchFunds()
})
</script>

<style scoped>
.text-orange {
  color: #e6a23c;
}
.text-green {
  color: #67c23a;
}
.mt-20 {
  margin-top: 20px;
}
.fund-summary .summary-item {
  text-align: center;
  padding: 10px 0;
}
.fund-summary .summary-label {
  font-size: 14px;
  color: #909399;
  margin-bottom: 8px;
}
.fund-summary .summary-value {
  font-size: 24px;
  font-weight: 600;
  color: #303133;
}
</style>
