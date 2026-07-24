<!--
  VoucherListTab.vue - 凭证列表 Tab（拆分重构版）
  任务编号: P14 批 2 I-3 第 1 批
  拆分：870 行 → ~150 行 + 4 子组件 + 2 composable + 1 工具
  行为完全保持一致（仅结构重构）
  批次 287：适配 useTableApi（v-model:page/page-size, queryParams, 移除 onMounted loadData）
-->
<template>
  <div class="voucher-list-tab">
    <div class="page-header">
      <h2 class="page-title">凭证管理</h2>
    </div>

    <VoucherListFilter
      :query-params="vchr.queryParams"
      @fetch="vchr.handleSearch"
      @add="onAdd"
      @print="vchrProc.handlePrint"
      @export="vchrProc.handleExport"
      @update:query-params="(v) => Object.assign(vchr.queryParams, v)"
    />

    <VoucherListTable
      :table-data="vchr.tableData"
      :loading="vchr.loading"
      :total="vchr.total"
      v-model:page="vchr.page"
      v-model:page-size="vchr.pageSize"
      @view="onView"
      @edit="onEdit"
      @approve="vchrProc.handleApprove"
      @post="vchrProc.handlePost"
      @unpost="vchrProc.handleUnpost"
      @delete="vchrProc.handleDelete"
    />

    <VoucherListForm
      v-model:visible="dialogVisible"
      :title="vchr.dialogTitle"
      :form="vchr.form"
      :voucher-types="vchr.voucherTypes"
      :account-subject-options="vchr.accountSubjectOptions"
      @add-entry="vchr.addEntry"
      @remove-entry="vchr.removeEntry"
      @submit="onSubmitForm"
      @update:form="(v) => Object.assign(vchr.form, v)"
    />

    <VoucherListDetail
      v-model:visible="viewDialogVisible"
      :view-data="vchr.viewData"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, toRef } from 'vue'
import type { VoucherEntity } from '@/api/voucher'
import { useVchrLst } from './composables/useVchrLst'
import { useVchrLstProc } from './composables/useVchrLstProc'
import VoucherListFilter from './components/VoucherListFilter.vue'
import VoucherListTable from './components/VoucherListTable.vue'
import VoucherListForm from './components/VoucherListForm.vue'
import VoucherListDetail from './components/VoucherListDetail.vue'

const vchr = useVchrLst()
// 使用 toRef 包装 reactive 属性为 ref，保持 useVchrLstProc 内 getList() 能读取最新 tableData
// （reactive 自动解包 ref，直接传 vchr.tableData 会丢失响应性引用）
const vchrProc = useVchrLstProc(toRef(vchr, 'tableData'), vchr.loadData)

// 对话框可见性本地 ref
const dialogVisible = ref(false)
const viewDialogVisible = ref(false)
/** 新增凭证：composable 准备数据，本地打开对话框 */
const onAdd = async () => {
  await vchr.openAddDialog()
  dialogVisible.value = true
}

/** 编辑凭证 */
const onEdit = async (row: VoucherEntity) => {
  await vchr.openEditDialog(row)
  dialogVisible.value = true
}

/** 查看详情 */
const onView = async (row: VoucherEntity) => {
  await vchr.openViewDialog(row)
  viewDialogVisible.value = true
}

/** 提交表单后关闭对话框 */
const onSubmitForm = async () => {
  const ok = await vchr.handleSubmit()
  if (ok) dialogVisible.value = false
}

// 列表由 useTableApi setup 自动加载，onMounted 仅加载辅助数据
onMounted(() => {
  vchr.loadVoucherTypes()
  vchr.loadAccountSubjects()
})
</script>

<style scoped>
.filter-container {
  margin-bottom: 20px;
}

.filter-item {
  width: 100%;
}

.filter-actions {
  margin-top: 10px;
}

.pagination-wrapper {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}

.status-tag {
  display: inline-block;
  padding: 4px 12px;
  border-radius: 20px;
  font-size: 12px;
}
.status-draft {
  background: #f5f7fa;
  color: #909399;
}
.status-approved {
  background: #e6f7ff;
  color: #1890ff;
}
.status-posted {
  background: #f0f9eb;
  color: #67c23a;
}
</style>
