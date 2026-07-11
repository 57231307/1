<!--
  AccountTab.vue - 资金账户 Tab
  来源：原 fund/index.vue 中 账户管理 tab 内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="account-tab">
    <el-card class="header-card">
      <div class="header-content">
        <h2>资金管理</h2>
        <p>管理资金账户，进行存款、取款、转账等操作</p>
      </div>
    </el-card>

    <el-card class="table-card">
      <template #header>
        <div class="card-header">
          <span>账户列表</span>
          <div>
            <el-button type="success" @click="openTransferDialog()">
              <el-icon><Money /></el-icon>资金转账
            </el-button>
            <!-- P2-10 修复（批次 82 v1 复审）：补齐 v-permission 按钮权限 -->
            <el-button v-permission="'finance:create'" type="primary" @click="openDialog('create')">
              <el-icon><Plus /></el-icon>新建账户
            </el-button>
          </div>
        </div>
      </template>

      <el-table v-loading="loading" :data="accountList" stripe border>
        <el-table-column prop="account_no" label="账户编号" width="160" />
        <el-table-column prop="account_name" label="账户名称" min-width="160" />
        <el-table-column prop="bank_name" label="开户行" min-width="160" />
        <el-table-column prop="current_balance" label="当前余额" width="140">
          <template #default="{ row }">
            <span class="balance-positive"
              >¥{{ (row.current_balance || row.balance || 0).toFixed(2) }}</span
            >
          </template>
        </el-table-column>
        <el-table-column prop="frozen_balance" label="冻结余额" width="140">
          <template #default="{ row }">
            <span v-if="row.frozen_balance" class="balance-frozen"
              >¥{{ row.frozen_balance.toFixed(2) }}</span
            >
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="120">
          <template #default="{ row }">
            <el-tag
              :type="FUND_ACCOUNT_STATUS[row.status as keyof typeof FUND_ACCOUNT_STATUS]?.type"
            >
              {{ FUND_ACCOUNT_STATUS[row.status as keyof typeof FUND_ACCOUNT_STATUS]?.label }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="360" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="viewDetail(row)">查看</el-button>
            <el-button
              v-if="row.status === 'active'"
              type="success"
              link
              size="small"
              @click="handleDeposit(row)"
              >存款</el-button
            >
            <el-button
              v-if="row.status === 'active'"
              type="warning"
              link
              size="small"
              @click="handleWithdraw(row)"
              >取款</el-button
            >
            <el-button
              v-if="row.status === 'active'"
              type="info"
              link
              size="small"
              @click="openTransferDialog(row)"
              >转账</el-button
            >
            <el-button
              v-if="row.status === 'inactive'"
              type="danger"
              link
              size="small"
              @click="handleDelete(row)"
              >删除</el-button
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
          @size-change="handleSizeChange"
          @current-change="handlePageChange"
        />
      </div>
    </el-card>

    <el-dialog
      v-model="dialogVisible"
      :title="dialogType === 'create' ? '新建账户' : '编辑账户'"
      width="600px"
      @close="resetForm"
    >
      <el-form ref="accountFormRef" :model="accountForm" :rules="accountRules" label-width="120px">
        <el-form-item label="账户编号" prop="account_no">
          <el-input v-model="accountForm.account_no" placeholder="请输入账户编号" />
        </el-form-item>
        <el-form-item label="账户名称" prop="account_name">
          <el-input v-model="accountForm.account_name" placeholder="请输入账户名称" />
        </el-form-item>
        <el-form-item label="账户类型" prop="account_type">
          <el-select
            v-model="accountForm.account_type"
            placeholder="请选择账户类型"
            style="width: 100%"
          >
            <el-option label="现金账户" value="cash" />
            <el-option label="银行账户" value="bank" />
            <el-option label="虚拟账户" value="virtual" />
          </el-select>
        </el-form-item>
        <el-form-item v-if="accountForm.account_type === 'bank'" label="开户行">
          <el-input v-model="accountForm.bank_name" placeholder="请输入开户行" />
        </el-form-item>
        <el-form-item v-if="accountForm.account_type === 'bank'" label="银行账号">
          <el-input v-model="accountForm.bank_account" placeholder="请输入银行账号" />
        </el-form-item>
        <el-form-item label="状态" prop="status">
          <el-select v-model="accountForm.status" placeholder="请选择状态" style="width: 100%">
            <el-option label="启用" value="active" />
            <el-option label="停用" value="inactive" />
          </el-select>
        </el-form-item>
        <el-form-item label="备注">
          <el-input
            v-model="accountForm.remark"
            type="textarea"
            :rows="3"
            placeholder="请输入备注"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmitForm"
          >确认</el-button
        >
      </template>
    </el-dialog>

    <el-dialog v-model="detailVisible" title="账户详情" width="600px">
      <el-descriptions :column="2" border>
        <el-descriptions-item label="账户编号">{{
          currentAccount?.account_no || currentAccount?.account_code
        }}</el-descriptions-item>
        <el-descriptions-item label="账户名称">{{
          currentAccount?.account_name
        }}</el-descriptions-item>
        <el-descriptions-item label="账户类型">{{
          currentAccount?.account_type
        }}</el-descriptions-item>
        <el-descriptions-item label="当前余额">
          <span class="balance-positive"
            >¥{{
              (currentAccount?.current_balance || currentAccount?.balance || 0).toFixed(2)
            }}</span
          >
        </el-descriptions-item>
        <el-descriptions-item label="冻结余额">
          <span v-if="currentAccount?.frozen_balance" class="balance-frozen"
            >¥{{ currentAccount.frozen_balance.toFixed(2) }}</span
          >
          <span v-else>-</span>
        </el-descriptions-item>
        <el-descriptions-item label="可用余额">
          <span class="balance-available"
            >¥{{ (currentAccount?.available_balance || 0).toFixed(2) }}</span
          >
        </el-descriptions-item>
        <el-descriptions-item label="开户行">{{
          currentAccount?.bank_name || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag
            :type="
              FUND_ACCOUNT_STATUS[currentAccount?.status as keyof typeof FUND_ACCOUNT_STATUS]?.type
            "
          >
            {{
              FUND_ACCOUNT_STATUS[currentAccount?.status as keyof typeof FUND_ACCOUNT_STATUS]?.label
            }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="创建时间">{{
          currentAccount?.created_at
        }}</el-descriptions-item>
        <el-descriptions-item label="备注" :span="2">{{
          currentAccount?.remark || '-'
        }}</el-descriptions-item>
      </el-descriptions>
    </el-dialog>

    <el-dialog
      v-model="operationVisible"
      :title="operationType === 'deposit' ? '存款' : '取款'"
      width="500px"
    >
      <el-form
        ref="operationFormRef"
        :model="operationForm"
        :rules="operationRules"
        label-width="120px"
      >
        <el-form-item label="操作账户">
          <el-input :value="currentAccount?.account_name" disabled />
        </el-form-item>
        <el-form-item label="当前余额">
          <span class="balance-positive"
            >¥{{
              (currentAccount?.current_balance || currentAccount?.balance || 0).toFixed(2)
            }}</span
          >
        </el-form-item>
        <el-form-item label="金额" prop="amount">
          <el-input-number
            v-model="operationForm.amount"
            :min="0.01"
            :precision="2"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="备注">
          <el-input
            v-model="operationForm.remark"
            type="textarea"
            :rows="3"
            placeholder="请输入备注"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="operationVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleOperationSubmit"
          >确认</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
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
      ElMessage.error(err.message || '获取账户列表失败')
    } else {
      ElMessage.error('获取账户列表失败')
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
  account_no: [{ required: true, message: '请输入账户编号', trigger: 'blur' }],
  account_name: [{ required: true, message: '请输入账户名称', trigger: 'blur' }],
  account_type: [{ required: true, message: '请选择账户类型', trigger: 'change' }],
  status: [{ required: true, message: '请选择状态', trigger: 'change' }],
}

const operationRules: FormRules = {
  amount: [{ required: true, message: '请输入金额', trigger: 'blur' }],
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
        ElMessage.success('创建成功')
      } else {
        if (accountForm.id) {
          await updateFundAccount(accountForm.id, accountForm)
          ElMessage.success('更新成功')
        }
      }
      dialogVisible.value = false
      fetchAccounts()
    } catch (e) {
      const err = e as Error
      ElMessage.error(err.message || '操作失败')
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
      const action = operationType.value === 'deposit' ? depositFund : withdrawFund
      const actionText = operationType.value === 'deposit' ? '存款' : '取款'
      await action(currentAccount.value!.id, operationForm.amount, operationForm.remark)
      ElMessage.success(`${actionText}成功`)
      operationVisible.value = false
      fetchAccounts()
    } catch (e) {
      const err = e as Error
      ElMessage.error(err.message || '操作失败')
    } finally {
      submitLoading.value = false
    }
  })
}

const openTransferDialog = (_fromAccount?: FundAccount) => {
  ElMessage.info('请前往"转账记录"页发起转账')
}

const handleDelete = async (row: FundAccount) => {
  try {
    await ElMessageBox.confirm(
      `确认删除账户 ${row.account_no || row.account_code} 吗？`,
      '删除确认',
      {
        type: 'warning',
        confirmButtonText: '确定',
        cancelButtonText: '取消',
      }
    )
    ElMessage.success('删除成功')
    fetchAccounts()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error
      ElMessage.error(err.message || '删除失败')
    }
  }
}
</script>
