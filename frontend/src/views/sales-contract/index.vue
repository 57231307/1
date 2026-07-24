<!--
  sales-contract/index.vue - 销售合同管理（拆分重构版）
  任务编号: P14 批 2 I-3 第 1 批
  拆分：717 行 → ~150 行 + 4 子组件 + 2 composable + 1 工具
  批次 284：SalesContractFilter/SalesContractTable 接入 useTableApi（v-model:page/page-size + @fetch + @update:queryParams）
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
        <el-button @click="scProc.handleExport()">
          <el-icon><Download /></el-icon>
          导出
        </el-button>
      </div>
    </div>

    <SalesContractFilter
      :query-params="sc.queryParams"
      :customers="sc.customers"
      :date-range="sc.dateRange"
      @fetch="sc.handleQuery"
      @update:query-params="(v) => Object.assign(sc.queryParams, v)"
      @date-change="onDateChange"
    />

    <SalesContractTable
      v-model:page="sc.page"
      v-model:page-size="sc.pageSize"
      :contract-list="sc.contractList"
      :loading="sc.loading"
      :total="sc.total"
      @view="scProc.handleView"
      @edit="onEdit"
      @submit-approval="scProc.handleSubmitForApproval"
      @approve="scProc.handleApprove"
      @execute="scProc.handleExecute"
      @delete="scProc.handleDelete"
    />

    <SalesContractForm
      v-model:visible="dialogVisible"
      :title="sc.dialogTitle"
      :form-data="sc.formData"
      :customers="sc.customers"
      @update:form-data="(v) => Object.assign(sc.formData, v)"
      @submit="onSubmitForm"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { Plus, Printer, Download } from '@element-plus/icons-vue'
import type { SalesContract } from '@/api/sales-contract'
import { useSc } from './composables/useSc'
import { useScProc } from './composables/useScProc'
import SalesContractFilter from './components/SalesContractFilter.vue'
import SalesContractTable from './components/SalesContractTable.vue'
import SalesContractForm from './components/SalesContractForm.vue'

const sc = useSc()
const scProc = useScProc({
  getList: sc.getList,
  // V15 P0-S12 修复（Batch 475d）：传入当前筛选条件，用于后端导出
  // useTableApi 的 queryParams 为 Ref<Record<string, unknown>>，需类型断言以满足回调返回类型
  getQueryParams: () => ({
    keyword: sc.queryParams.keyword as string | undefined,
    status: sc.queryParams.status as string | undefined,
    customer_id: sc.queryParams.customer_id as number | undefined,
  }),
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
  sc.dateRange = v
  sc.handleDateChange()
}

// 列表由 useTableApi setup 自动加载，onMounted 仅加载辅助数据
onMounted(() => {
  sc.getCustomers()
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
