<!--
  TransferTab.vue - 转账记录 Tab
  来源：原 fund/index.vue 中 转账记录 tab 内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="transfer-tab">
    <el-card class="table-card">
      <template #header>
        <div class="card-header">
          <span>转账记录</span>
          <el-button type="success" @click="openTransferDialog()">
            <el-icon><Money /></el-icon>发起转账
          </el-button>
        </div>
      </template>

      <el-table v-loading="transferLoading" :data="transferList" stripe border aria-label="资金转账列表">
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
            <el-button type="primary" link size="small" @click="viewTransferDetail(row)"
              >详情</el-button
            >
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-container">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="transferTotal"
          layout="total, sizes, prev, pager, next, jumper"
          aria-label="资金转账列表分页"
        />
      </div>
    </el-card>

    <el-dialog v-model="transferVisible" title="资金转账" width="600px" aria-label="资金转账对话框">
      <el-form
        ref="transferFormRef"
        :model="transferForm"
        :rules="transferRules"
        label-width="120px"
        aria-label="资金转账表单"
      >
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
            可用余额:
            <span class="balance-available"
              >¥{{
                (
                  selectedFromAccount.available_balance ||
                  selectedFromAccount.current_balance ||
                  selectedFromAccount.balance ||
                  0
                ).toFixed(2)
              }}</span
            >
          </div>
        </el-form-item>
        <el-form-item label="备注">
          <el-input
            v-model="transferForm.remark"
            type="textarea"
            :rows="3"
            placeholder="请输入转账备注"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="transferVisible = false">取消</el-button>
        <el-button type="primary" :loading="transferSubmitLoading" @click="handleTransferSubmit"
          >确认转账</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Money } from '@element-plus/icons-vue'
import {
  listFundAccounts,
  getFundTransfer,
  transferFund,
  type FundAccount,
  type FundTransferRecord,
} from '@/api/fund'
// 批次 280：接入 useTableApi，消除手写 transferList/transferLoading/transferTotal/fetchTransfers 重复
import { useTableApi } from '@/composables/useTableApi'

const transferSubmitLoading = ref(false)
const transferVisible = ref(false)
const accountList = ref<FundAccount[]>([])
const transferFormRef = ref<FormInstance>()

// 批次 280：useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
// listFundTransfers 返回 ApiResponse<FundTransferRecord[]>（{ data: T[] }），useTableApi detectList 会 fallback 到 obj.data
const {
  data: transferList,
  loading: transferLoading,
  page,
  pageSize,
  total: transferTotal,
  refresh: fetchTransfers,
} = useTableApi<FundTransferRecord>({
  url: '/fund-management/transfers',
  onError: (err: unknown) =>
    ElMessage.error((err instanceof Error ? err.message : String(err)) || '获取转账记录失败'),
})

const transferForm = reactive({
  from_account_id: undefined as number | undefined,
  to_account_id: undefined as number | undefined,
  amount: 0,
  remark: '',
})

const transferRules: FormRules = {
  from_account_id: [{ required: true, message: '请选择转出账户', trigger: 'change' }],
  to_account_id: [{ required: true, message: '请选择转入账户', trigger: 'change' }],
  amount: [
    { required: true, message: '请输入转账金额', trigger: 'blur' },
    {
      validator: (_rule, value, callback) => {
        if (value <= 0) {
          callback(new Error('转账金额必须大于0'))
        } else if (value > availableBalance.value) {
          callback(new Error('转账金额不能超过可用余额'))
        } else {
          callback()
        }
      },
      trigger: 'blur',
    },
  ],
}

const activeAccounts = computed(() => {
  return accountList.value.filter(acc => acc.status === 'active')
})

const otherAccounts = computed(() => {
  return activeAccounts.value.filter(acc => acc.id !== transferForm.from_account_id)
})

const selectedFromAccount = computed(() => {
  return accountList.value.find(acc => acc.id === transferForm.from_account_id)
})

const availableBalance = computed(() => {
  return selectedFromAccount.value
    ? selectedFromAccount.value.available_balance ||
        selectedFromAccount.value.current_balance ||
        selectedFromAccount.value.balance ||
        0
    : 999999999
})

const fetchAccounts = async () => {
  try {
    const res = await listFundAccounts()
    const d = res.data as
      | { list?: FundAccount[]; items?: FundAccount[]; data?: FundAccount[] }
      | FundAccount[]
    accountList.value = Array.isArray(d) ? d : d?.list || d?.items || []
  } catch (e) {
    const err = e as Error
    ElMessage.error(err.message || '获取账户列表失败')
  }
}

const openTransferDialog = () => {
  transferForm.from_account_id = undefined
  transferForm.to_account_id = undefined
  transferForm.amount = 0
  transferForm.remark = ''
  transferVisible.value = true
}

const handleFromAccountChange = () => {
  if (transferForm.from_account_id === transferForm.to_account_id) {
    transferForm.to_account_id = undefined
  }
}

const handleTransferSubmit = async () => {
  if (!transferFormRef.value) return
  await transferFormRef.value.validate(async valid => {
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
      fetchTransfers()
    } catch (e) {
      const err = e as Error
      ElMessage.error(err.message || '转账失败')
    } finally {
      transferSubmitLoading.value = false
    }
  })
}

// 批次 157a P1-1 修复：接入 getFundTransfer API 展示转账详情
const viewTransferDetail = async (row: FundTransferRecord) => {
  try {
    const res = await getFundTransfer(row.id)
    const d = res.data
    if (!d) {
      ElMessage.warning('未找到转账详情')
      return
    }
    const lines = [
      `转账编号：${d.transfer_no}`,
      `转出账户：${d.from_account_name || '-'}`,
      `转入账户：${d.to_account_name || '-'}`,
      `转账金额：¥${d.amount.toFixed(2)}`,
      `当前状态：${getTransferStatusLabel(d.status)}`,
      `转账时间：${d.created_at}`,
      `备注：${d.remark || '-'}`,
    ]
    await ElMessageBox.alert(lines.join('\n'), '转账详情', {
      confirmButtonText: '关闭',
    })
  } catch (e) {
    const err = e as Error
    ElMessage.error(err.message || '获取转账详情失败')
  }
}

const getTransferStatusType = (status: string) => {
  const map: Record<string, string> = {
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

onMounted(() => {
  fetchAccounts()
})
</script>
