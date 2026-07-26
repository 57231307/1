<!--
  AccountTab.vue - 资金账户 Tab
  来源：原 fund/index.vue 中 账户管理 tab 内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="account-tab">
    <el-card class="header-card">
      <div class="header-content">
        <h2>{{ t('fund.accountTab.headerTitle') }}</h2>
        <p>{{ t('fund.accountTab.headerSubtitle') }}</p>
      </div>
    </el-card>

    <el-card class="table-card">
      <template #header>
        <div class="card-header">
          <span>{{ t('fund.accountTab.sectionAccountList') }}</span>
          <div>
            <el-button type="success" @click="openTransferDialog()">
              <el-icon><Money /></el-icon>{{ t('fund.accountTab.buttonTransfer') }}
            </el-button>
            <!-- P2-10 修复（批次 82 v1 复审）：补齐 v-permission 按钮权限 -->
            <el-button v-permission="'finance:create'" type="primary" @click="openDialog('create')">
              <el-icon><Plus /></el-icon>{{ t('fund.accountTab.buttonCreateAccount') }}
            </el-button>
          </div>
        </div>
      </template>

      <el-table
        v-loading="loading"
        :data="accountList"
        stripe
        border
        :aria-label="t('fund.accountTab.tableAriaLabel')"
      >
        <el-table-column
          prop="account_no"
          :label="t('fund.accountTab.columnAccountNo')"
          width="160"
        />
        <el-table-column
          prop="account_name"
          :label="t('fund.accountTab.columnAccountName')"
          min-width="160"
        />
        <el-table-column
          prop="bank_name"
          :label="t('fund.accountTab.columnBankName')"
          min-width="160"
        />
        <el-table-column
          prop="current_balance"
          :label="t('fund.accountTab.columnCurrentBalance')"
          width="140"
        >
          <template #default="{ row }">
            <span class="balance-positive"
              >¥{{ (row.current_balance || row.balance || 0).toFixed(2) }}</span
            >
          </template>
        </el-table-column>
        <el-table-column
          prop="frozen_balance"
          :label="t('fund.accountTab.columnFrozenBalance')"
          width="140"
        >
          <template #default="{ row }">
            <span v-if="row.frozen_balance" class="balance-frozen"
              >¥{{ row.frozen_balance.toFixed(2) }}</span
            >
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column prop="status" :label="t('fund.accountTab.columnStatus')" width="120">
          <template #default="{ row }">
            <el-tag
              :type="FUND_ACCOUNT_STATUS[row.status as keyof typeof FUND_ACCOUNT_STATUS]?.type"
            >
              {{ getAccountStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('fund.accountTab.columnActions')" width="360" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="viewDetail(row)">{{
              t('fund.accountTab.buttonView')
            }}</el-button>
            <el-button
              v-if="row.status === 'active'"
              type="success"
              link
              size="small"
              @click="handleDeposit(row)"
              >{{ t('fund.accountTab.buttonDeposit') }}</el-button
            >
            <el-button
              v-if="row.status === 'active'"
              type="warning"
              link
              size="small"
              @click="handleWithdraw(row)"
              >{{ t('fund.accountTab.buttonWithdraw') }}</el-button
            >
            <el-button
              v-if="row.status === 'active'"
              type="info"
              link
              size="small"
              @click="openTransferDialog(row)"
              >{{ t('fund.accountTab.buttonTransferAction') }}</el-button
            >
            <el-button
              v-if="row.status === 'inactive'"
              type="danger"
              link
              size="small"
              @click="handleDelete(row)"
              >{{ t('fund.accountTab.buttonDelete') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-container">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          :aria-label="t('fund.accountTab.paginationAriaLabel')"
          @size-change="handleSizeChange"
          @current-change="handlePageChange"
        />
      </div>
    </el-card>

    <el-dialog
      v-model="dialogVisible"
      :title="
        dialogType === 'create'
          ? t('fund.accountTab.dialogTitleCreate')
          : t('fund.accountTab.dialogTitleEdit')
      "
      width="600px"
      :aria-label="t('fund.accountTab.dialogAriaLabel')"
      @close="resetForm"
    >
      <el-form
        ref="accountFormRef"
        :model="accountForm"
        :rules="accountRules"
        label-width="120px"
        :aria-label="t('fund.accountTab.formAriaLabel')"
      >
        <el-form-item :label="t('fund.accountTab.fieldAccountNo')" prop="account_no">
          <el-input
            v-model="accountForm.account_no"
            :placeholder="t('fund.accountTab.placeholderAccountNo')"
          />
        </el-form-item>
        <el-form-item :label="t('fund.accountTab.fieldAccountName')" prop="account_name">
          <el-input
            v-model="accountForm.account_name"
            :placeholder="t('fund.accountTab.placeholderAccountName')"
          />
        </el-form-item>
        <el-form-item :label="t('fund.accountTab.fieldAccountType')" prop="account_type">
          <el-select
            v-model="accountForm.account_type"
            :placeholder="t('fund.accountTab.placeholderAccountType')"
            style="width: 100%"
          >
            <el-option :label="t('fund.accountTab.accountTypeCash')" value="cash" />
            <el-option :label="t('fund.accountTab.accountTypeBank')" value="bank" />
            <el-option :label="t('fund.accountTab.accountTypeVirtual')" value="virtual" />
          </el-select>
        </el-form-item>
        <el-form-item
          v-if="accountForm.account_type === 'bank'"
          :label="t('fund.accountTab.fieldBankName')"
        >
          <el-input
            v-model="accountForm.bank_name"
            :placeholder="t('fund.accountTab.placeholderBankName')"
          />
        </el-form-item>
        <el-form-item
          v-if="accountForm.account_type === 'bank'"
          :label="t('fund.accountTab.fieldBankAccount')"
        >
          <el-input
            v-model="accountForm.bank_account"
            :placeholder="t('fund.accountTab.placeholderBankAccount')"
          />
        </el-form-item>
        <el-form-item :label="t('fund.accountTab.fieldStatus')" prop="status">
          <el-select
            v-model="accountForm.status"
            :placeholder="t('fund.accountTab.placeholderStatus')"
            style="width: 100%"
          >
            <el-option :label="t('fund.accountTab.statusActive')" value="active" />
            <el-option :label="t('fund.accountTab.statusInactive')" value="inactive" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('fund.accountTab.fieldRemark')">
          <el-input
            v-model="accountForm.remark"
            type="textarea"
            :rows="3"
            :placeholder="t('fund.accountTab.placeholderRemark')"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{
          t('fund.accountTab.buttonCancel')
        }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmitForm">{{
          t('fund.accountTab.buttonConfirm')
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="detailVisible"
      :title="t('fund.accountTab.detailDialogTitle')"
      width="600px"
      :aria-label="t('fund.accountTab.detailDialogAriaLabel')"
    >
      <el-descriptions :column="2" border :aria-label="t('fund.accountTab.detailAriaLabel')">
        <el-descriptions-item :label="t('fund.accountTab.fieldAccountNo')">{{
          currentAccount?.account_no || currentAccount?.account_code
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('fund.accountTab.fieldAccountName')">{{
          currentAccount?.account_name
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('fund.accountTab.fieldAccountType')">{{
          getAccountTypeLabel(currentAccount?.account_type || '')
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('fund.accountTab.columnCurrentBalance')">
          <span class="balance-positive"
            >¥{{
              (currentAccount?.current_balance || currentAccount?.balance || 0).toFixed(2)
            }}</span
          >
        </el-descriptions-item>
        <el-descriptions-item :label="t('fund.accountTab.columnFrozenBalance')">
          <span v-if="currentAccount?.frozen_balance" class="balance-frozen"
            >¥{{ currentAccount.frozen_balance.toFixed(2) }}</span
          >
          <span v-else>-</span>
        </el-descriptions-item>
        <el-descriptions-item :label="t('fund.accountTab.fieldAvailableBalance')">
          <span class="balance-available"
            >¥{{ (currentAccount?.available_balance || 0).toFixed(2) }}</span
          >
        </el-descriptions-item>
        <el-descriptions-item :label="t('fund.accountTab.fieldBankName')">{{
          currentAccount?.bank_name || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('fund.accountTab.columnStatus')">
          <el-tag
            :type="
              FUND_ACCOUNT_STATUS[currentAccount?.status as keyof typeof FUND_ACCOUNT_STATUS]?.type
            "
          >
            {{ getAccountStatusLabel(currentAccount?.status || '') }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item :label="t('fund.accountTab.fieldCreatedAt')">{{
          currentAccount?.created_at
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('fund.accountTab.fieldRemark')" :span="2">{{
          currentAccount?.remark || '-'
        }}</el-descriptions-item>
      </el-descriptions>
    </el-dialog>

    <el-dialog
      v-model="operationVisible"
      :title="
        operationType === 'deposit' ? t('fund.accountTab.deposit') : t('fund.accountTab.withdraw')
      "
      width="500px"
      :aria-label="t('fund.accountTab.operationDialogAriaLabel')"
    >
      <el-form
        ref="operationFormRef"
        :model="operationForm"
        :rules="operationRules"
        label-width="120px"
        :aria-label="t('fund.accountTab.operationFormAriaLabel')"
      >
        <el-form-item :label="t('fund.accountTab.fieldOperationAccount')">
          <el-input :value="currentAccount?.account_name" disabled />
        </el-form-item>
        <el-form-item :label="t('fund.accountTab.columnCurrentBalance')">
          <span class="balance-positive"
            >¥{{
              (currentAccount?.current_balance || currentAccount?.balance || 0).toFixed(2)
            }}</span
          >
        </el-form-item>
        <el-form-item :label="t('fund.accountTab.fieldAmount')" prop="amount">
          <el-input-number
            v-model="operationForm.amount"
            :min="0.01"
            :precision="2"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="t('fund.accountTab.fieldRemark')">
          <el-input
            v-model="operationForm.remark"
            type="textarea"
            :rows="3"
            :placeholder="t('fund.accountTab.placeholderRemark')"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="operationVisible = false">{{
          t('fund.accountTab.buttonCancel')
        }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleOperationSubmit">{{
          t('fund.accountTab.buttonConfirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Money } from '@element-plus/icons-vue'
import {
  createFundAccount,
  updateFundAccount,
  depositFund,
  withdrawFund,
  FUND_ACCOUNT_STATUS,
  type FundAccount,
} from '@/api/fund'
// 批次 278：迁移到 useTableApi composable，自动管理分页与 loading
import { useTableApi } from '@/composables/useTableApi'

const { t } = useI18n({ useScope: 'global' })

const submitLoading = ref(false)
const dialogVisible = ref(false)
const detailVisible = ref(false)
const operationVisible = ref(false)
const dialogType = ref<'create' | 'edit'>('create')
const operationType = ref<'deposit' | 'withdraw'>('deposit')
const currentAccount = ref<FundAccount | null>(null)
const accountFormRef = ref<FormInstance>()
const operationFormRef = ref<FormInstance>()

// 批次 278：使用 useTableApi 管理账户列表分页
const {
  data: accountList,
  total,
  loading,
  page,
  pageSize,
  refresh: fetchAccounts,
} = useTableApi<FundAccount>({
  url: '/fund-management/accounts',
  defaultPageSize: 20,
  onError: (err: unknown) => {
    if (err instanceof Error) {
      ElMessage.error(err.message || t('fund.accountTab.messageFetchFailed'))
    } else {
      ElMessage.error(t('fund.accountTab.messageFetchFailed'))
    }
  },
})

// 批次 278：分页变化处理函数
const handlePageChange = (_p: number) => {
  // useTableApi 内部 watch page 自动触发刷新
}
const handleSizeChange = (_s: number) => {
  // useTableApi 内部 watch pageSize 自动触发刷新
  page.value = 1
}

const accountForm = reactive<Partial<FundAccount>>({
  account_no: '',
  account_code: '',
  account_name: '',
  account_type: 'cash',
  bank_name: '',
  bank_account: '',
  current_balance: 0,
  balance: 0,
  status: 'active',
  remark: '',
})

const operationForm = reactive({
  amount: 0,
  remark: '',
})

const accountRules: FormRules = {
  account_no: [
    { required: true, message: t('fund.accountTab.validateAccountNo'), trigger: 'blur' },
  ],
  account_name: [
    { required: true, message: t('fund.accountTab.validateAccountName'), trigger: 'blur' },
  ],
  account_type: [
    { required: true, message: t('fund.accountTab.validateAccountType'), trigger: 'change' },
  ],
  status: [{ required: true, message: t('fund.accountTab.validateStatus'), trigger: 'change' }],
}

const operationRules: FormRules = {
  amount: [{ required: true, message: t('fund.accountTab.validateAmount'), trigger: 'blur' }],
}

/** 账户状态 → i18n 标签（语言切换响应） */
const getAccountStatusLabel = (status: string): string => {
  switch (status) {
    case 'active':
      return t('fund.accountTab.statusActiveLabel')
    case 'inactive':
      return t('fund.accountTab.statusInactiveLabel')
    case 'frozen':
      return t('fund.accountTab.statusFrozenLabel')
    default:
      return status
  }
}

/** 账户类型 → i18n 标签（语言切换响应） */
const getAccountTypeLabel = (type: string): string => {
  switch (type) {
    case 'cash':
      return t('fund.accountTab.accountTypeCash')
    case 'bank':
      return t('fund.accountTab.accountTypeBank')
    case 'virtual':
      return t('fund.accountTab.accountTypeVirtual')
    default:
      return type
  }
}

const openDialog = (type: 'create' | 'edit', row?: FundAccount) => {
  dialogType.value = type
  resetForm()
  if (type === 'edit' && row) {
    Object.assign(accountForm, row)
  }
  dialogVisible.value = true
}

const resetForm = () => {
  Object.assign(accountForm, {
    id: undefined,
    account_no: '',
    account_code: '',
    account_name: '',
    account_type: 'cash',
    bank_name: '',
    bank_account: '',
    current_balance: 0,
    balance: 0,
    status: 'active',
    remark: '',
  })
  accountFormRef.value?.clearValidate()
}

const handleSubmitForm = async () => {
  if (!accountFormRef.value) return
  await accountFormRef.value.validate(async valid => {
    if (!valid) return
    submitLoading.value = true
    try {
      if (dialogType.value === 'create') {
        await createFundAccount(accountForm)
        ElMessage.success(t('fund.accountTab.messageCreateSuccess'))
      } else {
        if (accountForm.id) {
          await updateFundAccount(accountForm.id, accountForm)
          ElMessage.success(t('fund.accountTab.messageUpdateSuccess'))
        }
      }
      dialogVisible.value = false
      fetchAccounts()
    } catch (e) {
      const err = e as Error
      ElMessage.error(err.message || t('fund.accountTab.messageOperationFailed'))
    } finally {
      submitLoading.value = false
    }
  })
}

const viewDetail = (row: FundAccount) => {
  currentAccount.value = row
  detailVisible.value = true
}

const handleDeposit = (row: FundAccount) => {
  currentAccount.value = row
  operationType.value = 'deposit'
  operationForm.amount = 0
  operationForm.remark = ''
  operationVisible.value = true
}

const handleWithdraw = (row: FundAccount) => {
  currentAccount.value = row
  operationType.value = 'withdraw'
  operationForm.amount = 0
  operationForm.remark = ''
  operationVisible.value = true
}

const handleOperationSubmit = async () => {
  if (!operationFormRef.value || !currentAccount.value) return
  await operationFormRef.value.validate(async valid => {
    if (!valid) return
    submitLoading.value = true
    try {
      const isDeposit = operationType.value === 'deposit'
      const action = isDeposit ? depositFund : withdrawFund
      await action(currentAccount.value!.id, operationForm.amount, operationForm.remark)
      ElMessage.success(
        isDeposit
          ? t('fund.accountTab.messageDepositSuccess')
          : t('fund.accountTab.messageWithdrawSuccess')
      )
      operationVisible.value = false
      fetchAccounts()
    } catch (e) {
      const err = e as Error
      ElMessage.error(err.message || t('fund.accountTab.messageOperationFailed'))
    } finally {
      submitLoading.value = false
    }
  })
}

const openTransferDialog = (_fromAccount?: FundAccount) => {
  ElMessage.info(t('fund.accountTab.messageGotoTransfer'))
}

const handleDelete = async (row: FundAccount) => {
  try {
    await ElMessageBox.confirm(
      t('fund.accountTab.confirmDeleteMessage', { value: row.account_no || row.account_code }),
      t('fund.accountTab.confirmDeleteTitle'),
      {
        type: 'warning',
        confirmButtonText: t('fund.accountTab.buttonConfirm'),
        cancelButtonText: t('fund.accountTab.buttonCancel'),
      }
    )
    ElMessage.success(t('fund.accountTab.messageDeleteSuccess'))
    fetchAccounts()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error
      ElMessage.error(err.message || t('fund.accountTab.messageDeleteFailed'))
    }
  }
}
</script>
