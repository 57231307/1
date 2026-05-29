<template>
  <div class="ap-page">
    <el-tabs v-model="activeTab">
      <el-tab-pane label="应付发票" name="invoice">
        <div class="page-header">
          <h2 class="page-title">应付发票</h2>
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
                <el-button type="primary" link size="small" @click="viewInvoice(row)"
                  >查看</el-button
                >
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
      </el-tab-pane>

      <el-tab-pane label="付款管理" name="payment">
        <div class="page-header">
          <h2 class="page-title">付款管理</h2>
          <el-button type="primary" @click="openPaymentDialog()">
            <el-icon><Plus /></el-icon>
            新建付款
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table v-loading="paymentLoading" :data="payments" stripe>
            <el-table-column prop="payment_no" label="付款单号" width="140" />
            <el-table-column prop="supplier_name" label="供应商" width="150" />
            <el-table-column prop="payment_date" label="付款日期" width="120" />
            <el-table-column label="付款金额" width="120" align="right">
              <template #default="{ row }">
                {{ formatMoney(row.payment_amount) }}
              </template>
            </el-table-column>
            <el-table-column prop="payment_method" label="付款方式" width="100">
              <template #default="{ row }">
                {{ getPaymentMethodLabel(row.payment_method) }}
              </template>
            </el-table-column>
            <el-table-column prop="status" label="状态" width="90" align="center">
              <template #default="{ row }">
                <el-tag :type="row.status === 'confirmed' ? 'success' : 'warning'" size="small">
                  {{ row.status === 'confirmed' ? '已确认' : '待确认' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="bank_account" label="银行账户" width="150" />
            <el-table-column prop="created_at" label="创建时间" width="160" />
            <el-table-column label="操作" width="120" fixed="right">
              <template #default="{ row }">
                <el-button
                  v-if="row.status !== 'confirmed'"
                  type="success"
                  link
                  size="small"
                  @click="confirmPayment(row)"
                  >确认</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="核销管理" name="verification">
        <div class="page-header">
          <h2 class="page-title">核销管理</h2>
          <el-button type="primary" @click="openVerificationDialog()">
            <el-icon><Plus /></el-icon>
            新建核销
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table v-loading="verificationLoading" :data="verifications" stripe>
            <el-table-column prop="verification_no" label="核销单号" width="140" />
            <el-table-column prop="invoice_no" label="发票号" width="140" />
            <el-table-column prop="payment_no" label="付款单号" width="140" />
            <el-table-column prop="verification_date" label="核销日期" width="120" />
            <el-table-column label="核销金额" width="120" align="right">
              <template #default="{ row }">
                {{ formatMoney(row.verification_amount) }}
              </template>
            </el-table-column>
            <el-table-column prop="status" label="状态" width="90" align="center">
              <template #default="{ row }">
                <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
                  {{ row.status === 'active' ? '有效' : '已取消' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="created_at" label="创建时间" width="160" />
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="对账管理" name="reconciliation">
        <div class="page-header">
          <h2 class="page-title">对账管理</h2>
          <el-button type="primary" @click="generateReconciliation()">
            <el-icon><Plus /></el-icon>
            生成对账单
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table v-loading="reconciliationLoading" :data="reconciliations" stripe>
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
                  @click="confirmReconciliation(row)"
                  >确认</el-button
                >
                <el-button
                  v-if="row.status === 'pending'"
                  type="warning"
                  link
                  size="small"
                  @click="disputeReconciliation(row)"
                  >异议</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>
    </el-tabs>

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

    <el-dialog v-model="paymentDialogVisible" title="新建付款" width="600px">
      <el-form ref="paymentFormRef" :model="paymentForm" :rules="paymentRules" label-width="80px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="供应商" prop="supplier_id">
              <el-select
                v-model="paymentForm.supplier_id"
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
            <el-form-item label="付款日期" prop="payment_date">
              <el-date-picker
                v-model="paymentForm.payment_date"
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
            <el-form-item label="付款金额" prop="payment_amount">
              <el-input-number
                v-model="paymentForm.payment_amount"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="付款方式" prop="payment_method">
              <el-select
                v-model="paymentForm.payment_method"
                placeholder="选择方式"
                style="width: 100%"
              >
                <el-option label="银行转账" value="bank_transfer" />
                <el-option label="现金" value="cash" />
                <el-option label="支票" value="check" />
                <el-option label="承兑汇票" value="acceptance_bill" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="银行账户">
          <el-input v-model="paymentForm.bank_account" placeholder="请输入银行账户" />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="paymentForm.remark" type="textarea" placeholder="请输入备注" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="paymentDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="paymentSubmitLoading" @click="submitPayment"
          >确定</el-button
        >
      </template>
    </el-dialog>

    <el-dialog v-model="verificationDialogVisible" title="核销" width="700px">
      <el-form label-width="80px">
        <el-form-item label="选择发票">
          <el-select
            v-model="verificationForm.invoice_id"
            placeholder="选择待核销发票"
            style="width: 100%"
          >
            <el-option
              v-for="inv in unverifiedInvoices"
              :key="inv.id"
              :label="`${inv.invoice_no} - ${formatMoney(inv.unverified_amount)}`"
              :value="inv.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="选择付款">
          <el-select
            v-model="verificationForm.payment_id"
            placeholder="选择付款记录"
            style="width: 100%"
          >
            <el-option
              v-for="p in unverifiedPayments"
              :key="p.id"
              :label="`${p.payment_no} - ${formatMoney(p.payment_amount)}`"
              :value="p.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="核销金额">
          <el-input-number
            v-model="verificationForm.amount"
            :min="0"
            :precision="2"
            style="width: 200px"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="verificationDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="verificationSubmitLoading" @click="submitVerification"
          >确定</el-button
        >
      </template>
    </el-dialog>

    <!-- 生成对账单对话框 -->
    <el-dialog v-model="reconciliationDialogVisible" title="生成对账单" width="500px">
      <el-form :model="reconciliationForm" label-width="100px">
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
            placeholder="选择开始日期"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="结束日期" required>
          <el-date-picker
            v-model="reconciliationForm.end_date"
            type="date"
            placeholder="选择结束日期"
            style="width: 100%"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <span class="dialog-footer">
          <el-button @click="reconciliationDialogVisible = false">取消</el-button>
          <el-button type="primary" @click="submitReconciliation">确定</el-button>
        </span>
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
  createAPInvoice,
  approveAPInvoice,
  cancelAPInvoice,
  type APInvoice,
} from '@/api/ap'
import { listAPPayments, createAPPayment, confirmAPPayment, type APPayment } from '@/api/ap'
import {
  listAPVerifications,
  manualVerifyAP,
  getUnverifiedAPInvoices,
  getUnverifiedAPPayments,
  type APVerification,
} from '@/api/ap'
import {
  listAPReconciliations,
  confirmAPReconciliation,
  disputeAPReconciliation,
  generateAPReconciliation,
  type APReconciliation,
} from '@/api/ap'
import { listSuppliers, type Supplier } from '@/api/supplier'

const activeTab = ref('invoice')

const invoices = ref<APInvoice[]>([])
const payments = ref<APPayment[]>([])
const verifications = ref<APVerification[]>([])
const reconciliations = ref<APReconciliation[]>([])
const suppliers = ref<Supplier[]>([])
const unverifiedInvoices = ref<APInvoice[]>([])
const unverifiedPayments = ref<APPayment[]>([])

const invoiceLoading = ref(false)
const paymentLoading = ref(false)
const verificationLoading = ref(false)
const reconciliationLoading = ref(false)

const invoiceQuery = reactive({
  supplier_name: '',
  invoice_no: '',
  status: '',
})

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

const getAPInvoiceStatusText = (status: string) => {
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

const getPaymentMethodLabel = (method: string) => {
  const map: Record<string, string> = {
    bank_transfer: '银行转账',
    cash: '现金',
    check: '支票',
    acceptance_bill: '承兑汇票',
  }
  return map[method] || method
}

const getReconciliationStatusLabel = (status: string) => {
  const map: Record<string, string> = { pending: '待确认', confirmed: '已确认', disputed: '有异议' }
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

const fetchInvoices = async () => {
  invoiceLoading.value = true
  try {
    const res = await listAPInvoices(invoiceQuery)
    const d = res.data as any
    invoices.value = d?.items || d?.data || d || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取发票列表失败')
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

const fetchPayments = async () => {
  paymentLoading.value = true
  try {
    const res = await listAPPayments()
    const d = res.data as any
    payments.value = d?.items || d?.data || d || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取付款列表失败')
  } finally {
    paymentLoading.value = false
  }
}

const fetchVerifications = async () => {
  verificationLoading.value = true
  try {
    const res = await listAPVerifications()
    const d = res.data as any
    verifications.value = d?.items || d?.data || d || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取核销列表失败')
  } finally {
    verificationLoading.value = false
  }
}

const fetchReconciliations = async () => {
  reconciliationLoading.value = true
  try {
    const res = await listAPReconciliations()
    const d = res.data as any
    reconciliations.value = d?.items || d?.data || d || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取对账列表失败')
  } finally {
    reconciliationLoading.value = false
  }
}

const fetchSuppliers = async () => {
  try {
    const res = await listSuppliers()
    const d = res.data as any
    suppliers.value = d?.list || d?.data || []
  } catch (error: any) {
    console.error('获取供应商列表失败:', error)
  }
}

const invoiceDialogVisible = ref(false)
const invoiceFormRef = ref<FormInstance>()
const invoiceSubmitLoading = ref(false)
const invoiceForm = reactive({
  supplier_id: undefined as number | undefined,
  invoice_no: '',
  invoice_date: '',
  invoice_amount: 0,
  tax_amount: 0,
  due_date: '',
  remark: '',
})

const invoiceRules: FormRules = {
  supplier_id: [{ required: true, message: '请选择供应商', trigger: 'change' }],
  invoice_no: [{ required: true, message: '请输入发票号', trigger: 'blur' }],
  invoice_date: [{ required: true, message: '请选择发票日期', trigger: 'change' }],
  invoice_amount: [{ required: true, message: '请输入发票金额', trigger: 'blur' }],
}

const openInvoiceDialog = () => {
  invoiceFormRef.value?.resetFields()
  invoiceForm.supplier_id = undefined
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
    await createAPInvoice(invoiceForm)
    ElMessage.success('创建成功')
    invoiceDialogVisible.value = false
    fetchInvoices()
  } catch (error: any) {
    ElMessage.error(error.message || '操作失败')
  } finally {
    invoiceSubmitLoading.value = false
  }
}

const viewInvoice = (row: APInvoice) => {
  ElMessage.info(`查看发票: ${row.invoice_no}`)
}

const approveInvoice = async (row: APInvoice) => {
  try {
    await ElMessageBox.confirm('确定审核该发票吗？', '审核确认', { type: 'info' })
    await approveAPInvoice(row.id)
    ElMessage.success('审核成功')
    fetchInvoices()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '操作失败')
    }
  }
}

const cancelInvoice = async (row: APInvoice) => {
  try {
    await ElMessageBox.confirm('确定取消该发票吗？', '取消确认', { type: 'warning' })
    await cancelAPInvoice(row.id)
    ElMessage.success('取消成功')
    fetchInvoices()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '操作失败')
    }
  }
}

const paymentDialogVisible = ref(false)
const paymentFormRef = ref<FormInstance>()
const paymentSubmitLoading = ref(false)
const paymentForm = reactive({
  supplier_id: undefined as number | undefined,
  payment_date: '',
  payment_amount: 0,
  payment_method: 'bank_transfer',
  bank_account: '',
  remark: '',
})

const paymentRules: FormRules = {
  supplier_id: [{ required: true, message: '请选择供应商', trigger: 'change' }],
  payment_date: [{ required: true, message: '请选择付款日期', trigger: 'change' }],
  payment_amount: [{ required: true, message: '请输入付款金额', trigger: 'blur' }],
  payment_method: [{ required: true, message: '请选择付款方式', trigger: 'change' }],
}

const openPaymentDialog = () => {
  paymentFormRef.value?.resetFields()
  paymentForm.supplier_id = undefined
  paymentForm.payment_date = new Date().toISOString().split('T')[0]
  paymentForm.payment_amount = 0
  paymentForm.payment_method = 'bank_transfer'
  paymentForm.bank_account = ''
  paymentForm.remark = ''
  paymentDialogVisible.value = true
}

const submitPayment = async () => {
  const valid = await paymentFormRef.value?.validate()
  if (!valid) return

  paymentSubmitLoading.value = true
  try {
    await createAPPayment(paymentForm)
    ElMessage.success('创建成功')
    paymentDialogVisible.value = false
    fetchPayments()
  } catch (error: any) {
    ElMessage.error(error.message || '操作失败')
  } finally {
    paymentSubmitLoading.value = false
  }
}

const confirmPayment = async (row: APPayment) => {
  try {
    await ElMessageBox.confirm('确定确认该付款吗？', '确认付款', { type: 'info' })
    await confirmAPPayment(row.id)
    ElMessage.success('确认成功')
    fetchPayments()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '操作失败')
    }
  }
}

