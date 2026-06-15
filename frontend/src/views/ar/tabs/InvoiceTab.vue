<!--
  InvoiceTab.vue - 应收发票 Tab
  来源：原 ar/index.vue 中 应收发票 tab 内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="invoice-tab">
    <div class="page-header">
      <h2 class="page-title">应收发票</h2>
      <div class="header-actions">
        <el-button type="primary" @click="openInvoiceDialog()">
          <el-icon><Plus /></el-icon>
          新建发票
        </el-button>
        <el-button @click="handlePrintInvoices">
          <el-icon><Printer /></el-icon>
          打印
        </el-button>
        <el-button @click="handleExportInvoices">
          <el-icon><Download /></el-icon>
          导出
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="invoiceQuery">
        <el-form-item label="客户">
          <el-input v-model="invoiceQuery.customer_name" placeholder="客户名称" clearable />
        </el-form-item>
        <el-form-item label="发票号">
          <el-input v-model="invoiceQuery.invoice_no" placeholder="发票号" clearable />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="invoiceQuery.status" placeholder="选择状态" clearable>
            <el-option label="待审核" value="pending" />
            <el-option label="已审核" value="approved" />
            <el-option label="已核销" value="verified" />
            <el-option label="已取消" value="cancelled" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="fetchInvoices">查询</el-button>
          <el-button @click="resetInvoiceQuery">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover">
      <el-table v-loading="invoiceLoading" :data="invoices" stripe>
        <el-table-column prop="invoice_no" label="发票号" width="140" />
        <el-table-column prop="customer_name" label="客户" width="150" />
        <el-table-column prop="invoice_date" label="发票日期" width="120" />
        <el-table-column label="发票金额" width="120" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.invoice_amount) }}
          </template>
        </el-table-column>
        <el-table-column label="税额" width="100" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.tax_amount) }}
          </template>
        </el-table-column>
        <el-table-column label="已核销金额" width="110" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.verified_amount) }}
          </template>
        </el-table-column>
        <el-table-column label="未核销金额" width="110" align="right">
          <template #default="{ row }">
            <span :class="{ 'text-red': row.unverified_amount > 0 }">
              {{ formatMoney(row.unverified_amount) }}
            </span>
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="90" align="center">
          <template #default="{ row }">
            <el-tag :type="getInvoiceStatusType(row.status)" size="small">
              {{ getInvoiceStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="due_date" label="到期日" width="120" />
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="viewInvoice(row)">查看</el-button>
            <el-button
              v-if="row.status === 'pending'"
              type="success"
              link
              size="small"
              @click="approveInvoice(row)"
              >审核</el-button
            >
            <el-button
              v-if="row.status === 'pending'"
              type="danger"
              link
              size="small"
              @click="cancelInvoice(row)"
              >取消</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="invoiceDialogVisible" title="新建应收发票" width="600px">
      <el-form ref="invoiceFormRef" :model="invoiceForm" :rules="invoiceRules" label-width="80px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="客户" prop="customer_id">
              <el-select
                v-model="invoiceForm.customer_id"
                placeholder="选择客户"
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
            <el-form-item label="发票号" prop="invoice_no">
              <el-input v-model="invoiceForm.invoice_no" placeholder="请输入发票号" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="发票日期" prop="invoice_date">
              <el-date-picker
                v-model="invoiceForm.invoice_date"
                type="date"
                placeholder="选择日期"
                value-format="YYYY-MM-DD"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="到期日">
              <el-date-picker
                v-model="invoiceForm.due_date"
                type="date"
                placeholder="选择日期"
                value-format="YYYY-MM-DD"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="发票金额" prop="invoice_amount">
              <el-input-number
                v-model="invoiceForm.invoice_amount"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="税额">
              <el-input-number
                v-model="invoiceForm.tax_amount"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="备注">
          <el-input v-model="invoiceForm.remark" type="textarea" placeholder="请输入备注" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="invoiceDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="invoiceSubmitLoading" @click="submitInvoice"
          >确定</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Printer, Download } from '@element-plus/icons-vue'
import printJS from 'print-js'
import type { FormInstance, FormRules } from 'element-plus'
import {
  listARInvoices,
  createARInvoice,
  approveARInvoice,
  cancelARInvoice,
  type ARInvoice,
} from '@/api/ar'
import type { Customer } from '@/api/customer'
import { logger } from '@/utils/logger'

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
  customer_id: [{ required: true, message: '请选择客户', trigger: 'change' }],
  invoice_no: [{ required: true, message: '请输入发票号', trigger: 'blur' }],
  invoice_date: [{ required: true, message: '请选择发票日期', trigger: 'change' }],
  invoice_amount: [{ required: true, message: '请输入发票金额', trigger: 'blur' }],
}

