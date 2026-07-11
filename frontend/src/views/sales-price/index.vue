<!--
  sales-price/index.vue - 销售价格管理（拆分重构版）
  任务编号: P14 批 2 I-3 第 3 批
  拆分：677 行 → ~150 行 + 5 子组件 + 2 composable + 1 工具
  批次 284：SpFilter/SpTbl 接入 useTableApi（v-model:page/page-size + @fetch + @update:queryParams）
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
      @fetch="sp.handleQuery"
      @update:query-params="(v) => Object.assign(sp.queryParams, v)"
    />

    <SpTbl
      v-model:page="sp.page"
      v-model:page-size="sp.pageSize"
      :price-list="sp.priceList"
      :loading="sp.loading"
      :total="sp.total"
      @view="spProc.handleView"
      @edit="onEdit"
      @approve="spProc.handleApprove"
      @history="spProc.handleHistory"
    />

    <SpForm
      v-model:visible="dialogVisible"
      :title="sp.dialogTitle"
      :form-data="sp.formData"
      :form-rules="sp.formRules"
      :customers="sp.customers"
      :products="sp.products"
      @submit="onSubmitForm"
      @update:form-data="(v) => Object.assign(sp.formData, v)"
    />

    <SpView v-model:visible="spProc.viewDialogVisible" :view-data="spProc.viewData" />

    <SpHistory v-model:visible="spProc.historyVisible" :history-list="spProc.historyList" />

    <!-- 价格策略对话框（批次 95 P3-17 修复：展示阶梯/批量/合同策略列表） -->
    <el-dialog
      :model-value="spProc.strategyVisible"
      title="价格策略"
      width="800px"
      @update:model-value="(v: boolean) => (spProc.strategyVisible = v)"
    >
      <el-table v-loading="spProc.strategyLoading" :data="spProc.strategyList" border>
        <el-table-column prop="name" label="策略名称" min-width="120" show-overflow-tooltip />
        <el-table-column
          prop="description"
          label="描述"
          min-width="180"
          show-overflow-tooltip
        />
        <el-table-column prop="type" label="类型" width="100" align="center">
          <template #default="{ row }">
            {{ row.type === 'tiered' ? '阶梯定价' : row.type === 'volume' ? '批量定价' : '合同定价' }}
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'">
              {{ row.status === 'active' ? '活跃' : '停用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="规则数" width="80" align="center">
          <template #default="{ row }">{{ row.rules?.length || 0 }}</template>
        </el-table-column>
      </el-table>
    </el-dialog>
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

// 列表由 useTableApi setup 自动加载，onMounted 仅加载辅助数据
onMounted(() => {
  sp.getCustomers()
  sp.getProducts()
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