const verificationDialogVisible = ref(false)
const verificationSubmitLoading = ref(false)
const verificationForm = reactive({
  invoice_id: undefined as number | undefined,
  payment_id: undefined as number | undefined,
  amount: 0,
})

const openVerificationDialog = async () => {
  try {
    const [invRes, payRes] = await Promise.all([
      getUnverifiedAPInvoices(),
      getUnverifiedAPPayments(),
    ])
    const d1 = invRes.data as any
    const d2 = payRes.data as any
    unverifiedInvoices.value = (Array.isArray(d1) ? d1 : d1?.items || d1?.data || []).filter(
      (i: APInvoice) => i.unverified_amount > 0
    )
    unverifiedPayments.value = Array.isArray(d2) ? d2 : d2?.items || d2?.data || []
  } catch (error) {
    console.error(error)
  }
  verificationForm.invoice_id = undefined
  verificationForm.payment_id = undefined
  verificationForm.amount = 0
  verificationDialogVisible.value = true
}

const submitVerification = async () => {
  if (
    !verificationForm.invoice_id ||
    !verificationForm.payment_id ||
    verificationForm.amount <= 0
  ) {
    ElMessage.warning('请完整填写核销信息')
    return
  }

  verificationSubmitLoading.value = true
  try {
    await manualVerifyAP({
      invoice_id: verificationForm.invoice_id,
      payment_id: verificationForm.payment_id,
      amount: verificationForm.amount,
    })
    ElMessage.success('核销成功')
    verificationDialogVisible.value = false
    fetchVerifications()
    fetchInvoices()
  } catch (error: any) {
    ElMessage.error(error.message || '操作失败')
  } finally {
    verificationSubmitLoading.value = false
  }
}

