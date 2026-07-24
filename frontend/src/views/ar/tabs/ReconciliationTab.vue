<!--
  ReconciliationTab.vue - 应收对账 Tab
  来源：原 ar/index.vue 中 应收对账 tab 内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="reconciliation-tab">
    <div class="page-header">
      <h2 class="page-title">{{ $t('arModule.reconciliation.title') }}</h2>
      <el-button type="primary" @click="openReconciliationDialog()">
        <el-icon><Plus /></el-icon>
        {{ $t('arModule.reconciliation.create') }}
      </el-button>
    </div>

    <el-card shadow="hover">
      <el-table v-loading="reconciliationLoading" :data="reconciliations" stripe :aria-label="$t('arModule.reconciliation.listAria')">
        <el-table-column prop="reconciliation_no" :label="$t('arModule.reconciliation.reconciliationNo')" width="140" />
        <el-table-column prop="customer_name" :label="$t('arModule.reconciliation.customer')" width="150" />
        <el-table-column prop="reconciliation_date" :label="$t('arModule.reconciliation.reconciliationDate')" width="120" />
        <el-table-column :label="$t('arModule.reconciliation.invoiceAmount')" width="120" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.total_invoice_amount) }}
          </template>
        </el-table-column>
        <el-table-column :label="$t('arModule.reconciliation.paymentAmount')" width="120" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.total_payment_amount) }}
          </template>
        </el-table-column>
        <el-table-column :label="$t('arModule.reconciliation.difference')" width="100" align="right">
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
        <el-table-column prop="confirmed_by" :label="$t('arModule.reconciliation.confirmedBy')" width="100" />
        <el-table-column prop="confirmed_at" :label="$t('arModule.reconciliation.confirmedAt')" width="160" />
        <el-table-column :label="$t('common.operation')" width="120" fixed="right">
          <template #default="{ row }">
            <el-button
              v-if="row.status === 'pending'"
              type="success"
              link
              size="small"
              @click="confirmReconciliation(row)"
              >{{ $t('arModule.reconciliation.confirm') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="reconciliationDialogVisible" :title="$t('arModule.reconciliation.createTitle')" width="500px" :aria-label="$t('arModule.reconciliation.createAria')">
      <el-form ref="reconciliationFormRef" :model="reconciliationForm" label-width="80px" :aria-label="$t('arModule.reconciliation.formAria')">
        <el-form-item :label="$t('arModule.reconciliation.customer')">
          <el-select
            v-model="reconciliationForm.customer_id"
            :placeholder="$t('arModule.reconciliation.customerPlaceholder')"
            style="width: 100%"
          >
            <el-option v-for="c in customers" :key="c.id" :label="c.customer_name" :value="c.id" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('arModule.reconciliation.reconciliationDate')">
          <el-date-picker
            v-model="reconciliationForm.reconciliation_date"
            type="date"
            :placeholder="$t('arModule.reconciliation.datePlaceholder')"
            value-format="YYYY-MM-DD"
            style="width: 100%"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="reconciliationDialogVisible = false">{{ $t('common.cancel') }}</el-button>
        <el-button
          type="primary"
          :loading="reconciliationSubmitLoading"
          @click="submitReconciliation"
          >{{ $t('common.confirm') }}</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
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

const { t } = useI18n({ useScope: 'global' })

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
  const keyMap: Record<string, string> = {
    pending: 'arModule.reconciliation.statusPending',
    confirmed: 'arModule.reconciliation.statusConfirmed',
    disputed: 'arModule.reconciliation.statusDisputed',
  }
  const key = keyMap[status]
  return key ? t(key) : status
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
    ElMessage.error(err.message || t('arModule.reconciliation.fetchListFailed'))
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
    ElMessage.warning(t('arModule.reconciliation.selectCustomer'))
    return
  }

  reconciliationSubmitLoading.value = true
  try {
    await createARReconciliation(reconciliationForm)
    ElMessage.success(t('common.message.createSuccess'))
    reconciliationDialogVisible.value = false
    fetchReconciliations()
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || t('common.failed'))
  } finally {
    reconciliationSubmitLoading.value = false
  }
}

const confirmReconciliation = async (row: ARReconciliation) => {
  try {
    await ElMessageBox.confirm(t('arModule.reconciliation.confirmMessage'), t('arModule.reconciliation.confirmTitle'), { type: 'info' })
    await updateARReconciliationStatus(row.id, 'confirmed')
    ElMessage.success(t('arModule.reconciliation.confirmSuccess'))
    fetchReconciliations()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || t('common.failed'))
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
