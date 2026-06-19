<!--
  VoucherTab.vue - 凭证管理 Tab（拆分重构版）
  任务编号: P14 批 1 B3 I-2
  拆分：567 行 → ~95 行 + 4 子组件 + 2 composable + 1 工具
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="voucher-tab">
    <div class="page-header">
      <h2 class="page-title">凭证管理</h2>
      <div class="header-actions">
        <el-button type="primary" @click="openVoucherDialog()">
          <el-icon><Plus /></el-icon>
          新建凭证
        </el-button>
        <el-button @click="vchrProc.handlePrintVouchers">
          <el-icon><Printer /></el-icon>
          打印
        </el-button>
        <el-button @click="vchrProc.handleExportVouchers">
          <el-icon><Download /></el-icon>
          导出
        </el-button>
      </div>
    </div>

    <VchrFilter
      :voucher-query="vchr.voucherQuery"
      @search="vchr.fetchVouchers"
      @reset="vchr.resetVoucherQuery"
    />

    <VchrTbl
      :vouchers="vchr.vouchers.value"
      :voucher-loading="vchr.voucherLoading.value"
      :voucher-total="vchr.voucherTotal.value"
      :voucher-query-params="vchr.voucherQueryParams"
      :format-money="vchr.formatMoney"
      :get-voucher-status-label="vchr.getVchrStatusLabel"
      :get-voucher-status-type="vchr.getVchrStatusType"
      @view="onView"
      @submit="vchrProc.submitVoucher"
      @review="vchrProc.reviewVoucher"
      @post="vchrProc.postVoucher"
      @page-change="vchr.fetchVouchers"
    />

    <VchrForm
      v-model:visible="voucherDialogVisible"
      :voucher-form-ref="vchr.voucherFormRef"
      :voucher-form="vchr.voucherForm"
      :voucher-submit-loading="vchr.voucherSubmitLoading.value"
      :voucher-rules="vchr.voucherRules"
      :leaf-subjects="vchr.leafSubjects.value"
      :total-debit="vchr.totalDebit.value"
      :total-credit="vchr.totalCredit.value"
      :is-balanced="vchr.isBalanced.value"
      :format-money="vchr.formatMoney"
      @add-entry="vchr.addEntry"
      @remove-entry="vchr.removeEntry"
      @submit-form="onSubmitForm"
    />

    <VchrDetail
      v-model:visible="voucherViewVisible"
      :current-voucher="vchr.currentVoucher.value"
      :format-money="vchr.formatMoney"
      :get-voucher-status-label="vchr.getVchrStatusLabel"
      :get-voucher-status-type="vchr.getVchrStatusType"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { Plus, Printer, Download } from '@element-plus/icons-vue'
import { useVchr } from './composables/useVchr'
import { useVchrProc } from './composables/useVchrProc'
import VchrFilter from './components/VchrFilter.vue'
import VchrTbl from './components/VchrTbl.vue'
import VchrForm from './components/VchrForm.vue'
import VchrDetail from './components/VchrDetail.vue'
import type { Voucher } from '@/api/finance'

const vchr = useVchr()
const vchrProc = useVchrProc(vchr.vouchers, vchr.fetchVouchers)

const voucherDialogVisible = ref(false)
const voucherViewVisible = ref(false)

const openVoucherDialog = () => {
  vchr.voucherFormRef.value?.resetFields()
  vchr.voucherForm.voucher_date = new Date().toISOString().split('T')[0]
  vchr.voucherForm.voucher_type = 'JZ'
  vchr.voucherForm.entries = [
    { subject_id: undefined, debit: 0, credit: 0, summary: '' },
    { subject_id: undefined, debit: 0, credit: 0, summary: '' },
  ]
  voucherDialogVisible.value = true
}

const onSubmitForm = async () => {
  const ok = await vchr.submitVoucherForm()
  if (ok) voucherDialogVisible.value = false
}

const onView = (row: Voucher) => {
  vchr.viewVoucher(row)
  voucherViewVisible.value = true
}

onMounted(() => {
  vchr.fetchSubjects()
  vchr.fetchVouchers()
})
</script>