const generateReconciliation = async () => {
  reconciliationForm.value = {
    supplier_id: undefined,
    start_date: '',
    end_date: '',
  }
  reconciliationDialogVisible.value = true
}

const reconciliationDialogVisible = ref(false)
const reconciliationForm = ref({
  supplier_id: undefined as number | undefined,
  start_date: '',
  end_date: '',
})

const submitReconciliation = async () => {
  if (
    !reconciliationForm.value.supplier_id ||
    !reconciliationForm.value.start_date ||
    !reconciliationForm.value.end_date
  ) {
    ElMessage.warning('请填写完整信息')
    return
  }
  try {
    await generateAPReconciliation({
      supplier_id: reconciliationForm.value.supplier_id,
      start_date: reconciliationForm.value.start_date,
      end_date: reconciliationForm.value.end_date,
    })
    ElMessage.success('对账单生成成功')
    reconciliationDialogVisible.value = false
    fetchReconciliations()
  } catch (error: any) {
    ElMessage.error(error.message || '生成失败')
  }
}

const confirmReconciliation = async (row: APReconciliation) => {
  try {
    await ElMessageBox.confirm('确定确认该对账单吗？', '确认对账', { type: 'info' })
    await confirmAPReconciliation(row.id)
    ElMessage.success('确认成功')
    fetchReconciliations()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '操作失败')
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
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '操作失败')
    }
  }
}

