<!--
  ReconciliationTab.vue - 对账管理 Tab
  来源：原 ap/index.vue 中 对账管理 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="reconciliation-tab">
    <div class="page-header">
      <h2 class="page-title">对账管理</h2>
      <el-button type="primary" @click="generateReconciliation()">
        <el-icon><Plus /></el-icon> 生成对账单
      </el-button>
    </div>

    <el-card shadow="hover">
      <el-table v-loading="reconciliationLoading" :data="reconciliations" stripe aria-label="对账单列表">
        <el-table-column prop="reconciliation_no" label="对账单号" width="140" />
        <el-table-column prop="supplier_name" label="供应商" width="150" />
        <el-table-column prop="reconciliation_date" label="对账日期" width="120" />
        <el-table-column label="发票金额" width="120" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.total_invoice_amount) }}
          </template>
        </el-table-column>
        <el-table-column label="付款金额" width="120" align="right">
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
        <el-table-column label="操作" width="150" fixed="right">
          <template #default="{ row }">
            <el-button
              v-if="row.status === 'pending'"
              type="success"
              link
              size="small"
              @click="confirmReconciliation(row as unknown as APReconciliation)"
              >确认</el-button
            >
            <el-button
              v-if="row.status === 'pending'"
              type="warning"
              link
              size="small"
              @click="disputeReconciliation(row as unknown as APReconciliation)"
              >异议</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="reconciliationDialogVisible" title="生成对账单" width="500px" aria-label="生成对账单对话框">
      <el-form :model="reconciliationForm" label-width="100px" aria-label="生成对账单表单">
        <el-form-item label="供应商" required>
          <el-select
            v-model="reconciliationForm.supplier_id"
            placeholder="选择供应商"
            style="width: 100%"
          >
            <el-option v-for="s in suppliers" :key="s.id" :label="s.supplier_name" :value="s.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="开始日期" required>
          <el-date-picker
            v-model="reconciliationForm.start_date"
            type="date"
            value-format="YYYY-MM-DD"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="结束日期" required>
          <el-date-picker
            v-model="reconciliationForm.end_date"
            type="date"
            value-format="YYYY-MM-DD"
            style="width: 100%"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="reconciliationDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="submitReconciliation">生成</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  listAPReconciliations,
  generateAPReconciliation,
  confirmAPReconciliation,
  disputeAPReconciliation,
  type APReconciliation,
} from '@/api/ap-reconciliation'
import type { Supplier } from '@/api/supplier'

const reconciliations = ref<APReconciliation[]>([])
const reconciliationLoading = ref(false)
const suppliers = ref<Supplier[]>([])

const formatMoney = (amount: number | undefined) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const getReconciliationStatusType = (status: string) => {
  const map: Record<string, string> = {
    pending: 'warning',
    confirmed: 'success',
    disputed: 'danger',
  }
  return map[status] || 'info'
}

const getReconciliationStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    pending: '待确认',
    confirmed: '已确认',
    disputed: '有异议',
  }
  return map[status] || status
}

const fetchReconciliations = async () => {
  reconciliationLoading.value = true
  try {
    const res = await listAPReconciliations()
    const d = res.data as
      | { list?: APReconciliation[]; items?: APReconciliation[] }
      | APReconciliation[]
      | undefined
    if (d && typeof d === 'object' && !Array.isArray(d)) {
      reconciliations.value = d.list || d.items || []
    } else {
      reconciliations.value = (d as APReconciliation[]) || []
    }
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || '获取对账单列表失败')
  } finally {
    reconciliationLoading.value = false
  }
}

const reconciliationDialogVisible = ref(false)
const reconciliationForm = reactive({
  supplier_id: undefined as number | undefined,
  start_date: '',
  end_date: '',
})

const generateReconciliation = () => {
  reconciliationForm.supplier_id = undefined
  reconciliationForm.start_date = ''
  reconciliationForm.end_date = ''
  reconciliationDialogVisible.value = true
}

const submitReconciliation = async () => {
  if (
    !reconciliationForm.supplier_id ||
    !reconciliationForm.start_date ||
    !reconciliationForm.end_date
  ) {
    ElMessage.warning('请填写完整信息')
    return
  }
  try {
    await generateAPReconciliation({
      supplier_id: reconciliationForm.supplier_id,
      start_date: reconciliationForm.start_date,
      end_date: reconciliationForm.end_date,
    })
    ElMessage.success('对账单生成成功')
    reconciliationDialogVisible.value = false
    fetchReconciliations()
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || '生成失败')
  }
}

const confirmReconciliation = async (row: APReconciliation) => {
  try {
    await ElMessageBox.confirm('确定确认该对账单吗？', '确认对账', { type: 'info' })
    await confirmAPReconciliation(row.id)
    ElMessage.success('确认成功')
    fetchReconciliations()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || '操作失败')
    }
  }
}

const disputeReconciliation = async (row: APReconciliation) => {
  try {
    const { value } = await ElMessageBox.prompt('请输入异议原因', '异议说明', {
      inputPattern: /.+/,
      inputErrorMessage: '请输入异议原因',
    })
    await disputeAPReconciliation(row.id, value)
    ElMessage.success('已提交异议')
    fetchReconciliations()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || '操作失败')
    }
  }
}

defineExpose({ refresh: fetchReconciliations })

onMounted(() => {
  fetchReconciliations()
  suppliers.value = []
})
</script>

<style scoped>
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
.page-title {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}
.text-red {
  color: #f56c6c;
}
</style>
