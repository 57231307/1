<!--
  sales-price/index.vue - 销售价格管理（拆分重构版）
  任务编号: P14 批 2 I-3 第 3 批
  拆分：677 行 → ~150 行 + 5 子组件 + 2 composable + 1 工具
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="sales-price-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">销售价格管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>销售管理</el-breadcrumb-item>
          <el-breadcrumb-item>销售价格</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="onCreate">
          <el-icon><Plus /></el-icon>
          新建价格
        </el-button>
        <el-button @click="spProc.handleStrategy">
          <el-icon><Setting /></el-icon>
          价格策略
        </el-button>
        <el-button @click="onExport">
          <el-icon><Download /></el-icon>
          导出
        </el-button>
      </div>
    </div>

    <SpFilter
      :query-params="sp.queryParams"
      :customers="sp.customers"
      :products="sp.products"
      @query="sp.handleQuery"
      @reset="sp.handleReset"
    />

    <SpTbl
      :price-list="sp.priceList"
      :loading="sp.loading"
      :total="sp.total"
      :query-params="sp.queryParams"
      @view="spProc.handleView"
      @edit="onEdit"
      @approve="spProc.handleApprove"
      @history="spProc.handleHistory"
      @size-change="sp.handleSizeChange"
      @current-change="sp.handleCurrentChange"
    />

    <SpForm
      v-model:visible="dialogVisible"
      :title="sp.dialogTitle"
      :form-data="sp.formData"
      :form-rules="sp.formRules"
      :customers="sp.customers"
      :products="sp.products"
      @submit="onSubmitForm"
    />

    <SpView v-model:visible="spProc.viewDialogVisible" :view-data="spProc.viewData" />

    <SpHistory v-model:visible="spProc.historyVisible" :history-list="spProc.historyList" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { Plus, Setting, Download } from '@element-plus/icons-vue'
import type { SalesPrice } from '@/api/sales-price'
import { useSp } from './composables/useSp'
import { useSpProc } from './composables/useSpProc'
import SpFilter from './components/SpFilter.vue'
import SpTbl from './components/SpTbl.vue'
import SpForm from './components/SpForm.vue'
import SpView from './components/SpView.vue'
import SpHistory from './components/SpHistory.vue'

const sp = useSp()
const spProc = useSpProc({
  getList: sp.getList,
})

// 对话框可见性本地 ref
const dialogVisible = ref(false)

/** 新建价格 */
const onCreate = () => {
  sp.prepareCreate()
  dialogVisible.value = true
}

/** 编辑价格 */
const onEdit = (row: SalesPrice) => {
  sp.prepareEdit(row)
  dialogVisible.value = true
}

/** 提交表单 */
const onSubmitForm = async () => {
  const ok = await sp.handleSubmitForm()
  if (ok) dialogVisible.value = false
}

/** 导出当前列表 */
const onExport = () => {
  spProc.handleExport(sp.priceList)
}

onMounted(() => {
  sp.initLoad()
})
</script>

<style scoped>
.sales-price-page {
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
