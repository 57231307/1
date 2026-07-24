<!--
  ReconciliationTab.vue - 对账管理 Tab
  来源：原 ap/index.vue 中 对账管理 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="reconciliation-tab">
    <div class="page-header">
      <h2 class="page-title">{{ $t('apModule.reconciliation.title') }}</h2>
      <el-button type="primary" @click="generateReconciliation()">
        <el-icon><Plus /></el-icon> {{ $t('apModule.reconciliation.generate') }}
      </el-button>
    </div>

    <el-card shadow="hover">
      <el-table v-loading="reconciliationLoading" :data="reconciliations" stripe :aria-label="$t('apModule.reconciliation.listAria')">
        <el-table-column prop="reconciliation_no" :label="$t('apModule.reconciliation.reconciliationNo')" width="140" />
        <el-table-column prop="supplier_name" :label="$t('apModule.reconciliation.supplier')" width="150" />
        <el-table-column prop="reconciliation_date" :label="$t('apModule.reconciliation.reconciliationDate')" width="120" />
        <el-table-column :label="$t('apModule.reconciliation.invoiceAmount')" width="120" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.total_invoice_amount) }}
          </template>
        </el-table-column>
        <el-table-column :label="$t('apModule.reconciliation.paymentAmount')" width="120" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.total_payment_amount) }}
          </template>
        </el-table-column>
        <el-table-column :label="$t('apModule.reconciliation.difference')" width="100" align="right">
          <template #default="{ row }">
            <span :class="{ 'text-red': row.difference_amount !== 0 }">
              {{ formatMoney(row.difference_amount) }}
            </span>
          </template>
        </el-table-column>
        <el-table-column prop="status" :label="$t('common.status')" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getReconciliationStatusType(row.status)" size="small">
              {{ getReconciliationStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="$t('common.operation')" width="150" fixed="right">
          <template #default="{ row }">
            <el-button
              v-if="row.status === 'pending'"
              type="success"
              link
              size="small"
              @click="confirmReconciliation(row as unknown as APReconciliation)"
              >{{ $t('apModule.reconciliation.confirm') }}</el-button
            >
            <el-button
              v-if="row.status === 'pending'"
              type="warning"
              link
              size="small"
              @click="disputeReconciliation(row as unknown as APReconciliation)"
              >{{ $t('apModule.reconciliation.dispute') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="reconciliationDialogVisible" :title="$t('apModule.reconciliation.generateTitle')" width="500px" :aria-label="$t('apModule.reconciliation.generateAria')">
      <el-form :model="reconciliationForm" label-width="100px" :aria-label="$t('apModule.reconciliation.generateFormAria')">
        <el-form-item :label="$t('apModule.reconciliation.supplier')" required>
          <el-select
            v-model="reconciliationForm.supplier_id"
            :placeholder="$t('apModule.reconciliation.supplierPlaceholder')"
            style="width: 100%"
          >
            <el-option v-for="s in suppliers" :key="s.id" :label="s.supplier_name" :value="s.id" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('apModule.reconciliation.startDate')" required>
          <el-date-picker
            v-model="reconciliationForm.start_date"
            type="date"
            value-format="YYYY-MM-DD"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="$t('apModule.reconciliation.endDate')" required>
          <el-date-picker
            v-model="reconciliationForm.end_date"
            type="date"
            value-format="YYYY-MM-DD"
            style="width: 100%"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="reconciliationDialogVisible = false">{{ $t('common.cancel') }}</el-button>
        <el-button type="primary" @click="submitReconciliation">{{ $t('apModule.reconciliation.generate') }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  getAPReconciliationList,
  generateAPReconciliation,
  confirmAPReconciliation,
  disputeAPReconciliation,
  type APReconciliation,
} from '@/api/ap-reconciliation'
import type { Supplier } from '@/api/supplier'

const { t } = useI18n({ useScope: 'global' })

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
  const keyMap: Record<string, string> = {
    pending: 'apModule.reconciliation.statusPending',
    confirmed: 'apModule.reconciliation.statusConfirmed',
    disputed: 'apModule.reconciliation.statusDisputed',
  }
  const key = keyMap[status]
  return key ? t(key) : status
}

const fetchReconciliations = async () => {
  reconciliationLoading.value = true
  try {
    const res = await getAPReconciliationList()
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
    ElMessage.error(err.message || t('apModule.reconciliation.fetchListFailed'))
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
    ElMessage.warning(t('apModule.reconciliation.pleaseFillComplete'))
    return
  }
  try {
    await generateAPReconciliation({
      supplier_id: reconciliationForm.supplier_id,
      start_date: reconciliationForm.start_date,
      end_date: reconciliationForm.end_date,
    })
    ElMessage.success(t('apModule.reconciliation.generateSuccess'))
    reconciliationDialogVisible.value = false
    fetchReconciliations()
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || t('apModule.reconciliation.generateFailed'))
  }
}

const confirmReconciliation = async (row: APReconciliation) => {
  try {
    await ElMessageBox.confirm(t('apModule.reconciliation.confirmConfirm'), t('apModule.reconciliation.confirmTitle'), { type: 'info' })
    await confirmAPReconciliation(row.id)
    ElMessage.success(t('apModule.reconciliation.confirmSuccess'))
    fetchReconciliations()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || t('common.failed'))
    }
  }
}

const disputeReconciliation = async (row: APReconciliation) => {
  try {
    const { value } = await ElMessageBox.prompt(t('apModule.reconciliation.disputePrompt'), t('apModule.reconciliation.disputeTitle'), {
      inputPattern: /.+/,
      inputErrorMessage: t('apModule.reconciliation.disputePrompt'),
    })
    await disputeAPReconciliation(row.id, value)
    ElMessage.success(t('apModule.reconciliation.disputeSubmitted'))
    fetchReconciliations()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || t('common.failed'))
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
