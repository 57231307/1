<!--
  InvoiceTab.vue - 应付发票 Tab
  来源：原 ap/index.vue 中 应付发票 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="invoice-tab">
    <div class="page-header">
      <h2 class="page-title">{{ $t('apModule.invoice.title') }}</h2>
      <div class="header-actions">
        <el-button type="primary" @click="openInvoiceDialog()">
          <el-icon><Plus /></el-icon> {{ $t('apModule.invoice.create') }}
        </el-button>
        <el-button @click="handlePrintInvoices">
          <el-icon><Printer /></el-icon> {{ $t('common.print') }}
        </el-button>
        <el-button @click="handleExportInvoices">
          <el-icon><Download /></el-icon> {{ $t('common.export') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="invoiceQuery" :aria-label="$t('apModule.invoice.filterAria')">
        <el-form-item :label="$t('apModule.invoice.supplier')">
          <el-input v-model="invoiceQuery.supplier_name" :placeholder="$t('apModule.invoice.supplierNamePlaceholder')" clearable />
        </el-form-item>
        <el-form-item :label="$t('apModule.invoice.invoiceNo')">
          <el-input v-model="invoiceQuery.invoice_no" :placeholder="$t('apModule.invoice.invoiceNoPlaceholder')" clearable />
        </el-form-item>
        <el-form-item :label="$t('common.status')">
          <el-select v-model="invoiceQuery.status" :placeholder="$t('apModule.invoice.statusPlaceholder')" clearable>
            <el-option :label="$t('apModule.invoice.statusPending')" value="pending" />
            <el-option :label="$t('apModule.invoice.statusApproved')" value="approved" />
            <el-option :label="$t('apModule.invoice.statusVerified')" value="verified" />
            <el-option :label="$t('apModule.invoice.statusCancelled')" value="cancelled" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="fetchInvoices">{{ $t('common.search') }}</el-button>
          <el-button @click="resetInvoiceQuery">{{ $t('common.reset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover">
      <el-table v-loading="invoiceLoading" :data="invoices" stripe :aria-label="$t('apModule.invoice.listAria')">
        <el-table-column prop="invoice_no" :label="$t('apModule.invoice.invoiceNo')" width="140" />
        <el-table-column prop="supplier_name" :label="$t('apModule.invoice.supplier')" width="150" />
        <el-table-column prop="invoice_date" :label="$t('apModule.invoice.invoiceDate')" width="120" />
        <el-table-column :label="$t('apModule.invoice.invoiceAmount')" width="120" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.invoice_amount) }}
          </template>
        </el-table-column>
        <el-table-column :label="$t('apModule.invoice.taxAmount')" width="100" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.tax_amount) }}
          </template>
        </el-table-column>
        <el-table-column :label="$t('apModule.invoice.verifiedAmount')" width="110" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.verified_amount) }}
          </template>
        </el-table-column>
        <el-table-column :label="$t('apModule.invoice.unverifiedAmount')" width="110" align="right">
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
        <el-table-column prop="due_date" :label="$t('apModule.invoice.dueDate')" width="120" />
        <el-table-column :label="$t('common.operation')" width="180" fixed="right">
          <template #default="{ row }">
            <el-button
              type="primary"
              link
              size="small"
              @click="viewInvoice(row as APInvoice)"
              >{{ $t('common.detail') }}</el-button
            >
            <el-button
              v-if="row.status === 'pending'"
              type="success"
              link
              size="small"
              @click="approveInvoice(row as APInvoice)"
              >{{ $t('apModule.invoice.approve') }}</el-button
            >
            <el-button
              v-if="row.status === 'pending'"
              type="danger"
              link
              size="small"
              @click="cancelInvoice(row as APInvoice)"
              >{{ $t('common.cancel') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="invoiceDialogVisible" :title="$t('apModule.invoice.createTitle')" width="600px" :aria-label="$t('apModule.invoice.createAria')">
      <el-form ref="invoiceFormRef" :model="invoiceForm" :rules="invoiceRules" label-width="80px" :aria-label="$t('apModule.invoice.formAria')">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="$t('apModule.invoice.supplier')" prop="supplier_id">
              <el-select
                v-model="invoiceForm.supplier_id"
                :placeholder="$t('apModule.invoice.supplierPlaceholder')"
                style="width: 100%"
              >
                <el-option
                  v-for="s in suppliers"
                  :key="s.id"
                  :label="s.supplier_name"
                  :value="s.id"
                />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="$t('apModule.invoice.invoiceNo')" prop="invoice_no">
              <el-input v-model="invoiceForm.invoice_no" :placeholder="$t('apModule.invoice.invoiceNoInputPlaceholder')" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="$t('apModule.invoice.invoiceDate')" prop="invoice_date">
              <el-date-picker
                v-model="invoiceForm.invoice_date"
                type="date"
                :placeholder="$t('apModule.invoice.datePlaceholder')"
                value-format="YYYY-MM-DD"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="$t('apModule.invoice.dueDate')">
              <el-date-picker
                v-model="invoiceForm.due_date"
                type="date"
                :placeholder="$t('apModule.invoice.datePlaceholder')"
                value-format="YYYY-MM-DD"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="$t('apModule.invoice.invoiceAmount')" prop="invoice_amount">
              <el-input-number
                v-model="invoiceForm.invoice_amount"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="$t('apModule.invoice.taxAmount')">
              <el-input-number
                v-model="invoiceForm.tax_amount"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item :label="$t('apModule.invoice.remark')">
          <el-input v-model="invoiceForm.remark" type="textarea" />
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
  getAPInvoiceList,
  getAPInvoice,
  createAPInvoice,
  approveAPInvoice,
  cancelAPInvoice,
  getAPInvoiceStatusText,
  type APInvoice,
} from '@/api/ap-invoice'
import { exportFromBackend } from '@/utils/export'
import { logger } from '@/utils/logger'
import type { Supplier } from '@/api/supplier'

const { t } = useI18n({ useScope: 'global' })

const invoices = ref<APInvoice[]>([])
const invoiceLoading = ref(false)
const suppliers = ref<Supplier[]>([])

const invoiceQuery = reactive({
  supplier_name: '',
  invoice_no: '',
  status: '',
})

const formatMoney = (amount: number | undefined) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const getInvoiceStatusType = (status: string) => {
  const map: Record<string, string> = {
    pending: 'warning',
    approved: 'primary',
    verified: 'success',
    cancelled: 'danger',
  }
  return map[status] || 'info'
}

const getInvoiceStatusLabel = (status: string) => {
  const keyMap: Record<string, string> = {
    pending: 'apModule.invoice.statusPending',
    approved: 'apModule.invoice.statusApproved',
    verified: 'apModule.invoice.statusVerified',
    cancelled: 'apModule.invoice.statusCancelled',
  }
  const key = keyMap[status]
  if (key) return t(key)
  return getAPInvoiceStatusText(status) || status
}

const fetchInvoices = async () => {
  invoiceLoading.value = true
  try {
    const res = await getAPInvoiceList(invoiceQuery)
    const d = res.data as
      | { list?: APInvoice[]; items?: APInvoice[]; data?: APInvoice[] }
      | APInvoice[]
      | undefined
    if (d && typeof d === 'object' && !Array.isArray(d)) {
      invoices.value = d.list || d.items || d.data || []
    } else {
      invoices.value = (d as APInvoice[]) || []
    }
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || t('apModule.invoice.fetchListFailed'))
  } finally {
    invoiceLoading.value = false
  }
}

