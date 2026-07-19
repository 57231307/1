<!--
  VerificationTab.vue - 核销管理 Tab
  来源：原 ap/index.vue 中 核销管理 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="verification-tab">
    <div class="page-header">
      <h2 class="page-title">核销管理</h2>
      <el-button type="primary" @click="openVerificationDialog()">
        <el-icon><Plus /></el-icon> 新建核销
      </el-button>
    </div>

    <el-card shadow="hover">
      <el-table v-loading="verificationLoading" :data="verifications" stripe aria-label="核销列表">
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

    <el-dialog v-model="verificationDialogVisible" title="新建核销" width="600px" aria-label="新建核销对话框">
      <el-form :model="verificationForm" label-width="100px" aria-label="核销表单">
        <el-form-item label="发票号">
          <el-select
            v-model="verificationForm.invoice_id"
            placeholder="选择发票"
            style="width: 100%"
          >
            <el-option
              v-for="inv in unverifiedInvoices"
              :key="inv.id"
              :label="`${inv.invoice_no} (未核销: ${formatMoney(inv.unverified_amount)})`"
              :value="inv.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="付款单号">
          <el-select
            v-model="verificationForm.payment_id"
            placeholder="选择付款单"
            style="width: 100%"
          >
            <el-option
              v-for="pay in unverifiedPayments"
              :key="pay.id"
              :label="`${pay.payment_no} (金额: ${formatMoney(pay.payment_amount)})`"
              :value="pay.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="核销金额">
          <el-input-number
            v-model="verificationForm.amount"
            :min="0"
            :precision="2"
            style="width: 100%"
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
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  listAPVerifications,
  manualVerifyAP,
  getUnverifiedAPInvoices,
  getUnverifiedAPPayments,
  type APVerification,
} from '@/api/ap-verification'
import type { APInvoice } from '@/api/ap-invoice'
import type { APPayment } from '@/api/ap-payment'

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
    const res = await listAPVerifications()
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
    ElMessage.error(err.message || '获取核销列表失败')
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
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || '操作失败')
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
