<template>
  <div class="ar-page">
    <el-tabs v-model="activeTab">
      <el-tab-pane label="应收发票" name="invoice">
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
          <el-table :data="invoices" v-loading="invoiceLoading" stripe>
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
                <el-button v-if="row.status === 'pending'" type="success" link size="small" @click="approveInvoice(row)">审核</el-button>
                <el-button v-if="row.status === 'pending'" type="danger" link size="small" @click="cancelInvoice(row)">取消</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="应收对账" name="reconciliation">
        <div class="page-header">
          <h2 class="page-title">应收对账</h2>
          <el-button type="primary" @click="openReconciliationDialog()">
            <el-icon><Plus /></el-icon>
            新建对账
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table :data="reconciliations" v-loading="reconciliationLoading" stripe>
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
                <el-button v-if="row.status === 'pending'" type="success" link size="small" @click="confirmReconciliation(row)">确认</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="资金账户" name="fund">
        <div class="page-header">
          <h2 class="page-title">资金账户</h2>
          <el-button type="primary" @click="openFundDialog()">
            <el-icon><Plus /></el-icon>
            新建账户
          </el-button>
        </div>

        <el-row :gutter="20" class="fund-summary">
          <el-col :span="6">
            <el-card shadow="hover">
              <div class="summary-item">
                <div class="summary-label">总余额</div>
                <div class="summary-value">{{ formatMoney(totalBalance) }}</div>
              </div>
            </el-card>
          </el-col>
          <el-col :span="6">
            <el-card shadow="hover">
              <div class="summary-item">
                <div class="summary-label">冻结金额</div>
                <div class="summary-value text-orange">{{ formatMoney(totalFrozen) }}</div>
              </div>
            </el-card>
          </el-col>
          <el-col :span="6">
            <el-card shadow="hover">
              <div class="summary-item">
                <div class="summary-label">可用余额</div>
                <div class="summary-value text-green">{{ formatMoney(totalAvailable) }}</div>
              </div>
            </el-card>
          </el-col>
          <el-col :span="6">
            <el-card shadow="hover">
              <div class="summary-item">
                <div class="summary-label">账户数量</div>
                <div class="summary-value">{{ funds.length }}</div>
              </div>
            </el-card>
          </el-col>
        </el-row>

        <el-card shadow="hover" class="mt-20">
          <el-table :data="funds" v-loading="fundLoading" stripe>
            <el-table-column prop="account_code" label="账户编码" width="120" />
            <el-table-column prop="account_name" label="账户名称" min-width="150" />
            <el-table-column prop="account_type" label="账户类型" width="100">
              <template #default="{ row }">
                {{ getAccountTypeLabel(row.account_type) }}
              </template>
            </el-table-column>
            <el-table-column label="余额" width="140" align="right">
              <template #default="{ row }">
                {{ formatMoney(row.balance) }}
              </template>
            </el-table-column>
            <el-table-column label="冻结金额" width="120" align="right">
              <template #default="{ row }">
                <span class="text-orange">{{ formatMoney(row.frozen_balance) }}</span>
              </template>
            </el-table-column>
            <el-table-column label="可用余额" width="140" align="right">
              <template #default="{ row }">
                <span class="text-green">{{ formatMoney(row.available_balance) }}</span>
              </template>
            </el-table-column>
            <el-table-column prop="bank_name" label="开户银行" width="150" />
            <el-table-column prop="bank_account" label="银行账号" width="180" />
            <el-table-column prop="status" label="状态" width="80" align="center">
              <template #default="{ row }">
                <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
                  {{ row.status === 'active' ? '正常' : '冻结' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="200" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="depositFund(row)">存入</el-button>
                <el-button type="primary" link size="small" @click="withdrawFund(row)">取出</el-button>
                <el-button type="warning" link size="small" @click="freezeFund(row)">冻结</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>
    </el-tabs>

    <el-dialog v-model="invoiceDialogVisible" title="新建应收发票" width="600px">
      <el-form ref="invoiceFormRef" :model="invoiceForm" :rules="invoiceRules" label-width="80px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="客户" prop="customer_id">
              <el-select v-model="invoiceForm.customer_id" placeholder="选择客户" style="width: 100%">
                <el-option v-for="c in customers" :key="c.id" :label="c.customer_name" :value="c.id" />
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
              <el-date-picker v-model="invoiceForm.invoice_date" type="date" placeholder="选择日期" value-format="YYYY-MM-DD" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="到期日">
              <el-date-picker v-model="invoiceForm.due_date" type="date" placeholder="选择日期" value-format="YYYY-MM-DD" style="width: 100%" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="发票金额" prop="invoice_amount">
              <el-input-number v-model="invoiceForm.invoice_amount" :min="0" :precision="2" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="税额">
              <el-input-number v-model="invoiceForm.tax_amount" :min="0" :precision="2" style="width: 100%" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="备注">
          <el-input v-model="invoiceForm.remark" type="textarea" placeholder="请输入备注" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="invoiceDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="invoiceSubmitLoading" @click="submitInvoice">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="reconciliationDialogVisible" title="新建对账" width="500px">
      <el-form ref="reconciliationFormRef" :model="reconciliationForm" label-width="80px">
        <el-form-item label="客户">
          <el-select v-model="reconciliationForm.customer_id" placeholder="选择客户" style="width: 100%">
            <el-option v-for="c in customers" :key="c.id" :label="c.customer_name" :value="c.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="对账日期">
          <el-date-picker v-model="reconciliationForm.reconciliation_date" type="date" placeholder="选择日期" value-format="YYYY-MM-DD" style="width: 100%" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="reconciliationDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="reconciliationSubmitLoading" @click="submitReconciliation">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="fundDialogVisible" title="新建资金账户" width="500px">
      <el-form ref="fundFormRef" :model="fundForm" :rules="fundRules" label-width="80px">
        <el-form-item label="账户编码" prop="account_code">
          <el-input v-model="fundForm.account_code" placeholder="请输入账户编码" />
        </el-form-item>
        <el-form-item label="账户名称" prop="account_name">
          <el-input v-model="fundForm.account_name" placeholder="请输入账户名称" />
        </el-form-item>
        <el-form-item label="账户类型" prop="account_type">
          <el-select v-model="fundForm.account_type" placeholder="选择类型" style="width: 100%">
            <el-option label="银行账户" value="bank" />
            <el-option label="现金账户" value="cash" />
            <el-option label="支付宝" value="alipay" />
            <el-option label="微信" value="wechat" />
          </el-select>
        </el-form-item>
        <el-form-item label="开户银行">
          <el-input v-model="fundForm.bank_name" placeholder="请输入开户银行" />
        </el-form-item>
        <el-form-item label="银行账号">
          <el-input v-model="fundForm.bank_account" placeholder="请输入银行账号" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="fundDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="fundSubmitLoading" @click="submitFund">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="fundOperationDialogVisible" :title="fundOperationTitle" width="400px">
      <el-form label-width="80px">
        <el-form-item label="金额">
          <el-input-number v-model="fundOperationAmount" :min="0" :precision="2" style="width: 100%" />
        </el-form-item>
        <el-form-item v-if="fundOperationType === 'freeze'" label="原因">
          <el-input v-model="fundOperationReason" type="textarea" placeholder="请输入冻结原因" />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="fundOperationRemark" placeholder="请输入备注" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="fundOperationDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="fundOperationLoading" @click="submitFundOperation">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Printer, Download } from '@element-plus/icons-vue'
import printJS from 'print-js'
import type { FormInstance, FormRules } from 'element-plus'
import { listARInvoices, createARInvoice, approveARInvoice, cancelARInvoice, type ARInvoice } from '@/api/ar'
import { listARReconciliations, createARReconciliation, updateARReconciliationStatus, type ARReconciliation } from '@/api/ar'
import { listFundAccounts, createFundAccount, depositFund as depositFundApi, withdrawFund as withdrawFundApi, freezeFund as freezeFundApi, type FundAccount } from '@/api/ar'
import { listCustomers, type Customer } from '@/api/customer'

const activeTab = ref('invoice')

const invoices = ref<ARInvoice[]>([])
const reconciliations = ref<ARReconciliation[]>([])
const funds = ref<FundAccount[]>([])
const customers = ref<Customer[]>([])

const invoiceLoading = ref(false)
const reconciliationLoading = ref(false)
const fundLoading = ref(false)

const invoiceQuery = reactive({
  customer_name: '',
  invoice_no: '',
  status: ''
})

const formatMoney = (amount: number) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const getInvoiceStatusLabel = (status: string) => {
  const map: Record<string, string> = { pending: '待审核', approved: '已审核', verified: '已核销', cancelled: '已取消' }
  return map[status] || status
}

const getARInvoiceStatusText = (status: string) => {
  const map: Record<string, string> = { pending: '待审核', approved: '已审核', verified: '已核销', cancelled: '已取消' }
  return map[status] || status
}

const getInvoiceStatusType = (status: string) => {
  const map: Record<string, string> = { pending: 'warning', approved: 'success', verified: 'primary', cancelled: 'info' }
  return map[status] || 'info'
}

const getReconciliationStatusLabel = (status: string) => {
  const map: Record<string, string> = { pending: '待确认', confirmed: '已确认', disputed: '有异议' }
  return map[status] || status
}

const getReconciliationStatusType = (status: string) => {
  const map: Record<string, string> = { pending: 'warning', confirmed: 'success', disputed: 'danger' }
  return map[status] || 'info'
}

const getAccountTypeLabel = (type: string) => {
  const map: Record<string, string> = { bank: '银行账户', cash: '现金账户', alipay: '支付宝', wechat: '微信' }
  return map[type] || type
}

const totalBalance = computed(() => funds.value.reduce((sum, f) => sum + (f.balance || 0), 0))
const totalFrozen = computed(() => funds.value.reduce((sum, f) => sum + (f.frozen_balance || 0), 0))
const totalAvailable = computed(() => funds.value.reduce((sum, f) => sum + (f.available_balance || 0), 0))

const fetchInvoices = async () => {
  invoiceLoading.value = true
  try {
    const res = await listARInvoices(invoiceQuery)
    invoices.value = res.data! || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取发票列表失败')
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

const fetchReconciliations = async () => {
  reconciliationLoading.value = true
  try {
    const res = await listARReconciliations()
    reconciliations.value = res.data! || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取对账列表失败')
  } finally {
    reconciliationLoading.value = false
  }
}

const fetchFunds = async () => {
  fundLoading.value = true
  try {
    const res = await listFundAccounts()
    funds.value = res.data! || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取资金账户列表失败')
  } finally {
    fundLoading.value = false
  }
}

const fetchCustomers = async () => {
  try {
    const res = await listCustomers()
    const d = res.data as any
    customers.value = d?.list || d?.data || []
  } catch (error: any) {
    console.error('获取客户列表失败:', error)
  }
}

const invoiceDialogVisible = ref(false)
const invoiceFormRef = ref<FormInstance>()
const invoiceSubmitLoading = ref(false)
const invoiceForm = reactive({
  customer_id: undefined as number | undefined,
  invoice_no: '',
  invoice_date: '',
  invoice_amount: 0,
  tax_amount: 0,
  due_date: '',
  remark: ''
})

const invoiceRules: FormRules = {
  customer_id: [{ required: true, message: '请选择客户', trigger: 'change' }],
  invoice_no: [{ required: true, message: '请输入发票号', trigger: 'blur' }],
  invoice_date: [{ required: true, message: '请选择发票日期', trigger: 'change' }],
  invoice_amount: [{ required: true, message: '请输入发票金额', trigger: 'blur' }]
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
  } catch (error: any) {
    ElMessage.error(error.message || '操作失败')
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
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '操作失败')
    }
  }
}

const cancelInvoice = async (row: ARInvoice) => {
  try {
    await ElMessageBox.confirm('确定取消该发票吗？', '取消确认', { type: 'warning' })
    await cancelARInvoice(row.id)
    ElMessage.success('取消成功')
    fetchInvoices()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '操作失败')
    }
  }
}