const formatMoney = (amount: number) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const getInvoiceStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    pending: '待审核',
    approved: '已审核',
    verified: '已核销',
    cancelled: '已取消',
  }
  return map[status] || status
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
    const res = await listARInvoices(invoiceQuery)
    const d = res.data as
      | { list?: ARInvoice[]; items?: ARInvoice[]; data?: ARInvoice[] }
      | ARInvoice[]
    invoices.value = Array.isArray(d) ? d : d?.items || d?.data || []
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '获取发票列表失败')
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
    ElMessage.success('创建成功')
    invoiceDialogVisible.value = false
    fetchInvoices()
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '操作失败')
  } finally {
    invoiceSubmitLoading.value = false
  }
}

const viewInvoice = (row: ARInvoice) => {
  ElMessage.info(`查看发票: ${row.invoice_no}`)
}

const approveInvoice = async (row: ARInvoice) => {
  try {
    await ElMessageBox.confirm('确定审核该发票吗？', '审核确认', { type: 'info' })
    await approveARInvoice(row.id)
    ElMessage.success('审核成功')
    fetchInvoices()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || '操作失败')
    }
  }
}

const cancelInvoice = async (row: ARInvoice) => {
  try {
    await ElMessageBox.confirm('确定取消该发票吗？', '取消确认', { type: 'warning' })
    await cancelARInvoice(row.id)
    ElMessage.success('取消成功')
    fetchInvoices()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || '操作失败')
    }
  }
}

const handlePrintInvoices = () => {
  const printData = invoices.value.map((item, index) => ({
    序号: index + 1,
    发票号: item.invoice_no,
    客户: item.customer_name,
    发票金额: `¥${item.invoice_amount}`,
    税额: `¥${item.tax_amount}`,
    状态: getInvoiceStatusLabel(item.status),
    发票日期: item.invoice_date,
  }))
  printJS({
    printable: printData,
    properties: Object.keys(printData[0] || {}) as string[],
    type: 'table',
    header: '应收发票列表',
    style: 'padding: 20px; font-size: 14px;',
    headerStyle: 'font-size: 18px; font-weight: bold; margin-bottom: 20px;',
    gridHeaderStyle: 'font-weight: bold; background-color: #f5f7fa;',
    gridStyle: 'border-collapse: collapse; width: 100%;',
  } as never)
}

const handleExportInvoices = () => {
  const csvContent = [
    ['发票号', '客户', '发票金额', '税额', '状态', '发票日期'],
    ...invoices.value.map(item => [
      item.invoice_no,
      item.customer_name,
      item.invoice_amount,
      item.tax_amount,
      getInvoiceStatusLabel(item.status),
      item.invoice_date,
    ]),
  ]
    .map(row => row.map(cell => `"${cell}"`).join(','))
    .join('\n')
  const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' })
  const link = document.createElement('a')
  link.href = URL.createObjectURL(blob)
  link.download = `应收发票_${new Date().toISOString().split('T')[0]}.csv`
  link.click()
  ElMessage.success('导出成功')
  logger.info('应收发票列表已导出')
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
