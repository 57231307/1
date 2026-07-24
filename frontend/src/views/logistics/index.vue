<!--
  logistics/index.vue - 物流管理（拆分重构版）
  任务编号: P14 批 2 I-3 第 4 批
  拆分：605 行 → ~120 行 + 6 子组件 + 2 composable + 1 工具
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="logistics">
    <div class="page-header">
      <h2>物流管理</h2>
      <el-button type="primary" @click="lgsProc.handleCreate">
        <el-icon><Plus /></el-icon>
        新建运单
      </el-button>
    </div>

    <LogisticsStat :stats="lgs.stats" />

    <LogisticsFilter
      :query-params="lgs.queryParams"
      :date-range="lgs.dateRange"
      @fetch="lgs.handleQuery"
      @date-change="lgs.handleDateChange"
      @update:query-params="(v) => Object.assign(lgs.queryParams, v)"
    />

    <LogisticsTable
      v-model:page="lgs.page"
      v-model:page-size="lgs.pageSize"
      :data="lgs.tableData"
      :loading="lgs.loading"
      :total="lgs.total"
      @view="lgsProc.handleView"
      @edit="lgsProc.handleEdit"
      @ship="lgsProc.handleShip"
      @update-status="lgsProc.handleUpdateStatus"
      @delete="lgsProc.handleDelete"
    />

    <LogisticsForm
      v-model:visible="lgs.dialogVisible"
      :is-edit="lgs.isEdit"
      :loading="lgs.submitLoading"
      :orders="lgs.orders"
      :form="lgs.formData"
      :rules="lgs.formRules"
      @submit="lgsProc.handleSubmit"
      @update:form="(v) => Object.assign(lgs.formData, v)"
    />

    <LogisticsDetail
      v-model:visible="lgs.detailDialogVisible"
      :detail="lgs.detailData"
    />

    <LogisticsStatDialog
      v-model:visible="lgs.statusDialogVisible"
      :form="lgs.statusForm"
      :statuses="lgsProc.availableStatuses"
      @submit="lgsProc.handleStatusSubmit"
      @update:form="(v) => Object.assign(lgs.statusForm, v)"
    />
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { Plus } from '@element-plus/icons-vue'
import { useLgs } from './composables/useLgs'
import { useLgsProc } from './composables/useLgsProc'
import LogisticsStat from './components/LogisticsStat.vue'
import LogisticsFilter from './components/LogisticsFilter.vue'
import LogisticsTable from './components/LogisticsTable.vue'
import LogisticsForm from './components/LogisticsForm.vue'
import LogisticsDetail from './components/LogisticsDetail.vue'
import LogisticsStatDialog from './components/LogisticsStatDialog.vue'

// 业务状态（reactive 包装，父组件可直接访问字段）
const lgs = useLgs()
const lgsProc = useLgsProc({
  detailDialogVisible: lgs.detailDialogVisible,
  detailData: lgs.detailData,
  isEdit: lgs.isEdit,
  formData: lgs.formData,
  submitLoading: lgs.submitLoading,
  dialogVisible: lgs.dialogVisible,
  statusForm: lgs.statusForm,
  statusDialogVisible: lgs.statusDialogVisible,
  fetchData: lgs.fetchData,
})

// 懒加载标记
const hasLoaded = createLazyLoader()

// 列表由 useTableApi setup 自动加载，onMounted 仅加载辅助数据（关联订单）
onMounted(() => {
  loadIfNot('orders', lgs.fetchOrders, hasLoaded)
})
</script>

<style scoped>
.logistics {
  padding: 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.page-header h2 {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
}
</style>