const reconciliationDialogVisible = ref(false)
const reconciliationFormRef = ref<FormInstance>()
const reconciliationSubmitLoading = ref(false)
const reconciliationForm = reactive({
  customer_id: undefined as number | undefined,
  reconciliation_date: ''
})

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
  } catch (error: any) {
    ElMessage.error(error.message || '操作失败')
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
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '操作失败')
    }
  }
}

const fundDialogVisible = ref(false)
const fundFormRef = ref<FormInstance>()
const fundSubmitLoading = ref(false)
const fundForm = reactive({
  account_code: '',
  account_name: '',
  account_type: 'bank',
  bank_name: '',
  bank_account: ''
})

const fundRules: FormRules = {
  account_code: [{ required: true, message: '请输入账户编码', trigger: 'blur' }],
  account_name: [{ required: true, message: '请输入账户名称', trigger: 'blur' }],
  account_type: [{ required: true, message: '请选择账户类型', trigger: 'change' }]
}

const openFundDialog = () => {
  fundFormRef.value?.resetFields()
  fundForm.account_code = ''
  fundForm.account_name = ''
  fundForm.account_type = 'bank'
  fundForm.bank_name = ''
  fundForm.bank_account = ''
  fundDialogVisible.value = true
}

const submitFund = async () => {
  const valid = await fundFormRef.value?.validate()
  if (!valid) return
  
  fundSubmitLoading.value = true
  try {
    await createFundAccount(fundForm)
    ElMessage.success('创建成功')
    fundDialogVisible.value = false
    fetchFunds()
  } catch (error: any) {
    ElMessage.error(error.message || '操作失败')
  } finally {
    fundSubmitLoading.value = false
  }
}

