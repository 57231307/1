<!--
  purchase-price/index.vue - 采购价格管理（拆分重构版）
  任务编号: P14 批 2 I-3 第 3 批
  拆分：622 行 → ~150 行 + 5 子组件 + 2 composable + 1 工具
  批次 285：PpFilter/PpTbl 接入 useTableApi（v-model:page/page-size + @fetch + @update:queryParams）
-->
<template>
  <div class="purchase-price-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">采购价格管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>采购管理</el-breadcrumb-item>
          <el-breadcrumb-item>采购价格</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="onCreate">
          <el-icon><Plus /></el-icon>
          新建价格
        </el-button>
        <el-button @click="ppProc.handleExport">
          <el-icon><Download /></el-icon>
          导出
        </el-button>
      </div>
    </div>

    <PpFilter
      :query-params="pp.queryParams"
      :suppliers="pp.suppliers"
      :products="pp.products"
      @fetch="pp.handleQuery"
      @update:query-params="(v) => Object.assign(pp.queryParams, v)"
    />

    <PpTbl
      v-model:page="pp.page"
      v-model:page-size="pp.pageSize"
      :price-list="pp.priceList"
      :loading="pp.loading"
      :total="pp.total"
      @view="ppProc.handleView"
      @edit="onEdit"
      @disable="ppProc.handleDisable"
      @history="ppProc.handleHistory"
    />

    <PpForm
      v-model:visible="dialogVisible"
      :title="pp.dialogTitle"
      :form-data="pp.formData"
      :suppliers="pp.suppliers"
      :products="pp.products"
      @submit="onSubmitForm"
      @update:form-data="(v) => Object.assign(pp.formData, v)"
    />

    <PpHistory v-model:visible="ppProc.historyVisible" :history-list="ppProc.historyList" />

    <PpDetail v-model:visible="ppProc.viewDialogVisible" :view-data="ppProc.viewData" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { Plus, Download } from '@element-plus/icons-vue'
import type { PurchasePrice } from '@/api/purchase-price'
import { usePp } from './composables/usePp'
import { usePpProc } from './composables/usePpProc'
import PpFilter from './components/PpFilter.vue'
import PpTbl from './components/PpTbl.vue'
import PpForm from './components/PpForm.vue'
import PpHistory from './components/PpHistory.vue'
import PpDetail from './components/PpDetail.vue'

const pp = usePp()
const ppProc = usePpProc({
  getList: pp.getList,
})

// 对话框可见性本地 ref
const dialogVisible = ref(false)

/** 新建价格 */
const onCreate = () => {
  pp.prepareCreate()
  dialogVisible.value = true
}

/** 编辑价格 */
const onEdit = (row: PurchasePrice) => {
  pp.prepareEdit(row)
  dialogVisible.value = true
}

/** 提交表单 */
const onSubmitForm = async () => {
  const ok = await pp.handleSubmitForm()
  if (ok) dialogVisible.value = false
}

// 列表由 useTableApi setup 自动加载，onMounted 仅加载辅助数据
onMounted(() => {
  pp.initLoad()
})
</script>

<style scoped>
.purchase-price-page {
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
