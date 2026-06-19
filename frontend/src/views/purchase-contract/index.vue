<!--
  purchase-contract/index.vue - 采购合同管理（拆分重构版）
  任务编号: P14 批 2 I-3 第 3 批
  拆分：644 行 → ~150 行 + 4 子组件 + 2 composable + 1 工具
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="purchase-contract-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">采购合同管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>采购管理</el-breadcrumb-item>
          <el-breadcrumb-item>采购合同</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="onCreate">
          <el-icon><Plus /></el-icon>
          新建合同
        </el-button>
        <el-button @click="pcProc.handleExport">
          <el-icon><Download /></el-icon>
          导出
        </el-button>
      </div>
    </div>

    <PcFilter
      :query-params="pc.queryParams"
      :suppliers="pc.suppliers"
      @query="pc.handleQuery"
      @reset="pc.handleReset"
    />

    <PcTbl
      :contract-list="pc.contractList"
      :loading="pc.loading"
      :total="pc.total"
      :query-params="pc.queryParams"
      @view="onView"
      @edit="onEdit"
      @submit="pcProc.handleSubmit"
      @approve="pcProc.handleApprove"
      @execute="pcProc.handleExecute"
      @delete="pcProc.handleDelete"
      @size-change="pc.handleSizeChange"
      @current-change="pc.handleCurrentChange"
    />

    <PcForm
      v-model:visible="dialogVisible"
      :title="pc.dialogTitle"
      :form-data="pc.formData"
      :suppliers="pc.suppliers"
      @submit="onSubmitForm"
    />

    <PcDetail v-model:visible="detailDialogVisible" :view-data="viewData" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { Plus, Download } from '@element-plus/icons-vue'
import type { PurchaseContract } from '@/api/purchase-contract'
import { usePc } from './composables/usePc'
import { usePcProc } from './composables/usePcProc'
import PcFilter from './components/PcFilter.vue'
import PcTbl from './components/PcTbl.vue'
import PcForm from './components/PcForm.vue'
import PcDetail from './components/PcDetail.vue'

const pc = usePc()
const pcProc = usePcProc({
  getList: pc.getList,
})

// 对话框可见性本地 ref
const dialogVisible = ref(false)
const detailDialogVisible = ref(false)
const viewData = ref<any>({})

/** 新建合同 */
const onCreate = () => {
  pc.prepareCreate()
  dialogVisible.value = true
}

/** 编辑合同 */
const onEdit = (row: PurchaseContract) => {
  pc.prepareEdit(row)
  dialogVisible.value = true
}

/** 查看详情 */
const onView = (row: any) => {
  viewData.value = row
  detailDialogVisible.value = true
}

/** 提交表单 */
const onSubmitForm = async () => {
  const ok = await pc.handleSubmitForm()
  if (ok) dialogVisible.value = false
}

onMounted(() => {
  pc.initLoad()
})
</script>

<style scoped>
.purchase-contract-page {
  padding: 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.header-left {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.page-title {
  margin: 0;
  font-size: 24px;
  font-weight: 600;
}

.header-actions {
  display: flex;
  gap: 10px;
}
</style>
