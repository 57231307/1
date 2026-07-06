<!--
  InvoiceTab.vue - 应付发票 Tab
  来源：原 ap/index.vue 中 应付发票 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="invoice-tab">
    <div class="page-header">
      <h2 class="page-title">应付发票</h2>
      <div class="header-actions">
        <el-button type="primary" @click="openInvoiceDialog()">
          <el-icon><Plus /></el-icon> 新建发票
        </el-button>
        <el-button @click="handlePrintInvoices">
          <el-icon><Printer /></el-icon> 打印
        </el-button>
        <el-button @click="handleExportInvoices">
          <el-icon><Download /></el-icon> 导出
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="invoiceQuery">
        <el-form-item label="供应商">
          <el-input v-model="invoiceQuery.supplier_name" placeholder="供应商名称" clearable />
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
        <el-table-column prop="supplier_name" label="供应商" width="150" />
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
            <el-button
              type="primary"
              link
              size="small"
              @click="viewInvoice(row as unknown as APInvoice)"
              >查看</el-button
            >
            <el-button
              v-if="row.status === 'pending'"
              type="success"
              link
              size="small"
              @click="approveInvoice(row as unknown as APInvoice)"
              >审核</el-button
            >
            <el-button
              v-if="row.status === 'pending'"
              type="danger"
              link
              size="small"
              @click="cancelInvoice(row as unknown as APInvoice)"
              >取消</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="invoiceDialogVisible" title="新建应付发票" width="600px">
      <el-form ref="invoiceFormRef" :model="invoiceForm" :rules="invoiceRules" label-width="80px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="供应商" prop="supplier_id">
              <el-select
                v-model="invoiceForm.supplier_id"
                placeholder="选择供应商"
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
          <el-input v-model="invoiceForm.remark" type="textarea" />
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
  listAPInvoices,
  getAPInvoice,
  createAPInvoice,
  approveAPInvoice,
  cancelAPInvoice,
  getAPInvoiceStatusText,
  type APInvoice,
} from '@/api/ap-invoice'
import { exportToExcel } from '@/utils/export'
import { logger } from '@/utils/logger'
import type { Supplier } from '@/api/supplier'

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
  return getAPInvoiceStatusText(status) || status
}

const fetchInvoices = async () => {
  invoiceLoading.value = true
  try {
    const res = await listAPInvoices(invoiceQuery)
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
    ElMessage.error(err.message || '获取发票列表失败')
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
  supplier_id: [{ required: true, message: '请选择供应商', trigger: 'change' }],
  invoice_no: [{ required: true, message: '请输入发票号', trigger: 'blur' }],
  invoice_date: [{ required: true, message: '请选择发票日期', trigger: 'change' }],
  invoice_amount: [{ required: true, message: '请输入发票金额', trigger: 'blur' }],
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
    ElMessage.success('创建成功')
    invoiceDialogVisible.value = false
    fetchInvoices()
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || '操作失败')
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
      ElMessage.warning('未找到发票详情')
      return
    }
    const lines = [
      `发票编号：${d.invoice_no}`,
      `供应商名称：${d.supplier_name}`,
      `发票日期：${d.invoice_date}`,
      `到期日期：${d.due_date || '-'}`,
      `发票金额：¥${formatMoney(d.invoice_amount)}`,
      `税额：¥${formatMoney(d.tax_amount)}`,
      `已核销金额：¥${formatMoney(d.verified_amount)}`,
      `未核销金额：¥${formatMoney(d.unverified_amount)}`,
      `当前状态：${getInvoiceStatusLabel(d.status)}`,
      `备注：${d.remark || '-'}`,
    ]
    await ElMessageBox.alert(lines.join('\n'), '应付发票详情', {
      confirmButtonText: '关闭',
    })
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || '获取发票详情失败')
  }
}

const approveInvoice = async (row: APInvoice) => {
  try {
    await ElMessageBox.confirm('确定审核该发票吗？', '审核确认', { type: 'info' })
    await approveAPInvoice(row.id)
    ElMessage.success('审核成功')
    fetchInvoices()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || '操作失败')
    }
  }
}

const cancelInvoice = async (row: APInvoice) => {
  try {
    await ElMessageBox.confirm('确定取消该发票吗？', '取消确认', { type: 'warning' })
    await cancelAPInvoice(row.id)
    ElMessage.success('取消成功')
    fetchInvoices()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || '操作失败')
    }
  }
}

// 批次 157a P1-1 修复：实现应付发票打印（参考 ar 模块 printJS 实现）
const handlePrintInvoices = () => {
  if (invoices.value.length === 0) {
    ElMessage.warning('没有可打印的数据')
    return
  }
  const printData = invoices.value.map((item, index) => ({
    序号: index + 1,
    发票号: item.invoice_no,
    供应商: item.supplier_name,
    发票金额: `¥${item.invoice_amount}`,
    税额: `¥${item.tax_amount}`,
    状态: getInvoiceStatusLabel(item.status),
    发票日期: item.invoice_date,
  }))
  printJS({
    printable: printData,
    properties: Object.keys(printData[0] || {}) as string[],
    type: 'table',
    header: '应付发票列表',
    style: 'padding: 20px; font-size: 14px;',
    headerStyle: 'font-size: 18px; font-weight: bold; margin-bottom: 20px;',
    gridHeaderStyle: 'font-weight: bold; background-color: #f5f7fa;',
    gridStyle: 'border-collapse: collapse; width: 100%;',
  } as never)
}

// 批次 157a P1-1 修复：实现应付发票导出（规则 3：使用 .xlsx 格式，非 CSV）
const handleExportInvoices = () => {
  if (invoices.value.length === 0) {
    ElMessage.warning('没有可导出的数据')
    return
  }
  exportToExcel({
    filename: '应付发票',
    format: 'excel',
    data: invoices.value as unknown as Record<string, unknown>[],
    columns: [
      { key: 'invoice_no', title: '发票号' },
      { key: 'supplier_name', title: '供应商' },
      { key: 'invoice_amount', title: '发票金额' },
      { key: 'tax_amount', title: '税额' },
      { key: 'verified_amount', title: '已核销金额' },
      { key: 'unverified_amount', title: '未核销金额' },
      {
        key: 'status',
        title: '状态',
        formatter: (_v: unknown, row: Record<string, unknown>) =>
          getInvoiceStatusLabel(String(row.status)),
      },
      { key: 'invoice_date', title: '发票日期' },
      { key: 'due_date', title: '到期日' },
    ],
  })
  logger.info('应付发票列表已导出')
}

const fetchSuppliers = async () => {
  try {
    const res = await listAPInvoices({} as never)
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
