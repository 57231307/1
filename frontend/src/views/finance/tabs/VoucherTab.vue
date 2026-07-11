<!--
  VoucherTab.vue - 凭证管理 Tab（拆分重构版）
  任务编号: P14 批 1 B3 I-2
  拆分：567 行 → ~95 行 + 4 子组件 + 2 composable + 1 工具
  行为完全保持一致（仅结构重构）
  批次 289：适配 useTableApi（v-model:page/page-size, queryParams, 移除 onMounted fetchVouchers）
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
      :query-params="vchr.queryParams"
      @fetch="vchr.handleSearch"
      @update:query-params="(v) => Object.assign(vchr.queryParams, v)"
    />

    <VchrTbl
      :vouchers="vchr.vouchers"
      :voucher-loading="vchr.voucherLoading"
      :voucher-total="vchr.voucherTotal"
      v-model:page="vchr.page"
      v-model:page-size="vchr.pageSize"
      :format-money="vchr.formatMoney"
      :get-voucher-status-label="vchr.getVchrStatusLabel"
      :get-voucher-status-type="vchr.getVchrStatusType"
      @view="onView"
      @submit="vchrProc.submitVoucher"
      @review="vchrProc.reviewVoucher"
      @post="vchrProc.postVoucher"
    />

    <VchrForm
      v-model:visible="voucherDialogVisible"
      :voucher-form-ref="voucherFormRef"
      :voucher-form="vchr.voucherForm"
      :voucher-submit-loading="vchr.voucherSubmitLoading"
      :voucher-rules="vchr.voucherRules"
      :leaf-subjects="vchr.leafSubjects"
      :total-debit="vchr.totalDebit"
      :total-credit="vchr.totalCredit"
      :is-balanced="vchr.isBalanced"
      :format-money="vchr.formatMoney"
      @add-entry="vchr.addEntry"
      @remove-entry="vchr.removeEntry"
      @submit-form="onSubmitForm"
      @update:voucher-form="(v) => Object.assign(vchr.voucherForm, v)"
    />

    <VchrDetail
      v-model:visible="voucherViewVisible"
      :current-voucher="vchr.currentVoucher"
      :format-money="vchr.formatMoney"
      :get-voucher-status-label="vchr.getVchrStatusLabel"
      :get-voucher-status-type="vchr.getVchrStatusType"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, toRef } from 'vue'
import { Plus, Printer, Download } from '@element-plus/icons-vue'
import { useVchr } from './composables/useVchr'
import { useVchrProc } from './composables/useVchrProc'
import VchrFilter from './components/VchrFilter.vue'
import VchrTbl from './components/VchrTbl.vue'
import VchrForm from './components/VchrForm.vue'
import VchrDetail from './components/VchrDetail.vue'
import type { Voucher } from '@/api/finance'

const vchr = useVchr()
// 使用 toRef 包装 reactive 属性为 ref，保持 useVchrProc 内部能读取最新 vouchers
const vchrProc = useVchrProc(toRef(vchr, 'vouchers'), vchr.fetchVouchers)
// VchrForm 需要 ref-like 的 voucherFormRef（{ value: FormInstance | undefined }），
// reactive 会自动解包 ref，因此用 toRef 还原为 ref 传入
const voucherFormRef = toRef(vchr, 'voucherFormRef')

const voucherDialogVisible = ref(false)
const voucherViewVisible = ref(false)

const openVoucherDialog = () => {
  vchr.voucherFormRef?.resetFields()
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

// 列表由 useTableApi setup 自动加载，onMounted 仅加载辅助数据
onMounted(() => {
  vchr.fetchSubjects()
})
</script>
