<!--
  InvoiceTab.vue - 应收发票 Tab
  来源：原 ar/index.vue 中 应收发票 tab 内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="invoice-tab">
    <div class="page-header">
      <h2 class="page-title">{{ $t('arModule.invoice.title') }}</h2>
      <div class="header-actions">
        <el-button type="primary" @click="openInvoiceDialog()">
          <el-icon><Plus /></el-icon>
          {{ $t('arModule.invoice.create') }}
        </el-button>
        <el-button @click="handlePrintInvoices">
          <el-icon><Printer /></el-icon>
          {{ $t('common.print') }}
        </el-button>
        <el-button @click="handleExportInvoices">
          <el-icon><Download /></el-icon>
          {{ $t('common.export') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="invoiceQuery" :aria-label="$t('arModule.invoice.filterAria')">
        <el-form-item :label="$t('arModule.invoice.customer')">
          <el-input v-model="invoiceQuery.customer_name" :placeholder="$t('arModule.invoice.customerNamePlaceholder')" clearable />
        </el-form-item>
        <el-form-item :label="$t('arModule.invoice.invoiceNo')">
          <el-input v-model="invoiceQuery.invoice_no" :placeholder="$t('arModule.invoice.invoiceNoPlaceholder')" clearable />
        </el-form-item>
        <el-form-item :label="$t('common.status')">
          <el-select v-model="invoiceQuery.status" :placeholder="$t('arModule.invoice.statusPlaceholder')" clearable>
            <el-option :label="$t('arModule.invoice.statusPending')" value="pending" />
            <el-option :label="$t('arModule.invoice.statusApproved')" value="approved" />
            <el-option :label="$t('arModule.invoice.statusVerified')" value="verified" />
            <el-option :label="$t('arModule.invoice.statusCancelled')" value="cancelled" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="fetchInvoices">{{ $t('common.search') }}</el-button>
          <el-button @click="resetInvoiceQuery">{{ $t('common.reset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover">
      <el-table v-loading="invoiceLoading" :data="invoices" stripe :aria-label="$t('arModule.invoice.listAria')">
        <el-table-column prop="invoice_no" :label="$t('arModule.invoice.invoiceNo')" width="140" />
        <el-table-column prop="customer_name" :label="$t('arModule.invoice.customer')" width="150" />
        <el-table-column prop="invoice_date" :label="$t('arModule.invoice.invoiceDate')" width="120" />
        <el-table-column :label="$t('arModule.invoice.invoiceAmount')" width="120" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.invoice_amount) }}
          </template>
        </el-table-column>
        <el-table-column :label="$t('arModule.invoice.taxAmount')" width="100" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.tax_amount) }}
          </template>
        </el-table-column>
        <el-table-column :label="$t('arModule.invoice.verifiedAmount')" width="110" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.verified_amount) }}
          </template>
        </el-table-column>
        <el-table-column :label="$t('arModule.invoice.unverifiedAmount')" width="110" align="right">
          <template #default="{ row }">
            <span :class="{ 'text-red': row.unverified_amount > 0 }">
              {{ formatMoney(row.unverified_amount) }}
            </span>
          </template>
        </el-table-column>
        <el-table-column prop="status" :label="$t('common.status')" width="90" align="center">
          <template #default="{ row }">
            <el-tag :type="getInvoiceStatusType(row.status)" size="small">
              {{ getInvoiceStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="due_date" :label="$t('arModule.invoice.dueDate')" width="120" />
        <el-table-column :label="$t('common.operation')" width="180" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="viewInvoice(row)">{{ $t('common.detail') }}</el-button>
            <el-button
              v-if="row.status === 'pending'"
              type="success"
              link
              size="small"
              @click="approveInvoice(row)"
              >{{ $t('arModule.invoice.approve') }}</el-button
            >
            <el-button
              v-if="row.status === 'pending'"
              type="danger"
              link
              size="small"
              @click="cancelInvoice(row)"
              >{{ $t('common.cancel') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="invoiceDialogVisible" :title="$t('arModule.invoice.createTitle')" width="600px" :aria-label="$t('arModule.invoice.createAria')">
      <el-form ref="invoiceFormRef" :model="invoiceForm" :rules="invoiceRules" label-width="80px" :aria-label="$t('arModule.invoice.formAria')">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="$t('arModule.invoice.customer')" prop="customer_id">
              <el-select
                v-model="invoiceForm.customer_id"
                :placeholder="$t('arModule.invoice.customerPlaceholder')"
                style="width: 100%"
              >
                <el-option
                  v-for="c in customers"
                  :key="c.id"
                  :label="c.customer_name"
                  :value="c.id"
                />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="$t('arModule.invoice.invoiceNo')" prop="invoice_no">
              <el-input v-model="invoiceForm.invoice_no" :placeholder="$t('arModule.invoice.invoiceNoInputPlaceholder')" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="$t('arModule.invoice.invoiceDate')" prop="invoice_date">
              <el-date-picker
                v-model="invoiceForm.invoice_date"
                type="date"
                :placeholder="$t('arModule.invoice.datePlaceholder')"
                value-format="YYYY-MM-DD"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="$t('arModule.invoice.dueDate')">
              <el-date-picker
                v-model="invoiceForm.due_date"
                type="date"
                :placeholder="$t('arModule.invoice.datePlaceholder')"
                value-format="YYYY-MM-DD"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="$t('arModule.invoice.invoiceAmount')" prop="invoice_amount">
              <el-input-number
                v-model="invoiceForm.invoice_amount"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="$t('arModule.invoice.taxAmount')">
              <el-input-number
                v-model="invoiceForm.tax_amount"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item :label="$t('arModule.invoice.remark')">
          <el-input v-model="invoiceForm.remark" type="textarea" :placeholder="$t('arModule.invoice.remarkPlaceholder')" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="invoiceDialogVisible = false">{{ $t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="invoiceSubmitLoading" @click="submitInvoice"
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
import { Plus, Printer, Download } from '@element-plus/icons-vue'
import printJS from 'print-js'
import type { FormInstance, FormRules } from 'element-plus'
import {
  getARInvoiceList,
  getARInvoice,
  createARInvoice,
  approveARInvoice,
  cancelARInvoice,
  type ARInvoice,
} from '@/api/ar'
import type { Customer } from '@/api/customer'
import { logger } from '@/utils/logger'
import { exportFromBackend } from '@/utils/export'

const { t } = useI18n({ useScope: 'global' })

const invoices = ref<ARInvoice[]>([])
const customers = ref<Customer[]>([])
const invoiceLoading = ref(false)
const invoiceSubmitLoading = ref(false)
const invoiceDialogVisible = ref(false)
const invoiceFormRef = ref<FormInstance>()

const invoiceQuery = reactive({
  customer_name: '',
  invoice_no: '',
  status: '',
})

const invoiceForm = reactive({
  customer_id: undefined as number | undefined,
  invoice_no: '',
  invoice_date: '',
  invoice_amount: 0,
  tax_amount: 0,
  due_date: '',
  remark: '',
})

const invoiceRules: FormRules = {
  customer_id: [{ required: true, message: t('arModule.invoice.customerRequired'), trigger: 'change' }],
  invoice_no: [{ required: true, message: t('arModule.invoice.invoiceNoRequired'), trigger: 'blur' }],
  invoice_date: [{ required: true, message: t('arModule.invoice.invoiceDateRequired'), trigger: 'change' }],
  invoice_amount: [{ required: true, message: t('arModule.invoice.invoiceAmountRequired'), trigger: 'blur' }],
}

const formatMoney = (amount: number) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const getInvoiceStatusLabel = (status: string) => {
  const keyMap: Record<string, string> = {
    pending: 'arModule.invoice.statusPending',
    approved: 'arModule.invoice.statusApproved',
    verified: 'arModule.invoice.statusVerified',
    cancelled: 'arModule.invoice.statusCancelled',
  }
  const key = keyMap[status]
  return key ? t(key) : status
}

const getInvoiceStatusType = (status: string) => {
  const map: Record<string, string> = {
    pending: 'warning',
    approved: 'success',
    verified: 'primary',
    cancelled: 'info',
  }
  return map[status] || 'info'
}

const fetchInvoices = async () => {
  invoiceLoading.value = true
  try {
    const res = await getARInvoiceList(invoiceQuery)
    const d = res.data as
      | { list?: ARInvoice[]; items?: ARInvoice[]; data?: ARInvoice[] }
      | ARInvoice[]
    invoices.value = Array.isArray(d) ? d : d?.items || d?.data || []
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || t('arModule.invoice.fetchListFailed'))
  } finally {
    invoiceLoading.value = false
  }
}

const resetInvoiceQuery = () => {
  invoiceQuery.customer_name = ''
  invoiceQuery.invoice_no = ''
  invoiceQuery.status = ''
  fetchInvoices()
}

const openInvoiceDialog = () => {
  invoiceFormRef.value?.resetFields()
  invoiceForm.customer_id = undefined
  invoiceForm.invoice_no = ''
  invoiceForm.invoice_date = new Date().toISOString().split('T')[0]
  invoiceForm.invoice_amount = 0
  invoiceForm.tax_amount = 0
  invoiceForm.due_date = ''
  invoiceForm.remark = ''
  invoiceDialogVisible.value = true
}

const submitInvoice = async () => {
  const valid = await invoiceFormRef.value?.validate()
  if (!valid) return

  invoiceSubmitLoading.value = true
  try {
    await createARInvoice(invoiceForm)
    ElMessage.success(t('common.success'))
    invoiceDialogVisible.value = false
    fetchInvoices()
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || t('common.failed'))
  } finally {
    invoiceSubmitLoading.value = false
  }
}

// 批次 157a P1-1 修复：接入 getARInvoice API 展示应收发票详情
const viewInvoice = async (row: ARInvoice) => {
  try {
    const res = await getARInvoice(row.id)
    const d = res.data
    if (!d) {
      ElMessage.warning(t('arModule.invoice.notFoundDetail'))
      return
    }
    const lines = [
      t('arModule.invoice.detailNo', { value: d.invoice_no }),
      t('arModule.invoice.detailCustomer', { value: d.customer_name }),
      t('arModule.invoice.detailDate', { value: d.invoice_date }),
      t('arModule.invoice.detailDueDate', { value: d.due_date || '-' }),
      t('arModule.invoice.detailAmount', { value: formatMoney(d.invoice_amount) }),
      t('arModule.invoice.detailTax', { value: formatMoney(d.tax_amount) }),
      t('arModule.invoice.detailVerified', { value: formatMoney(d.verified_amount) }),
      t('arModule.invoice.detailUnverified', { value: formatMoney(d.unverified_amount) }),
      t('arModule.invoice.detailStatus', { value: getInvoiceStatusLabel(d.status) }),
      t('arModule.invoice.detailRemark', { value: d.remark || '-' }),
    ]
    await ElMessageBox.alert(lines.join('\n'), t('arModule.invoice.detailTitle'), {
      confirmButtonText: t('common.close'),
    })
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || t('arModule.invoice.fetchDetailFailed'))
  }
}

const approveInvoice = async (row: ARInvoice) => {
  try {
    await ElMessageBox.confirm(t('arModule.invoice.approveConfirm'), t('arModule.invoice.approveTitle'), { type: 'info' })
    await approveARInvoice(row.id)
    ElMessage.success(t('arModule.invoice.approveSuccess'))
    fetchInvoices()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || t('common.failed'))
    }
  }
}

const cancelInvoice = async (row: ARInvoice) => {
  try {
    await ElMessageBox.confirm(t('arModule.invoice.cancelConfirm'), t('arModule.invoice.cancelTitle'), { type: 'warning' })
    await cancelARInvoice(row.id)
    ElMessage.success(t('arModule.invoice.cancelSuccess'))
    fetchInvoices()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || t('common.failed'))
    }
  }
}

