<!--
  FundTab.vue - 资金账户 Tab
  来源：原 ar/index.vue 中 资金账户 tab 内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="fund-tab">
    <div class="page-header">
      <h2 class="page-title">资金账户</h2>
      <el-button type="primary" @click="openFundDialog()">
        <el-icon><Plus /></el-icon>
        新建账户
      </el-button>
    </div>

    <el-row :gutter="20" class="fund-summary">
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="summary-item">
            <div class="summary-label">总余额</div>
            <div class="summary-value">{{ formatMoney(totalBalance) }}</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="summary-item">
            <div class="summary-label">冻结金额</div>
            <div class="summary-value text-orange">{{ formatMoney(totalFrozen) }}</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="summary-item">
            <div class="summary-label">可用余额</div>
            <div class="summary-value text-green">{{ formatMoney(totalAvailable) }}</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="summary-item">
            <div class="summary-label">账户数量</div>
            <div class="summary-value">{{ funds.length }}</div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover" class="mt-20">
      <el-table v-loading="fundLoading" :data="funds" stripe aria-label="资金账户列表">
        <el-table-column prop="account_code" label="账户编码" width="120" />
        <el-table-column prop="account_name" label="账户名称" min-width="150" />
        <el-table-column prop="account_type" label="账户类型" width="100">
          <template #default="{ row }">
            {{ getAccountTypeLabel(row.account_type) }}
          </template>
        </el-table-column>
        <el-table-column label="余额" width="140" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.balance) }}
          </template>
        </el-table-column>
        <el-table-column label="冻结金额" width="120" align="right">
          <template #default="{ row }">
            <span class="text-orange">{{ formatMoney(row.frozen_balance) }}</span>
          </template>
        </el-table-column>
        <el-table-column label="可用余额" width="140" align="right">
          <template #default="{ row }">
            <span class="text-green">{{ formatMoney(row.available_balance) }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="bank_name" label="开户银行" width="150" />
        <el-table-column prop="bank_account" label="银行账号" width="180" />
        <el-table-column prop="status" label="状态" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{ row.status === 'active' ? '正常' : '冻结' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="depositFund(row)">存入</el-button>
            <el-button type="primary" link size="small" @click="withdrawFund(row)">取出</el-button>
            <el-button type="warning" link size="small" @click="freezeFund(row)">冻结</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="fundDialogVisible" title="新建资金账户" width="500px" aria-label="新建资金账户对话框">
      <el-form ref="fundFormRef" :model="fundForm" :rules="fundRules" label-width="80px" aria-label="新建资金账户表单">
        <el-form-item label="账户编码" prop="account_code">
          <el-input v-model="fundForm.account_code" placeholder="请输入账户编码" />
        </el-form-item>
        <el-form-item label="账户名称" prop="account_name">
          <el-input v-model="fundForm.account_name" placeholder="请输入账户名称" />
        </el-form-item>
        <el-form-item label="账户类型" prop="account_type">
          <el-select v-model="fundForm.account_type" placeholder="选择类型" style="width: 100%">
            <el-option label="银行账户" value="bank" />
            <el-option label="现金账户" value="cash" />
            <el-option label="支付宝" value="alipay" />
            <el-option label="微信" value="wechat" />
          </el-select>
        </el-form-item>
        <el-form-item label="开户银行">
          <el-input v-model="fundForm.bank_name" placeholder="请输入开户银行" />
        </el-form-item>
        <el-form-item label="银行账号">
          <el-input v-model="fundForm.bank_account" placeholder="请输入银行账号" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="fundDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="fundSubmitLoading" @click="submitFund">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="fundOperationDialogVisible" :title="fundOperationTitle" width="400px" aria-label="资金账户操作对话框">
      <el-form label-width="80px" aria-label="资金账户操作表单">
        <el-form-item label="金额">
          <el-input-number
            v-model="fundOperationAmount"
            :min="0"
            :precision="2"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item v-if="fundOperationType === 'freeze'" label="原因">
          <el-input v-model="fundOperationReason" type="textarea" placeholder="请输入冻结原因" />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="fundOperationRemark" placeholder="请输入备注" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="fundOperationDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="fundOperationLoading" @click="submitFundOperation"
          >确定</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
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
  account_code: [{ required: true, message: '请输入账户编码', trigger: 'blur' }],
  account_name: [{ required: true, message: '请输入账户名称', trigger: 'blur' }],
  account_type: [{ required: true, message: '请选择账户类型', trigger: 'change' }],
}

const formatMoney = (amount: number) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const getAccountTypeLabel = (type: string) => {
  const map: Record<string, string> = {
    bank: '银行账户',
    cash: '现金账户',
    alipay: '支付宝',
    wechat: '微信',
  }
  return map[type] || type
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
    ElMessage.error(err.message || '获取资金账户列表失败')
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
    ElMessage.success('创建成功')
    fundDialogVisible.value = false
    fetchFunds()
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '操作失败')
  } finally {
    fundSubmitLoading.value = false
  }
}

const depositFund = (row: FundAccount) => {
  currentFundAccount.value = row
  fundOperationType.value = 'deposit'
  fundOperationTitle.value = '存入资金'
  fundOperationAmount.value = 0
  fundOperationRemark.value = ''
  fundOperationDialogVisible.value = true
}

const withdrawFund = (row: FundAccount) => {
  currentFundAccount.value = row
  fundOperationType.value = 'withdraw'
  fundOperationTitle.value = '取出资金'
  fundOperationAmount.value = 0
  fundOperationRemark.value = ''
  fundOperationDialogVisible.value = true
}

const freezeFund = (row: FundAccount) => {
  currentFundAccount.value = row
  fundOperationType.value = 'freeze'
  fundOperationTitle.value = '冻结资金'
  fundOperationAmount.value = 0
  fundOperationReason.value = ''
  fundOperationRemark.value = ''
  fundOperationDialogVisible.value = true
}

const submitFundOperation = async () => {
  if (fundOperationAmount.value <= 0) {
    ElMessage.warning('请输入有效金额')
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
      ElMessage.success('存入成功')
    } else if (fundOperationType.value === 'withdraw') {
      await withdrawFundApi(
        currentFundAccount.value.id,
        fundOperationAmount.value,
        fundOperationRemark.value
      )
      ElMessage.success('取出成功')
    } else if (fundOperationType.value === 'freeze') {
      if (!fundOperationReason.value) {
        ElMessage.warning('请输入冻结原因')
        return
      }
      await freezeFundApi(
        currentFundAccount.value.id,
        fundOperationAmount.value,
        fundOperationReason.value
      )
      ElMessage.success('冻结成功')
    }
    fundOperationDialogVisible.value = false
    fetchFunds()
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '操作失败')
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
