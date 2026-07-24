<!--
  VerificationTab.vue - 核销管理 Tab
  来源：原 ap/index.vue 中 核销管理 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="verification-tab">
    <div class="page-header">
      <h2 class="page-title">{{ $t('apModule.verification.title') }}</h2>
      <el-button type="primary" @click="openVerificationDialog()">
        <el-icon><Plus /></el-icon> {{ $t('apModule.verification.create') }}
      </el-button>
    </div>

    <el-card shadow="hover">
      <el-table v-loading="verificationLoading" :data="verifications" stripe :aria-label="$t('apModule.verification.listAria')">
        <el-table-column prop="verification_no" :label="$t('apModule.verification.verificationNo')" width="140" />
        <el-table-column prop="invoice_no" :label="$t('apModule.verification.invoiceNo')" width="140" />
        <el-table-column prop="payment_no" :label="$t('apModule.verification.paymentNo')" width="140" />
        <el-table-column prop="verification_date" :label="$t('apModule.verification.verificationDate')" width="120" />
        <el-table-column :label="$t('apModule.verification.verificationAmount')" width="120" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.verification_amount) }}
          </template>
        </el-table-column>
        <el-table-column prop="status" :label="$t('common.status')" width="90" align="center">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{ row.status === 'active' ? $t('apModule.verification.statusActive') : $t('apModule.verification.statusCancelled') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_at" :label="$t('common.createTime')" width="160" />
      </el-table>
    </el-card>

    <el-dialog v-model="verificationDialogVisible" :title="$t('apModule.verification.createTitle')" width="600px" :aria-label="$t('apModule.verification.createAria')">
      <el-form :model="verificationForm" label-width="100px" :aria-label="$t('apModule.verification.formAria')">
        <el-form-item :label="$t('apModule.verification.invoiceNo')">
          <el-select
            v-model="verificationForm.invoice_id"
            :placeholder="$t('apModule.verification.invoicePlaceholder')"
            style="width: 100%"
          >
            <el-option
              v-for="inv in unverifiedInvoices"
              :key="inv.id"
              :label="$t('apModule.verification.invoiceOption', { no: inv.invoice_no, amount: formatMoney(inv.unverified_amount) })"
              :value="inv.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('apModule.verification.paymentNo')">
          <el-select
            v-model="verificationForm.payment_id"
            :placeholder="$t('apModule.verification.paymentPlaceholder')"
            style="width: 100%"
          >
            <el-option
              v-for="pay in unverifiedPayments"
              :key="pay.id"
              :label="$t('apModule.verification.paymentOption', { no: pay.payment_no, amount: formatMoney(pay.payment_amount) })"
              :value="pay.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('apModule.verification.verificationAmount')">
          <el-input-number
            v-model="verificationForm.amount"
            :min="0"
            :precision="2"
            style="width: 100%"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="verificationDialogVisible = false">{{ $t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="verificationSubmitLoading" @click="submitVerification"
          >{{ $t('common.confirm') }}</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  getAPVerificationList,
  manualVerifyAP,
  getUnverifiedAPInvoices,
  getUnverifiedAPPayments,
  type APVerification,
} from '@/api/ap-verification'
import type { APInvoice } from '@/api/ap-invoice'
import type { APPayment } from '@/api/ap-payment'

const { t } = useI18n({ useScope: 'global' })

const verifications = ref<APVerification[]>([])
const verificationLoading = ref(false)
const unverifiedInvoices = ref<APInvoice[]>([])
const unverifiedPayments = ref<APPayment[]>([])

const formatMoney = (amount: number | undefined) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const fetchVerifications = async () => {
  verificationLoading.value = true
  try {
    const res = await getAPVerificationList()
    const d = res.data as
      | { list?: APVerification[]; items?: APVerification[] }
      | APVerification[]
      | undefined
    if (d && typeof d === 'object' && !Array.isArray(d)) {
      verifications.value = d.list || d.items || []
    } else {
      verifications.value = (d as APVerification[]) || []
    }
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || t('apModule.verification.fetchListFailed'))
  } finally {
    verificationLoading.value = false
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
    const d1 = invRes.data as
      | { list?: APInvoice[]; items?: APInvoice[]; data?: APInvoice[] }
      | APInvoice[]
      | undefined
    const d2 = payRes.data as
      | { list?: APPayment[]; items?: APPayment[]; data?: APPayment[] }
      | APPayment[]
      | undefined
    const invs: APInvoice[] =
      d1 && typeof d1 === 'object' && !Array.isArray(d1)
        ? d1.list || d1.items || d1.data || []
        : (d1 as APInvoice[]) || []
    const pays: APPayment[] =
      d2 && typeof d2 === 'object' && !Array.isArray(d2)
        ? d2.list || d2.items || d2.data || []
        : (d2 as APPayment[]) || []
    unverifiedInvoices.value = invs.filter(i => i.unverified_amount > 0)
    unverifiedPayments.value = pays
  } catch (e) {
    void e
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
    ElMessage.warning(t('apModule.verification.pleaseFillComplete'))
    return
  }
  verificationSubmitLoading.value = true
  try {
    await manualVerifyAP({
      invoice_id: verificationForm.invoice_id,
      payment_id: verificationForm.payment_id,
      amount: verificationForm.amount,
    })
    ElMessage.success(t('apModule.verification.verifySuccess'))
    verificationDialogVisible.value = false
    fetchVerifications()
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || t('common.failed'))
  } finally {
    verificationSubmitLoading.value = false
  }
}

defineExpose({ refresh: fetchVerifications })

onMounted(() => {
  fetchVerifications()
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
