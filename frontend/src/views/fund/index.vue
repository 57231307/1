<template>
  <div class="fund-container">
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
          <el-button type="primary" @click="openDialog('create')">
            <el-icon><Plus /></el-icon>新建账户
          </el-button>
        </div>
      </template>
      
      <el-table :data="accountList" v-loading="loading" stripe border>
        <el-table-column prop="account_no" label="账户编号" width="160" />
        <el-table-column prop="account_name" label="账户名称" min-width="160" />
        <el-table-column prop="bank_name" label="开户行" min-width="160" />
        <el-table-column prop="current_balance" label="当前余额" width="140">
          <template #default="{ row }">
            ¥{{ row.current_balance?.toFixed(2) }}
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="120">
          <template #default="{ row }">
            <el-tag :type="FUND_ACCOUNT_STATUS[row.status as keyof typeof FUND_ACCOUNT_STATUS]?.type">
              {{ FUND_ACCOUNT_STATUS[row.status as keyof typeof FUND_ACCOUNT_STATUS]?.label }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="280" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="viewDetail(row)">查看</el-button>
            <el-button type="success" link size="small" @click="handleDeposit(row)" v-if="row.status === 'active'">存款</el-button>
            <el-button type="warning" link size="small" @click="handleWithdraw(row)" v-if="row.status === 'active'">取款</el-button>
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
        <el-form-item label="开户行">
          <el-input v-model="accountForm.bank_name" placeholder="请输入开户行" />
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
        <el-descriptions-item label="账户编号">{{ currentAccount?.account_no }}</el-descriptions-item>
        <el-descriptions-item label="账户名称">{{ currentAccount?.account_name }}</el-descriptions-item>
        <el-descriptions-item label="开户行">{{ currentAccount?.bank_name || '-' }}</el-descriptions-item>
        <el-descriptions-item label="当前余额">¥{{ currentAccount?.current_balance?.toFixed(2) }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag :type="FUND_ACCOUNT_STATUS[currentAccount?.status as keyof typeof FUND_ACCOUNT_STATUS]?.type">
            {{ FUND_ACCOUNT_STATUS[currentAccount?.status as keyof typeof FUND_ACCOUNT_STATUS]?.label }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="创建时间">{{ currentAccount?.created_at }}</el-descriptions-item>
        <el-descriptions-item label="备注" :span="2">{{ currentAccount?.remark || '-' }}</el-descriptions-item>
      </el-descriptions>
    </el-dialog>

    <el-dialog v-model="operationVisible" title="资金操作" width="500px">
      <el-form :model="operationForm" :rules="operationRules" ref="operationFormRef" label-width="120px">
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
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  listFundAccounts,
  createFundAccount,
  updateFundAccount,
  depositFund,
  withdrawFund,
  type FundAccount,
  FUND_ACCOUNT_STATUS,
} from '../../api/fund'

const loading = ref(false)
const submitLoading = ref(false)
const dialogVisible = ref(false)
const detailVisible = ref(false)
const operationVisible = ref(false)
const dialogType = ref<'create' | 'edit'>('create')
const operationType = ref<'deposit' | 'withdraw'>('deposit')
const accountList = ref<FundAccount[]>([])
const currentAccount = ref<FundAccount | null>(null)
const accountFormRef = ref<FormInstance>()
const operationFormRef = ref<FormInstance>()
const total = ref(0)

const queryForm = reactive({
  page: 1,
  page_size: 20,
})

const accountForm = reactive<Partial<FundAccount>>({
  account_no: '',
  account_name: '',
  bank_name: '',
  current_balance: 0,
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
  status: [{ required: true, message: '请选择状态', trigger: 'change' }],
}

const operationRules: FormRules = {
  amount: [{ required: true, message: '请输入金额', trigger: 'blur' }],
}

const fetchAccounts = async () => {
  loading.value = true
  try {
    const res = await listFundAccounts(queryForm)
    accountList.value = res.data?.list || []
    total.value = res.data?.total || 0
  } catch (e: any) {
    ElMessage.error(e.message || '获取账户列表失败')
  } finally {
    loading.value = false
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
    account_name: '',
    bank_name: '',
    current_balance: 0,
    status: 'active',
    remark: '',
  })
  accountFormRef.value?.clearValidate()
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

const handleDelete = async (row: FundAccount) => {
  try {
    await ElMessageBox.confirm(`确认删除账户 ${row.account_no} 吗？`, '删除确认', {
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
</style>