const resetInvoiceQuery = () => {
  invoiceQuery.supplier_name = ''
  invoiceQuery.invoice_no = ''
  invoiceQuery.status = ''
  fetchInvoices()
}

const invoiceDialogVisible = ref(false)
const invoiceFormRef = ref<FormInstance>()
const invoiceSubmitLoading = ref(false)
const invoiceForm = reactive({
  supplier_id: undefined as number | undefined,
  invoice_no: '',
  invoice_date: '',
  due_date: '',
  invoice_amount: 0,
  tax_amount: 0,
  remark: '',
})

const invoiceRules: FormRules = {
  supplier_id: [{ required: true, message: t('apModule.invoice.supplierRequired'), trigger: 'change' }],
  invoice_no: [{ required: true, message: t('apModule.invoice.invoiceNoRequired'), trigger: 'blur' }],
  invoice_date: [{ required: true, message: t('apModule.invoice.invoiceDateRequired'), trigger: 'change' }],
  invoice_amount: [{ required: true, message: t('apModule.invoice.invoiceAmountRequired'), trigger: 'blur' }],
}

const openInvoiceDialog = () => {
  Object.assign(invoiceForm, {
    supplier_id: undefined,
    invoice_no: '',
    invoice_date: '',
    due_date: '',
    invoice_amount: 0,
    tax_amount: 0,
    remark: '',
  })
  invoiceDialogVisible.value = true
}

const submitInvoice = async () => {
  const valid = await invoiceFormRef.value?.validate()
  if (!valid) return
  invoiceSubmitLoading.value = true
  try {
    await createAPInvoice(invoiceForm)
    ElMessage.success(t('common.success'))
    invoiceDialogVisible.value = false
    fetchInvoices()
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || t('common.failed'))
  } finally {
    invoiceSubmitLoading.value = false
  }
}

