<!--
  PaymentTab.vue - 付款管理 Tab
  来源：原 ap/index.vue 中 付款管理 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="payment-tab">
    <div class="page-header">
      <h2 class="page-title">付款管理</h2>
      <el-button type="primary" @click="openPaymentDialog()">
        <el-icon><Plus /></el-icon> 新建付款
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
              @click="confirmPayment(row as unknown as APPayment)"
              >确认</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="paymentDialogVisible" title="新建付款" width="600px">
      <el-form ref="paymentFormRef" :model="paymentForm" :rules="paymentRules" label-width="100px">
        <el-form-item label="供应商" prop="supplier_id">
          <el-select v-model="paymentForm.supplier_id" placeholder="选择供应商" style="width: 100%">
            <el-option v-for="s in suppliers" :key="s.id" :label="s.supplier_name" :value="s.id" />
          </el-select>
        </el-form-item>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="付款日期" prop="payment_date">
              <el-date-picker
                v-model="paymentForm.payment_date"
                type="date"
                value-format="YYYY-MM-DD"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
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
        </el-row>
        <el-form-item label="付款方式" prop="payment_method">
          <el-select v-model="paymentForm.payment_method" style="width: 100%">
            <el-option label="银行转账" value="bank_transfer" />
            <el-option label="现金" value="cash" />
            <el-option label="支票" value="check" />
            <el-option label="承兑汇票" value="bill" />
          </el-select>
        </el-form-item>
        <el-form-item label="银行账户">
          <el-input v-model="paymentForm.bank_account" />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="paymentForm.remark" type="textarea" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="paymentDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="paymentSubmitLoading" @click="submitPayment"
          >确定</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import type { FormInstance, FormRules } from 'element-plus'
import {
  listAPPayments,
  createAPPayment,
  confirmAPPayment,
  getAPPaymentMethodText,
  type APPayment,
} from '@/api/ap-payment'
import type { Supplier } from '@/api/supplier'

const payments = ref<APPayment[]>([])
const paymentLoading = ref(false)
const suppliers = ref<Supplier[]>([])

const formatMoney = (amount: number | undefined) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const getPaymentMethodLabel = (method: string) => {
  return getAPPaymentMethodText(method) || method
}

const fetchPayments = async () => {
  paymentLoading.value = true
  try {
    const res = await listAPPayments()
    const d = res.data as { list?: APPayment[]; items?: APPayment[] } | APPayment[] | undefined
    if (d && typeof d === 'object' && !Array.isArray(d)) {
      payments.value = d.list || d.items || []
    } else {
      payments.value = (d as APPayment[]) || []
    }
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || '获取付款列表失败')
  } finally {
    paymentLoading.value = false
  }
}

const paymentDialogVisible = ref(false)
const paymentFormRef = ref<FormInstance>()
const paymentSubmitLoading = ref(false)
const paymentForm = reactive({
  supplier_id: undefined as number | undefined,
  payment_date: new Date().toISOString().split('T')[0],
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
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || '操作失败')
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
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || '操作失败')
    }
  }
}

defineExpose({ refresh: fetchPayments })

onMounted(() => {
  fetchPayments()
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
</style>