const fundOperationDialogVisible = ref(false)
const fundOperationType = ref('')
const fundOperationTitle = ref('')
const fundOperationAmount = ref(0)
const fundOperationReason = ref('')
const fundOperationRemark = ref('')
const fundOperationLoading = ref(false)
const currentFundAccount = ref<FundAccount | null>(null)

const depositFund = (row: FundAccount) => {
  currentFundAccount.value = row
  fundOperationType.value = 'deposit'
  fundOperationTitle.value = '存入资金'
  fundOperationAmount.value = 0
  fundOperationRemark.value = ''
  fundOperationDialogVisible.value = true
}

const withdrawFund = (row: FundAccount) => {
  currentFundAccount.value = row
  fundOperationType.value = 'withdraw'
  fundOperationTitle.value = '取出资金'
  fundOperationAmount.value = 0
  fundOperationRemark.value = ''
  fundOperationDialogVisible.value = true
}

const freezeFund = (row: FundAccount) => {
  currentFundAccount.value = row
  fundOperationType.value = 'freeze'
  fundOperationTitle.value = '冻结资金'
  fundOperationAmount.value = 0
  fundOperationReason.value = ''
  fundOperationRemark.value = ''
  fundOperationDialogVisible.value = true
}

const submitFundOperation = async () => {
  if (fundOperationAmount.value <= 0) {
    ElMessage.warning('请输入有效金额')
    return
  }
  
  if (!currentFundAccount.value) return
  
  fundOperationLoading.value = true
  try {
    if (fundOperationType.value === 'deposit') {
      await depositFundApi(currentFundAccount.value.id, { amount: fundOperationAmount.value, remark: fundOperationRemark.value })
      ElMessage.success('存入成功')
    } else if (fundOperationType.value === 'withdraw') {
      await withdrawFundApi(currentFundAccount.value.id, { amount: fundOperationAmount.value, remark: fundOperationRemark.value })
      ElMessage.success('取出成功')
    } else if (fundOperationType.value === 'freeze') {
      if (!fundOperationReason.value) {
        ElMessage.warning('请输入冻结原因')
        return
      }
      await freezeFundApi(currentFundAccount.value.id, { amount: fundOperationAmount.value, reason: fundOperationReason.value })
      ElMessage.success('冻结成功')
    }
    fundOperationDialogVisible.value = false
    fetchFunds()
  } catch (error: any) {
    ElMessage.error(error.message || '操作失败')
  } finally {
    fundOperationLoading.value = false
  }
}