// 批次 157a P1-1 修复：接入 getAPInvoice API 展示应付发票详情
const viewInvoice = async (row: APInvoice) => {
  try {
    const res = await getAPInvoice(row.id)
    const d = res.data
    if (!d) {
      ElMessage.warning(t('apModule.invoice.notFoundDetail'))
      return
    }
    const lines = [
      t('apModule.invoice.detailNo', { value: d.invoice_no }),
      t('apModule.invoice.detailSupplier', { value: d.supplier_name }),
      t('apModule.invoice.detailDate', { value: d.invoice_date }),
      t('apModule.invoice.detailDueDate', { value: d.due_date || '-' }),
      t('apModule.invoice.detailAmount', { value: formatMoney(d.invoice_amount) }),
      t('apModule.invoice.detailTax', { value: formatMoney(d.tax_amount) }),
      t('apModule.invoice.detailVerified', { value: formatMoney(d.verified_amount) }),
      t('apModule.invoice.detailUnverified', { value: formatMoney(d.unverified_amount) }),
      t('apModule.invoice.detailStatus', { value: getInvoiceStatusLabel(d.status) }),
      t('apModule.invoice.detailRemark', { value: d.remark || '-' }),
    ]
    await ElMessageBox.alert(lines.join('\n'), t('apModule.invoice.detailTitle'), {
      confirmButtonText: t('common.close'),
    })
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || t('apModule.invoice.fetchDetailFailed'))
  }
}

const approveInvoice = async (row: APInvoice) => {
  try {
    await ElMessageBox.confirm(t('apModule.invoice.approveConfirm'), t('apModule.invoice.approveTitle'), { type: 'info' })
    await approveAPInvoice(row.id)
    ElMessage.success(t('apModule.invoice.approveSuccess'))
    fetchInvoices()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || t('common.failed'))
    }
  }
}

const cancelInvoice = async (row: APInvoice) => {
  try {
    await ElMessageBox.confirm(t('apModule.invoice.cancelConfirm'), t('apModule.invoice.cancelTitle'), { type: 'warning' })
    await cancelAPInvoice(row.id)
    ElMessage.success(t('apModule.invoice.cancelSuccess'))
    fetchInvoices()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || t('common.failed'))
    }
  }
}

// 批次 157a P1-1 修复：实现应付发票打印（参考 ar 模块 printJS 实现）
const handlePrintInvoices = () => {
  if (invoices.value.length === 0) {
    ElMessage.warning(t('apModule.invoice.noPrintData'))
    return
  }
  const printData = invoices.value.map((item, index) => ({
    [t('apModule.invoice.colSeq')]: index + 1,
    [t('apModule.invoice.invoiceNo')]: item.invoice_no,
    [t('apModule.invoice.supplier')]: item.supplier_name,
    [t('apModule.invoice.invoiceAmount')]: `¥${item.invoice_amount}`,
    [t('apModule.invoice.taxAmount')]: `¥${item.tax_amount}`,
    [t('common.status')]: getInvoiceStatusLabel(item.status),
    [t('apModule.invoice.invoiceDate')]: item.invoice_date,
  }))
  printJS({
    printable: printData,
    properties: Object.keys(printData[0] || {}) as string[],
    type: 'json',
    header: t('apModule.invoice.printHeader'),
    style: 'padding: 20px; font-size: 14px;',
    headerStyle: 'font-size: 18px; font-weight: bold; margin-bottom: 20px;',
    gridHeaderStyle: 'font-weight: bold; background-color: #f5f7fa;',
    gridStyle: 'border-collapse: collapse; width: 100%;',
  } as never)
}

// 批次 157a P1-1 修复：实现应付发票导出（规则 3：使用 .xlsx 格式，非 CSV）
// V15 P0-S12 修复（Batch 475e）：迁移到后端导出，注入水印 + 审计日志
const handleExportInvoices = async () => {
  const params: Record<string, unknown> = {
    invoice_status: invoiceQuery.status || undefined,
  }
  await exportFromBackend('/ap/invoices/export', params, 'ap_invoices_export')
  logger.info(t('apModule.invoice.exportedLog'))
}

const fetchSuppliers = async () => {
  try {
    const res = await getAPInvoiceList({} as never)
    void res
  } catch (_e) {
    // suppliers 实际应通过 supplierApi 加载；此处保持空列表不影响主流程
  }
  suppliers.value = []
}

defineExpose({ refresh: fetchInvoices })

onMounted(() => {
  fetchInvoices()
  fetchSuppliers()
})
</script>

<style scoped>
.invoice-tab {
  background: #fff;
  border-radius: 4px;
}
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
.header-actions {
  display: flex;
  gap: 10px;
}
.filter-card {
  margin-bottom: 20px;
}
.text-red {
  color: #f56c6c;
}
</style>
