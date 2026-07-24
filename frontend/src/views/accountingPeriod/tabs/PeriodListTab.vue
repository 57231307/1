<!--
  PeriodListTab.vue - 会计期间 Tab
  来源：原 accountingPeriod/index.vue 主体内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="period-list-tab">
    <div class="page-header">
      <h2 class="page-title">{{ $t('accountingPeriod.title') }}</h2>
      <div>
        <el-button type="primary" @click="openDialog()">
          <el-icon><Plus /></el-icon>{{ $t('accountingPeriod.create') }}
        </el-button>
        <el-button @click="handleInitYear">
          <el-icon><Refresh /></el-icon>{{ $t('accountingPeriod.initYear') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryForm" :aria-label="$t('accountingPeriod.filter.ariaLabel')">
        <el-form-item :label="$t('accountingPeriod.filter.year')">
          <el-input-number v-model="queryForm.year" :min="2000" :max="2100" style="width: 140px" />
        </el-form-item>
        <el-form-item :label="$t('accountingPeriod.filter.status')">
          <el-select v-model="queryForm.status" :placeholder="$t('accountingPeriod.filter.statusPlaceholder')" clearable>
            <el-option :label="$t('accountingPeriod.status.pending')" value="pending" />
            <el-option :label="$t('accountingPeriod.status.active')" value="active" />
            <el-option :label="$t('accountingPeriod.status.closed')" value="closed" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleSearch">{{ $t('accountingPeriod.filter.query') }}</el-button>
          <el-button @click="handleReset">{{ $t('accountingPeriod.filter.reset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover">
      <el-table v-loading="loading" :data="periodList" stripe :aria-label="$t('accountingPeriod.table.ariaLabel')">
        <el-table-column prop="name" :label="$t('accountingPeriod.table.name')" width="160" />
        <el-table-column prop="year" :label="$t('accountingPeriod.table.year')" width="80" align="center" />
        <el-table-column prop="month" :label="$t('accountingPeriod.table.month')" width="80" align="center" />
        <el-table-column prop="start_date" :label="$t('accountingPeriod.table.startDate')" width="120" />
        <el-table-column prop="end_date" :label="$t('accountingPeriod.table.endDate')" width="120" />
        <el-table-column prop="status" :label="$t('accountingPeriod.table.status')" width="100">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="closed_at" :label="$t('accountingPeriod.table.closedAt')" width="180" />
        <el-table-column :label="$t('accountingPeriod.table.operation')" width="200" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="openDialog(row)">{{ $t('accountingPeriod.table.edit') }}</el-button>
            <el-button
              v-if="row.status === 'pending'"
              type="success"
              link
              size="small"
              @click="activatePeriod(row)"
              >{{ $t('accountingPeriod.table.enable') }}</el-button
            >
            <el-button
              v-if="row.status === 'active'"
              type="warning"
              link
              size="small"
              @click="closePeriod(row)"
              >{{ $t('accountingPeriod.table.close') }}</el-button
            >
            <el-button
              v-if="row.status === 'closed'"
              type="info"
              link
              size="small"
              @click="reopenPeriod(row)"
              >{{ $t('accountingPeriod.table.reopen') }}</el-button
            >
            <el-button
              v-if="row.status !== 'active'"
              type="danger"
              link
              size="small"
              @click="deletePeriod(row)"
              >{{ $t('accountingPeriod.table.delete') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="dialogVisible" :title="form.id ? $t('accountingPeriod.dialog.editTitle') : $t('accountingPeriod.dialog.createTitle')" width="500px" :aria-label="form.id ? $t('accountingPeriod.dialog.editAriaLabel') : $t('accountingPeriod.dialog.createAriaLabel')">
      <el-form ref="formRef" :model="form" :rules="rules" label-width="100px" :aria-label="$t('accountingPeriod.dialog.ariaLabel')">
        <el-form-item :label="$t('accountingPeriod.dialog.year')" prop="year">
          <el-input-number v-model="form.year" :min="2000" :max="2100" style="width: 100%" />
        </el-form-item>
        <el-form-item :label="$t('accountingPeriod.dialog.month')" prop="month">
          <el-input-number v-model="form.month" :min="1" :max="12" style="width: 100%" />
        </el-form-item>
        <el-form-item :label="$t('accountingPeriod.dialog.name')">
          <el-input :value="`${form.year}-${String(form.month).padStart(2, '0')}`" disabled />
        </el-form-item>
        <el-form-item :label="$t('accountingPeriod.dialog.startDate')" prop="start_date">
          <el-date-picker
            v-model="form.start_date"
            type="date"
            :placeholder="$t('accountingPeriod.dialog.datePlaceholder')"
            value-format="YYYY-MM-DD"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="$t('accountingPeriod.dialog.endDate')" prop="end_date">
          <el-date-picker
            v-model="form.end_date"
            type="date"
            :placeholder="$t('accountingPeriod.dialog.datePlaceholder')"
            value-format="YYYY-MM-DD"
            style="width: 100%"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ $t('accountingPeriod.dialog.cancel') }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{ $t('accountingPeriod.dialog.confirm') }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Refresh } from '@element-plus/icons-vue'
import {
  getAccountingPeriodList,
  createAccountingPeriod,
  updateAccountingPeriod,
  deleteAccountingPeriod,
  closePeriod as closePeriodApi,
  reopenPeriod as reopenPeriodApi,
  type AccountingPeriodEntity,
} from '@/api/accounting-period'
import { logger } from '@/utils/logger'

const { t } = useI18n({ useScope: 'global' })

const loading = ref(false)
const submitLoading = ref(false)
const dialogVisible = ref(false)
const periodList = ref<AccountingPeriodEntity[]>([])
const formRef = ref<FormInstance>()

const queryForm = reactive({
  year: new Date().getFullYear(),
  status: '',
})

const form = reactive<Partial<AccountingPeriodEntity>>({
  id: undefined,
  name: '',
  year: new Date().getFullYear(),
  month: 1,
  start_date: '',
  end_date: '',
  status: 'pending',
})

const rules = computed<FormRules>(() => ({
  year: [{ required: true, message: t('accountingPeriod.validation.yearRequired'), trigger: 'blur' }],
  month: [{ required: true, message: t('accountingPeriod.validation.monthRequired'), trigger: 'blur' }],
  start_date: [{ required: true, message: t('accountingPeriod.validation.startDateRequired'), trigger: 'change' }],
  end_date: [{ required: true, message: t('accountingPeriod.validation.endDateRequired'), trigger: 'change' }],
}))

const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    pending: t('accountingPeriod.status.pending'),
    active: t('accountingPeriod.status.active'),
    closed: t('accountingPeriod.status.closed'),
  }
  return map[status] || status
}

const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    pending: 'info',
    active: 'success',
    closed: 'warning',
  }
  return map[status] || 'info'
}

const fetchPeriods = async () => {
  loading.value = true
  try {
    const res = await getAccountingPeriodList(queryForm)
    const d = (res as { data?: unknown }).data as
      | AccountingPeriodEntity[]
      | {
          items?: AccountingPeriodEntity[]
          data?: AccountingPeriodEntity[]
          list?: AccountingPeriodEntity[]
        }
    if (Array.isArray(d)) {
      periodList.value = d
    } else {
      periodList.value = d?.items || d?.data || d?.list || []
    }
  } catch (e) {
    const err = e as Error
    ElMessage.error(err.message || t('accountingPeriod.message.fetchListFailed'))
  } finally {
    loading.value = false
  }
}

const handleSearch = () => {
  fetchPeriods()
}

const handleReset = () => {
  queryForm.year = new Date().getFullYear()
  queryForm.status = ''
  fetchPeriods()
}

const computeDateRange = (year: number, month: number) => {
  const start = new Date(year, month - 1, 1)
  const end = new Date(year, month, 0)
  return {
    start_date: start.toISOString().split('T')[0],
    end_date: end.toISOString().split('T')[0],
  }
}

const openDialog = (row?: AccountingPeriodEntity) => {
  formRef.value?.resetFields()
  if (row) {
    Object.assign(form, row)
  } else {
    const now = new Date()
    form.id = undefined
    form.name = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`
    form.year = now.getFullYear()
    form.month = now.getMonth() + 1
    const { start_date, end_date } = computeDateRange(form.year, form.month)
    form.start_date = start_date
    form.end_date = end_date
    form.status = 'pending'
  }
  dialogVisible.value = true
}

const handleSubmit = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async valid => {
    if (!valid) return
    submitLoading.value = true
    try {
      form.name = `${form.year}-${String(form.month).padStart(2, '0')}`
      if (form.id) {
        await updateAccountingPeriod(form.id, form)
        ElMessage.success(t('accountingPeriod.message.updateSuccess'))
      } else {
        await createAccountingPeriod(form)
        ElMessage.success(t('accountingPeriod.message.createSuccess'))
      }
      dialogVisible.value = false
      fetchPeriods()
    } catch (e) {
      const err = e as Error
      ElMessage.error(err.message || t('accountingPeriod.message.operationFailed'))
    } finally {
      submitLoading.value = false
    }
  })
}

const activatePeriod = async (row: AccountingPeriodEntity) => {
  if (!row.id) return
  try {
    await ElMessageBox.confirm(
      t('accountingPeriod.message.activateConfirm', { name: row.name }),
      t('accountingPeriod.message.activateConfirmTitle'),
      { type: 'info' }
    )
    await updateAccountingPeriod(row.id, { status: 'active' })
    ElMessage.success(t('accountingPeriod.message.activatedSuccess'))
    fetchPeriods()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error
      ElMessage.error(err.message || t('accountingPeriod.message.activateFailed'))
    }
  }
}

const closePeriod = async (row: AccountingPeriodEntity) => {
  if (!row.id) return
  try {
    await ElMessageBox.confirm(
      t('accountingPeriod.message.closeConfirm', { name: row.name }),
      t('accountingPeriod.message.closeConfirmTitle'),
      { type: 'warning' }
    )
    await closePeriodApi(row.id)
    ElMessage.success(t('accountingPeriod.message.closedSuccess'))
    fetchPeriods()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error
      ElMessage.error(err.message || t('accountingPeriod.message.closeFailed'))
    }
  }
}

const reopenPeriod = async (row: AccountingPeriodEntity) => {
  if (!row.id) return
  try {
    await ElMessageBox.confirm(
      t('accountingPeriod.message.reopenConfirm', { name: row.name }),
      t('accountingPeriod.message.reopenConfirmTitle'),
      { type: 'info' }
    )
    await reopenPeriodApi(row.id)
    ElMessage.success(t('accountingPeriod.message.reopenedSuccess'))
    fetchPeriods()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error
      ElMessage.error(err.message || t('accountingPeriod.message.operationFailed'))
    }
  }
}

const deletePeriod = async (row: AccountingPeriodEntity) => {
  if (!row.id) return
  try {
    await ElMessageBox.confirm(
      t('accountingPeriod.message.deleteConfirm', { name: row.name }),
      t('accountingPeriod.message.deleteConfirmTitle'),
      { type: 'warning' }
    )
    await deleteAccountingPeriod(row.id)
    ElMessage.success(t('accountingPeriod.message.deleteSuccess'))
    fetchPeriods()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error
      ElMessage.error(err.message || t('accountingPeriod.message.deleteFailed'))
    }
  }
}

const handleInitYear = async () => {
  try {
    const year = queryForm.year
    await ElMessageBox.confirm(
      t('accountingPeriod.message.initYearConfirm', { year }),
      t('accountingPeriod.message.initYearConfirmTitle'),
      { type: 'info' }
    )
    for (let month = 1; month <= 12; month++) {
      const { start_date, end_date } = computeDateRange(year, month)
      await createAccountingPeriod({
        name: `${year}-${String(month).padStart(2, '0')}`,
        year,
        month,
        start_date,
        end_date,
        status: 'pending',
      })
    }
    ElMessage.success(t('accountingPeriod.message.initSuccess'))
    fetchPeriods()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error
      logger.error(t('accountingPeriod.message.initFailed'), err)
      ElMessage.error(err.message || t('accountingPeriod.message.initFailed'))
    }
  }
}

onMounted(() => {
  fetchPeriods()
})
</script>