const handlePrintInvoices = () => {
  const printData = invoices.value.map((item: any, index: number) => ({
    '序号': index + 1,
    '发票号': item.invoice_no,
    '客户': item.customer_name,
    '发票金额': `¥${item.invoice_amount}`,
    '税额': `¥${item.tax_amount}`,
    '状态': getARInvoiceStatusText(item.status),
    '发票日期': item.invoice_date
  }))
  printJS({
    printable: printData,
    properties: Object.keys(printData[0] || {}),
    type: 'table' as any,
    header: '应收发票列表',
    style: 'padding: 20px; font-size: 14px;',
    headerStyle: 'font-size: 18px; font-weight: bold; margin-bottom: 20px;',
    gridHeaderStyle: 'font-weight: bold; background-color: #f5f7fa;',
    gridStyle: 'border-collapse: collapse; width: 100%;'
  })
}

const handleExportInvoices = () => {
  const csvContent = [
    ['发票号', '客户', '发票金额', '税额', '状态', '发票日期'],
    ...invoices.value.map((item: any) => [item.invoice_no, item.customer_name, item.invoice_amount, item.tax_amount, getARInvoiceStatusText(item.status), item.invoice_date])
  ].map((row: any[]) => row.map((cell: any) => `"${cell}"`).join(',')).join('\n')
  const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' })
  const link = document.createElement('a')
  link.href = URL.createObjectURL(blob)
  link.download = `应收发票_${new Date().toISOString().split('T')[0]}.csv`
  link.click()
  ElMessage.success('导出成功')
}

onMounted(() => {
  fetchCustomers()
  fetchInvoices()
  fetchReconciliations()
  fetchFunds()
})
</script>

<style scoped>
.ar-page { padding: 24px; background-color: #f5f7fa; min-height: 100%; }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; }
.page-title { font-size: 20px; font-weight: 600; color: #303133; margin: 0; }
.filter-card { margin-bottom: 20px; }
.text-red { color: #f56c6c; }
.text-orange { color: #e6a23c; }
.text-green { color: #67c23a; }
.mt-20 { margin-top: 20px; }
.fund-summary .summary-item { text-align: center; padding: 10px 0; }
.fund-summary .summary-label { font-size: 14px; color: #909399; margin-bottom: 8px; }
.fund-summary .summary-value { font-size: 24px; font-weight: 600; color: #303133; }
</style>
