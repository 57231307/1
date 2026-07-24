<!--
  PaymentTab.vue - 付款管理 Tab
  来源：原 ap/index.vue 中 付款管理 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="payment-tab">
    <div class="page-header">
      <h2 class="page-title">{{ $t('apModule.payment.title') }}</h2>
      <el-button type="primary" @click="openPaymentDialog()">
        <el-icon><Plus /></el-icon> {{ $t('apModule.payment.create') }}
      </el-button>
    </div>

    <el-card shadow="hover">
      <el-table v-loading="paymentLoading" :data="payments" stripe :aria-label="$t('apModule.payment.listAria')">
        <el-table-column prop="payment_no" :label="$t('apModule.payment.paymentNo')" width="140" />
        <el-table-column prop="supplier_name" :label="$t('apModule.payment.supplier')" width="150" />
        <el-table-column prop="payment_date" :label="$t('apModule.payment.paymentDate')" width="120" />
        <el-table-column :label="$t('apModule.payment.paymentAmount')" width="120" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.payment_amount) }}
          </template>
        </el-table-column>
        <el-table-column prop="payment_method" :label="$t('apModule.payment.paymentMethod')" width="100">
          <template #default="{ row }">
            {{ getPaymentMethodLabel(row.payment_method) }}
          </template>
        </el-table-column>
        <el-table-column prop="status" :label="$t('common.status')" width="90" align="center">
          <template #default="{ row }">
            <el-tag :type="row.status === 'confirmed' ? 'success' : 'warning'" size="small">
              {{ row.status === 'confirmed' ? $t('apModule.payment.statusConfirmed') : $t('apModule.payment.statusPending') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="bank_account" :label="$t('apModule.payment.bankAccount')" width="150" />
        <el-table-column prop="created_at" :label="$t('common.createTime')" width="160" />
        <el-table-column :label="$t('common.operation')" width="120" fixed="right">
          <template #default="{ row }">
            <el-button
              v-if="row.status !== 'confirmed'"
              type="success"
              link
              size="small"
              @click="confirmPayment(row as unknown as APPayment)"
              >{{ $t('apModule.payment.confirm') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="paymentDialogVisible" :title="$t('apModule.payment.createTitle')" width="600px" :aria-label="$t('apModule.payment.createAria')">
      <el-form ref="paymentFormRef" :model="paymentForm" :rules="paymentRules" label-width="100px" :aria-label="$t('apModule.payment.formAria')">
        <el-form-item :label="$t('apModule.payment.supplier')" prop="supplier_id">
          <el-select v-model="paymentForm.supplier_id" :placeholder="$t('apModule.payment.supplierPlaceholder')" style="width: 100%">
            <el-option v-for="s in suppliers" :key="s.id" :label="s.supplier_name" :value="s.id" />
          </el-select>
        </el-form-item>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="$t('apModule.payment.paymentDate')" prop="payment_date">
              <el-date-picker
                v-model="paymentForm.payment_date"
                type="date"
                value-format="YYYY-MM-DD"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="$t('apModule.payment.paymentAmount')" prop="payment_amount">
              <el-input-number
                v-model="paymentForm.payment_amount"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item :label="$t('apModule.payment.paymentMethod')" prop="payment_method">
          <el-select v-model="paymentForm.payment_method" style="width: 100%">
            <el-option :label="$t('apModule.payment.methodBankTransfer')" value="bank_transfer" />
            <el-option :label="$t('apModule.payment.methodCash')" value="cash" />
            <el-option :label="$t('apModule.payment.methodCheck')" value="check" />
            <el-option :label="$t('apModule.payment.methodBill')" value="bill" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('apModule.payment.bankAccount')">
          <el-input v-model="paymentForm.bank_account" />
        </el-form-item>
        <el-form-item :label="$t('apModule.payment.remark')">
          <el-input v-model="paymentForm.remark" type="textarea" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="paymentDialogVisible = false">{{ $t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="paymentSubmitLoading" @click="submitPayment"
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
import type { FormInstance, FormRules } from 'element-plus'
import {
  getAPPaymentList,
  createAPPayment,
  confirmAPPayment,
  getAPPaymentMethodText,
  type APPayment,
} from '@/api/ap-payment'
import type { Supplier } from '@/api/supplier'

const { t } = useI18n({ useScope: 'global' })

const payments = ref<APPayment[]>([])
const paymentLoading = ref(false)
const suppliers = ref<Supplier[]>([])

const formatMoney = (amount: number | undefined) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const getPaymentMethodLabel = (method: string) => {
  const keyMap: Record<string, string> = {
    bank_transfer: 'apModule.payment.methodBankTransfer',
    cash: 'apModule.payment.methodCash',
    check: 'apModule.payment.methodCheck',
    bill: 'apModule.payment.methodBill',
  }
  const key = keyMap[method]
  if (key) return t(key)
  return getAPPaymentMethodText(method) || method
}

const fetchPayments = async () => {
  paymentLoading.value = true
  try {
    const res = await getAPPaymentList()
    const d = res.data as { list?: APPayment[]; items?: APPayment[] } | APPayment[] | undefined
    if (d && typeof d === 'object' && !Array.isArray(d)) {
      payments.value = d.list || d.items || []
    } else {
      payments.value = (d as APPayment[]) || []
    }
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || t('apModule.payment.fetchListFailed'))
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
  supplier_id: [{ required: true, message: t('apModule.payment.supplierRequired'), trigger: 'change' }],
  payment_date: [{ required: true, message: t('apModule.payment.dateRequired'), trigger: 'change' }],
  payment_amount: [{ required: true, message: t('apModule.payment.amountRequired'), trigger: 'blur' }],
  payment_method: [{ required: true, message: t('apModule.payment.methodRequired'), trigger: 'change' }],
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
    ElMessage.success(t('common.success'))
    paymentDialogVisible.value = false
    fetchPayments()
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || t('common.failed'))
  } finally {
    paymentSubmitLoading.value = false
  }
}

const confirmPayment = async (row: APPayment) => {
  try {
    await ElMessageBox.confirm(t('apModule.payment.confirmConfirm'), t('apModule.payment.confirmTitle'), { type: 'info' })
    await confirmAPPayment(row.id)
    ElMessage.success(t('apModule.payment.confirmSuccess'))
    fetchPayments()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || t('common.failed'))
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
