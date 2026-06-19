<!--
  sales-contract/index.vue - 销售合同管理（拆分重构版）
  任务编号: P14 批 2 I-3 第 1 批
  拆分：717 行 → ~150 行 + 4 子组件 + 2 composable + 1 工具
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="sales-contract-page">
    <div class="page-header">
      <h2 class="page-title">销售合同管理</h2>
      <div class="header-actions">
        <el-button type="primary" @click="onCreate">
          <el-icon><Plus /></el-icon>
          新建合同
        </el-button>
        <el-button @click="scProc.handlePrint(sc.contractList)">
          <el-icon><Printer /></el-icon>
          打印
        </el-button>
        <el-button @click="scProc.handleExport(sc.contractList)">
          <el-icon><Download /></el-icon>
          导出
        </el-button>
      </div>
    </div>

    <ScFilter
      :query-params="sc.queryParams"
      :customers="sc.customers"
      :date-range="sc.dateRange"
      @query="sc.handleQuery"
      @reset="sc.handleReset"
      @date-change="onDateChange"
    />

    <ScTbl
      :contract-list="sc.contractList"
      :loading="sc.loading"
      :total="sc.total"
      :query-params="sc.queryParams"
      @view="scProc.handleView"
      @edit="onEdit"
      @submit-approval="scProc.handleSubmitForApproval"
      @approve="scProc.handleApprove"
      @execute="scProc.handleExecute"
      @delete="scProc.handleDelete"
      @size-change="sc.handleSizeChange"
      @current-change="sc.handleCurrentChange"
    />

    <ScForm
      v-model:visible="dialogVisible"
      :title="sc.dialogTitle"
      :form-data="sc.formData"
      :customers="sc.customers"
      @submit="onSubmitForm"
    />
  </div>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import { ref, onMounted } from 'vue'
import { Plus, Printer, Download } from '@element-plus/icons-vue'
import type { SalesContract } from '@/api/sales-contract'
import { useSc } from './composables/useSc'
import { useScProc } from './composables/useScProc'
import ScFilter from './components/ScFilter.vue'
import ScTbl from './components/ScTbl.vue'
import ScForm from './components/ScForm.vue'

const sc = useSc()
const scProc = useScProc({
  getList: sc.getList,
})

// 对话框可见性本地 ref
const dialogVisible = ref(false)

/** 新建合同 */
const onCreate = () => {
  sc.prepareCreate()
  dialogVisible.value = true
}

/** 编辑合同 */
const onEdit = (row: SalesContract) => {
  sc.prepareEdit(row)
  dialogVisible.value = true
}

/** 提交表单 */
const onSubmitForm = async () => {
  const ok = await sc.handleSubmitForm()
  if (ok) dialogVisible.value = false
}

/** 日期范围变化 */
const onDateChange = (v: [Date, Date] | null) => {
  sc.dateRange.value = v
  sc.handleDateChange()
}

onMounted(() => {
  sc.initLoad()
})
</script>

<style scoped>
.sales-contract-page {
  padding: 20px;
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
</style>