const handlePrintInvoices = () => {
  const printData = invoices.value.map((item, index) => ({
    [t('arModule.invoice.colSeq')]: index + 1,
    [t('arModule.invoice.invoiceNo')]: item.invoice_no,
    [t('arModule.invoice.customer')]: item.customer_name,
    [t('arModule.invoice.invoiceAmount')]: `¥${item.invoice_amount}`,
    [t('arModule.invoice.taxAmount')]: `¥${item.tax_amount}`,
    [t('common.status')]: getInvoiceStatusLabel(item.status),
    [t('arModule.invoice.invoiceDate')]: item.invoice_date,
  }))
  printJS({
    printable: printData,
    properties: Object.keys(printData[0] || {}) as string[],
    type: 'json',
    header: t('arModule.invoice.printHeader'),
    style: 'padding: 20px; font-size: 14px;',
    headerStyle: 'font-size: 18px; font-weight: bold; margin-bottom: 20px;',
    gridHeaderStyle: 'font-weight: bold; background-color: #f5f7fa;',
    gridStyle: 'border-collapse: collapse; width: 100%;',
  } as never)
}

const handleExportInvoices = async () => {
  const params: Record<string, unknown> = {
    status: invoiceQuery.status || undefined,
  }
  await exportFromBackend('/ar/invoices/export', params, 'ar_invoices_export')
  logger.info(t('arModule.invoice.exportedLog'))
}

onMounted(() => {
  fetchInvoices()
})
</script>

<style scoped>
.text-red {
  color: #f56c6c;
}
</style>
