<template>
  <div class="fund-container">
    <el-card class="header-card">
      <div class="header-content">
        <h2>资金管理</h2>
        <p>管理资金账户，进行存款、取款、转账等操作</p>
      </div>
    </el-card>

    <el-tabs v-model="activeTab" class="fund-tabs">
      <el-tab-pane label="账户管理" name="account">
        <el-card class="table-card">
          <template #header>
            <div class="card-header">
              <span>账户列表</span>
              <div>
                <el-button type="success" @click="openTransferDialog()">
                  <el-icon><Money /></el-icon>资金转账
                </el-button>
                <el-button type="primary" @click="openDialog('create')">
                  <el-icon><Plus /></el-icon>新建账户
                </el-button>
              </div>
            </div>
          </template>
          
          <el-table :data="accountList" v-loading="loading" stripe border>
            <el-table-column prop="account_no" label="账户编号" width="160" />
            <el-table-column prop="account_name" label="账户名称" min-width="160" />
            <el-table-column prop="bank_name" label="开户行" min-width="160" />
            <el-table-column prop="current_balance" label="当前余额" width="140">
              <template #default="{ row }">
                <span class="balance-positive">¥{{ (row.current_balance || row.balance || 0).toFixed(2) }}</span>
              </template>
            </el-table-column>
            <el-table-column prop="frozen_balance" label="冻结余额" width="140">
              <template #default="{ row }">
                <span v-if="row.frozen_balance" class="balance-frozen">¥{{ row.frozen_balance.toFixed(2) }}</span>
                <span v-else>-</span>
              </template>
            </el-table-column>
            <el-table-column prop="status" label="状态" width="120">
              <template #default="{ row }">
                <el-tag :type="FUND_ACCOUNT_STATUS[row.status as keyof typeof FUND_ACCOUNT_STATUS]?.type">
                  {{ FUND_ACCOUNT_STATUS[row.status as keyof typeof FUND_ACCOUNT_STATUS]?.label }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="360" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="viewDetail(row)">查看</el-button>
                <el-button type="success" link size="small" @click="handleDeposit(row)" v-if="row.status === 'active'">存款</el-button>
                <el-button type="warning" link size="small" @click="handleWithdraw(row)" v-if="row.status === 'active'">取款</el-button>
                <el-button type="info" link size="small" @click="openTransferDialog(row)" v-if="row.status === 'active'">转账</el-button>
                <el-button type="danger" link size="small" @click="handleDelete(row)" v-if="row.status === 'inactive'">删除</el-button>
              </template>
            </el-table-column>
          </el-table>

          <div class="pagination-container">
            <el-pagination
              v-model:current-page="queryForm.page"
              v-model:page-size="queryForm.page_size"
              :page-sizes="[10, 20, 50, 100]"
              :total="total"
              layout="total, sizes, prev, pager, next, jumper"
              @size-change="fetchAccounts"
              @current-change="fetchAccounts"
            />
          </div>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="转账记录" name="transfer">
        <el-card class="table-card">
          <template #header>
            <div class="card-header">
              <span>转账记录</span>
              <el-button type="success" @click="openTransferDialog()">
                <el-icon><Money /></el-icon>发起转账
              </el-button>
            </div>
          </template>
          
          <el-table :data="transferList" v-loading="transferLoading" stripe border>
            <el-table-column prop="transfer_no" label="转账编号" width="180" />
            <el-table-column prop="from_account_name" label="转出账户" min-width="140" />
            <el-table-column prop="to_account_name" label="转入账户" min-width="140" />
            <el-table-column prop="amount" label="转账金额" width="140">
              <template #default="{ row }">
                <span class="balance-positive">¥{{ row.amount.toFixed(2) }}</span>
              </template>
            </el-table-column>
            <el-table-column prop="status" label="状态" width="100">
              <template #default="{ row }">
                <el-tag :type="getTransferStatusType(row.status)">
                  {{ getTransferStatusLabel(row.status) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="remark" label="备注" min-width="150" show-overflow-tooltip />
            <el-table-column prop="created_at" label="转账时间" width="160" />
            <el-table-column label="操作" width="100" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="viewTransferDetail(row)">详情</el-button>
              </template>
            </el-table-column>
          </el-table>

          <div class="pagination-container">
            <el-pagination
              v-model:current-page="transferQueryForm.page"
              v-model:page-size="transferQueryForm.page_size"
              :page-sizes="[10, 20, 50, 100]"
              :total="transferTotal"
              layout="total, sizes, prev, pager, next, jumper"
              @size-change="fetchTransfers"
              @current-change="fetchTransfers"
            />
          </div>
        </el-card>
      </el-tab-pane>
    </el-tabs>

    <el-dialog
      v-model="dialogVisible"
      :title="dialogType === 'create' ? '新建账户' : '编辑账户'"
      width="600px"
      @close="resetForm"
    >
      <el-form :model="accountForm" :rules="accountRules" ref="accountFormRef" label-width="120px">
        <el-form-item label="账户编号" prop="account_no">
          <el-input v-model="accountForm.account_no" placeholder="请输入账户编号" />
        </el-form-item>
        <el-form-item label="账户名称" prop="account_name">
          <el-input v-model="accountForm.account_name" placeholder="请输入账户名称" />
        </el-form-item>
        <el-form-item label="账户类型" prop="account_type">
          <el-select v-model="accountForm.account_type" placeholder="请选择账户类型" style="width: 100%">
            <el-option label="现金账户" value="cash" />
            <el-option label="银行账户" value="bank" />
            <el-option label="虚拟账户" value="virtual" />
          </el-select>
        </el-form-item>
        <el-form-item label="开户行" v-if="accountForm.account_type === 'bank'">
          <el-input v-model="accountForm.bank_name" placeholder="请输入开户行" />
        </el-form-item>
        <el-form-item label="银行账号" v-if="accountForm.account_type === 'bank'">
          <el-input v-model="accountForm.bank_account" placeholder="请输入银行账号" />
        </el-form-item>
        <el-form-item label="状态" prop="status">
          <el-select v-model="accountForm.status" placeholder="请选择状态" style="width: 100%">
            <el-option label="启用" value="active" />
            <el-option label="停用" value="inactive" />
          </el-select>
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="accountForm.remark" type="textarea" :rows="3" placeholder="请输入备注" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmitForm">确认</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="detailVisible" title="账户详情" width="600px">
      <el-descriptions :column="2" border>
        <el-descriptions-item label="账户编号">{{ currentAccount?.account_no || currentAccount?.account_code }}</el-descriptions-item>
        <el-descriptions-item label="账户名称">{{ currentAccount?.account_name }}</el-descriptions-item>
        <el-descriptions-item label="账户类型">{{ currentAccount?.account_type }}</el-descriptions-item>
        <el-descriptions-item label="当前余额">
          <span class="balance-positive">¥{{ (currentAccount?.current_balance || currentAccount?.balance || 0).toFixed(2) }}</span>
        </el-descriptions-item>
        <el-descriptions-item label="冻结余额">
          <span v-if="currentAccount?.frozen_balance" class="balance-frozen">¥{{ currentAccount.frozen_balance.toFixed(2) }}</span>
          <span v-else>-</span>
        </el-descriptions-item>
        <el-descriptions-item label="可用余额">
          <span class="balance-available">¥{{ (currentAccount?.available_balance || 0).toFixed(2) }}</span>
        </el-descriptions-item>
        <el-descriptions-item label="开户行">{{ currentAccount?.bank_name || '-' }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag :type="FUND_ACCOUNT_STATUS[currentAccount?.status as keyof typeof FUND_ACCOUNT_STATUS]?.type">
            {{ FUND_ACCOUNT_STATUS[currentAccount?.status as keyof typeof FUND_ACCOUNT_STATUS]?.label }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="创建时间">{{ currentAccount?.created_at }}</el-descriptions-item>
        <el-descriptions-item label="备注" :span="2">{{ currentAccount?.remark || '-' }}</el-descriptions-item>
      </el-descriptions>
    </el-dialog>

    <el-dialog v-model="operationVisible" :title="operationType === 'deposit' ? '存款' : '取款'" width="500px">
      <el-form :model="operationForm" :rules="operationRules" ref="operationFormRef" label-width="120px">
        <el-form-item label="操作账户">
          <el-input :value="currentAccount?.account_name" disabled />
        </el-form-item>
        <el-form-item label="当前余额">
          <span class="balance-positive">¥{{ (currentAccount?.current_balance || currentAccount?.balance || 0).toFixed(2) }}</span>
        </el-form-item>
        <el-form-item label="金额" prop="amount">
          <el-input-number v-model="operationForm.amount" :min="0.01" :precision="2" style="width: 100%" />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="operationForm.remark" type="textarea" :rows="3" placeholder="请输入备注" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="operationVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleOperationSubmit">确认</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="transferVisible" title="资金转账" width="600px">
      <el-form :model="transferForm" :rules="transferRules" ref="transferFormRef" label-width="120px">
        <el-form-item label="转出账户" prop="from_account_id">
          <el-select 
            v-model="transferForm.from_account_id" 
            placeholder="请选择转出账户" 
            style="width: 100%"
            filterable
            @change="handleFromAccountChange"
          >
            <el-option 
              v-for="account in activeAccounts" 
              :key="account.id" 
              :label="`${account.account_name} (可用: ¥${(account.available_balance || account.current_balance || account.balance || 0).toFixed(2)})`"
              :value="account.id" 
            />
          </el-select>
        </el-form-item>
        <el-form-item label="转入账户" prop="to_account_id">
          <el-select 
            v-model="transferForm.to_account_id" 
            placeholder="请选择转入账户" 
            style="width: 100%"
            filterable
          >
            <el-option 
              v-for="account in otherAccounts" 
              :key="account.id" 
              :label="`${account.account_name} (当前: ¥${(account.current_balance || account.balance || 0).toFixed(2)})`"
              :value="account.id" 
            />
          </el-select>
        </el-form-item>
        <el-form-item label="转账金额" prop="amount">
          <el-input-number 
            v-model="transferForm.amount" 
            :min="0.01" 
            :max="availableBalance"
            :precision="2" 
            style="width: 100%"
            placeholder="请输入转账金额"
          />
          <div v-if="selectedFromAccount" class="balance-hint">
            可用余额: <span class="balance-available">¥{{ (selectedFromAccount.available_balance || selectedFromAccount.current_balance || selectedFromAccount.balance || 0).toFixed(2) }}</span>
          </div>
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="transferForm.remark" type="textarea" :rows="3" placeholder="请输入转账备注" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="transferVisible = false">取消</el-button>
        <el-button type="primary" :loading="transferSubmitLoading" @click="handleTransferSubmit">确认转账</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Money } from '@element-plus/icons-vue'
import {
  listFundAccounts,
  createFundAccount,
  updateFundAccount,
  depositFund,
  withdrawFund,
  transferFund,
  listFundTransfers,
  type FundAccount,
  type FundTransferRecord,
  FUND_ACCOUNT_STATUS,
} from '../../api/fund'

const activeTab = ref('account')
const loading = ref(false)
const submitLoading = ref(false)
const transferSubmitLoading = ref(false)
const transferLoading = ref(false)
const dialogVisible = ref(false)
const detailVisible = ref(false)
const operationVisible = ref(false)
const transferVisible = ref(false)
const dialogType = ref<'create' | 'edit'>('create')
const operationType = ref<'deposit' | 'withdraw'>('deposit')
const accountList = ref<FundAccount[]>([])
const transferList = ref<FundTransferRecord[]>([])
const currentAccount = ref<FundAccount | null>(null)
const accountFormRef = ref<FormInstance>()
const operationFormRef = ref<FormInstance>()
const transferFormRef = ref<FormInstance>()
const total = ref(0)
const transferTotal = ref(0)

const queryForm = reactive({
  page: 1,
  page_size: 20,
})

const transferQueryForm = reactive({
  page: 1,
  page_size: 20,
})

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

const transferForm = reactive({
  from_account_id: undefined as number | undefined,
  to_account_id: undefined as number | undefined,
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

const transferRules: FormRules = {
  from_account_id: [{ required: true, message: '请选择转出账户', trigger: 'change' }],
  to_account_id: [{ required: true, message: '请选择转入账户', trigger: 'change' }],
  amount: [
    { required: true, message: '请输入转账金额', trigger: 'blur' },
    { 
      validator: (_rule, value, callback) => {
        if (value <= 0) {
          callback(new Error('转账金额必须大于0'))
        } else if (value > availableBalance) {
          callback(new Error('转账金额不能超过可用余额'))
        } else {
          callback()
        }
      },
      trigger: 'blur' 
    }
  ],
}

const activeAccounts = computed(() => {
  return accountList.value.filter(acc => acc.status === 'active')
})

const otherAccounts = computed(() => {
  return accountList.value.filter(acc => acc.id !== transferForm.from_account_id && acc.status === 'active')
})

const selectedFromAccount = computed(() => {
  return accountList.value.find(acc => acc.id === transferForm.from_account_id)
})

const availableBalance = computed(() => {
  return selectedFromAccount.value ? (selectedFromAccount.value.available_balance || selectedFromAccount.value.current_balance || selectedFromAccount.value.balance || 0) : 999999999
})

const fetchAccounts = async () => {
  loading.value = true
  try {
    const res = await listFundAccounts(queryForm)
    accountList.value = res.data!.list || res.data! || []
    total.value = res.data?.total || accountList.value.length
  } catch (e: any) {
    ElMessage.error(e.message || '获取账户列表失败')
  } finally {
    loading.value = false
  }
}

const fetchTransfers = async () => {
  transferLoading.value = true
  try {
    const res = await listFundTransfers(transferQueryForm)
    transferList.value = res.data! || []
    transferTotal.value = res.data?.length || 0
  } catch (e: any) {
    ElMessage.error(e.message || '获取转账记录失败')
  } finally {
    transferLoading.value = false
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

const resetTransferForm = () => {
  Object.assign(transferForm, {
    from_account_id: undefined,
    to_account_id: undefined,
    amount: 0,
    remark: '',
  })
  transferFormRef.value?.clearValidate()
}

const handleSubmitForm = async () => {
  if (!accountFormRef.value) return
  
  await accountFormRef.value.validate(async (valid) => {
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
    } catch (e: any) {
      ElMessage.error(e.message || '操作失败')
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
  
  await operationFormRef.value.validate(async (valid) => {
    if (!valid) return
    
    submitLoading.value = true
    try {
      const action = operationType.value === 'deposit' ? depositFund : withdrawFund
      const actionText = operationType.value === 'deposit' ? '存款' : '取款'
      
      await action(currentAccount.value!.id, operationForm.amount, operationForm.remark)
      ElMessage.success(`${actionText}成功`)
      
      operationVisible.value = false
      fetchAccounts()
    } catch (e: any) {
      ElMessage.error(e.message || '操作失败')
    } finally {
      submitLoading.value = false
    }
  })
}

const openTransferDialog = (fromAccount?: FundAccount) => {
  resetTransferForm()
  if (fromAccount) {
    transferForm.from_account_id = fromAccount.id
  }
  transferVisible.value = true
}

const handleFromAccountChange = () => {
  if (transferForm.from_account_id === transferForm.to_account_id) {
    transferForm.to_account_id = undefined
  }
}

const handleTransferSubmit = async () => {
  if (!transferFormRef.value) return
  
  await transferFormRef.value.validate(async (valid) => {
    if (!valid) return
    
    transferSubmitLoading.value = true
    try {
      await transferFund({
        from_account_id: transferForm.from_account_id!,
        to_account_id: transferForm.to_account_id!,
        amount: transferForm.amount,
        remark: transferForm.remark,
      })
      ElMessage.success('转账成功')
      
      transferVisible.value = false
      fetchAccounts()
      if (activeTab.value === 'transfer') {
        fetchTransfers()
      }
    } catch (e: any) {
      ElMessage.error(e.message || '转账失败')
    } finally {
      transferSubmitLoading.value = false
    }
  })
}

const viewTransferDetail = (row: FundTransferRecord) => {
  ElMessage.info('查看转账详情: ' + row.transfer_no)
}

const getTransferStatusType = (status: string) => {
  const map: Record<string, any> = {
    success: 'success',
    pending: 'warning',
    failed: 'danger',
    processing: 'info',
  }
  return map[status] || 'info'
}

const getTransferStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    success: '成功',
    pending: '待处理',
    failed: '失败',
    processing: '处理中',
  }
  return map[status] || status
}

const handleDelete = async (row: FundAccount) => {
  try {
    await ElMessageBox.confirm(`确认删除账户 ${row.account_no || row.account_code} 吗？`, '删除确认', {
      type: 'warning',
      confirmButtonText: '确定',
      cancelButtonText: '取消',
    })
    
    ElMessage.success('删除成功')
    fetchAccounts()
  } catch (e: any) {
    if (e !== 'cancel') {
      ElMessage.error(e.message || '删除失败')
    }
  }
}

onMounted(() => {
  fetchAccounts()
  fetchTransfers()
})
</script>

<style scoped>
.fund-container {
  padding: 20px;
}

.header-card {
  margin-bottom: 20px;
}

.header-content h2 {
  margin: 0 0 8px 0;
  color: #303133;
}

.header-content p {
  margin: 0;
  color: #909399;
}

.fund-tabs {
  margin-top: 20px;
}

.table-card {
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.pagination-container {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}

.balance-positive {
  color: #67c23a;
  font-weight: 500;
}

.balance-frozen {
  color: #e6a23c;
  font-weight: 500;
}

.balance-available {
  color: #409eff;
  font-weight: 500;
}

.balance-hint {
  margin-top: 8px;
  font-size: 12px;
  color: #909399;
}
</style>