const handlePrintInvoices = () => {
  const printData = invoices.value.map((item: any, index: number) => ({
    序号: index + 1,
    发票号: item.invoice_no,
    供应商: item.supplier_name,
    发票金额: `¥${item.invoice_amount}`,
    税额: `¥${item.tax_amount}`,
    状态: getAPInvoiceStatusText(item.status),
    发票日期: item.invoice_date,
  }))
  printJS({
    printable: printData,
    properties: Object.keys(printData[0] || {}),
    type: 'table' as any,
    header: '应付发票列表',
    style: 'padding: 20px; font-size: 14px;',
    headerStyle: 'font-size: 18px; font-weight: bold; margin-bottom: 20px;',
    gridHeaderStyle: 'font-weight: bold; background-color: #f5f7fa;',
    gridStyle: 'border-collapse: collapse; width: 100%;',
  })
}

const handleExportInvoices = () => {
  const csvContent = [
    ['发票号', '供应商', '发票金额', '税额', '状态', '发票日期'],
    ...invoices.value.map((item: any) => [
      item.invoice_no,
      item.supplier_name,
      item.invoice_amount,
      item.tax_amount,
      getAPInvoiceStatusText(item.status),
      item.invoice_date,
    ]),
  ]
    .map((row: any[]) => row.map((cell: any) => `"${cell}"`).join(','))
    .join('\n')
  const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' })
  const link = document.createElement('a')
  link.href = URL.createObjectURL(blob)
  link.download = `应付发票_${new Date().toISOString().split('T')[0]}.csv`
  link.click()
  ElMessage.success('导出成功')
}

onMounted(() => {
  fetchSuppliers()
  fetchInvoices()
  fetchPayments()
  fetchVerifications()
  fetchReconciliations()
})
</script>

<style scoped>
.ap-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
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
.filter-card {
  margin-bottom: 20px;
}
.text-red {
  color: #f56c6c;
}
</style>
