<!--
  ReconciliationTab.vue - 应收对账 Tab
  来源：原 ar/index.vue 中 应收对账 tab 内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="reconciliation-tab">
    <div class="page-header">
      <h2 class="page-title">应收对账</h2>
      <el-button type="primary" @click="openReconciliationDialog()">
        <el-icon><Plus /></el-icon>
        新建对账
      </el-button>
    </div>

    <el-card shadow="hover">
      <el-table v-loading="reconciliationLoading" :data="reconciliations" stripe aria-label="对账列表">
        <el-table-column prop="reconciliation_no" label="对账单号" width="140" />
        <el-table-column prop="customer_name" label="客户" width="150" />
        <el-table-column prop="reconciliation_date" label="对账日期" width="120" />
        <el-table-column label="发票金额" width="120" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.total_invoice_amount) }}
          </template>
        </el-table-column>
        <el-table-column label="收款金额" width="120" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.total_payment_amount) }}
          </template>
        </el-table-column>
        <el-table-column label="差额" width="100" align="right">
          <template #default="{ row }">
            <span :class="{ 'text-red': row.difference_amount !== 0 }">
              {{ formatMoney(row.difference_amount) }}
            </span>
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getReconciliationStatusType(row.status)" size="small">
              {{ getReconciliationStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="confirmed_by" label="确认人" width="100" />
        <el-table-column prop="confirmed_at" label="确认时间" width="160" />
        <el-table-column label="操作" width="120" fixed="right">
          <template #default="{ row }">
            <el-button
              v-if="row.status === 'pending'"
              type="success"
              link
              size="small"
              @click="confirmReconciliation(row)"
              >确认</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="reconciliationDialogVisible" title="新建对账" width="500px" aria-label="新建对账对话框">
      <el-form ref="reconciliationFormRef" :model="reconciliationForm" label-width="80px" aria-label="对账表单">
        <el-form-item label="客户">
          <el-select
            v-model="reconciliationForm.customer_id"
            placeholder="选择客户"
            style="width: 100%"
          >
            <el-option v-for="c in customers" :key="c.id" :label="c.customer_name" :value="c.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="对账日期">
          <el-date-picker
            v-model="reconciliationForm.reconciliation_date"
            type="date"
            placeholder="选择日期"
            value-format="YYYY-MM-DD"
            style="width: 100%"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="reconciliationDialogVisible = false">取消</el-button>
        <el-button
          type="primary"
          :loading="reconciliationSubmitLoading"
          @click="submitReconciliation"
          >确定</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import type { FormInstance } from 'element-plus'
import {
  getARReconciliationList,
  createARReconciliation,
  updateARReconciliationStatus,
  type ARReconciliation,
} from '@/api/ar'
import type { Customer } from '@/api/customer'

const reconciliations = ref<ARReconciliation[]>([])
const customers = ref<Customer[]>([])
const reconciliationLoading = ref(false)
const reconciliationSubmitLoading = ref(false)
const reconciliationDialogVisible = ref(false)
const reconciliationFormRef = ref<FormInstance>()

const reconciliationForm = reactive({
  customer_id: undefined as number | undefined,
  reconciliation_date: '',
})

const formatMoney = (amount: number) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const getReconciliationStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    pending: '待确认',
    confirmed: '已确认',
    disputed: '有异议',
  }
  return map[status] || status
}

const getReconciliationStatusType = (status: string) => {
  const map: Record<string, string> = {
    pending: 'warning',
    confirmed: 'success',
    disputed: 'danger',
  }
  return map[status] || 'info'
}

const fetchReconciliations = async () => {
  reconciliationLoading.value = true
  try {
    const res = await getARReconciliationList()
    const d = res.data as
      | { list?: ARReconciliation[]; items?: ARReconciliation[]; data?: ARReconciliation[] }
      | ARReconciliation[]
    reconciliations.value = Array.isArray(d) ? d : d?.items || d?.data || []
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '获取对账列表失败')
  } finally {
    reconciliationLoading.value = false
  }
}

const openReconciliationDialog = () => {
  reconciliationForm.customer_id = undefined
  reconciliationForm.reconciliation_date = new Date().toISOString().split('T')[0]
  reconciliationDialogVisible.value = true
}

const submitReconciliation = async () => {
  if (!reconciliationForm.customer_id) {
    ElMessage.warning('请选择客户')
    return
  }

  reconciliationSubmitLoading.value = true
  try {
    await createARReconciliation(reconciliationForm)
    ElMessage.success('创建成功')
    reconciliationDialogVisible.value = false
    fetchReconciliations()
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '操作失败')
  } finally {
    reconciliationSubmitLoading.value = false
  }
}

const confirmReconciliation = async (row: ARReconciliation) => {
  try {
    await ElMessageBox.confirm('确定确认该对账单吗？', '确认对账', { type: 'info' })
    await updateARReconciliationStatus(row.id, 'confirmed')
    ElMessage.success('确认成功')
    fetchReconciliations()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || '操作失败')
    }
  }
}

onMounted(() => {
  fetchReconciliations()
})
</script>

<style scoped>
.text-red {
  color: #f56c6c;
}
</style>
