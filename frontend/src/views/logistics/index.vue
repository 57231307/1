<!--
  logistics/index.vue - 物流管理（拆分重构版）
  任务编号: P14 批 2 I-3 第 4 批
  拆分：605 行 → ~150 行 + 6 子组件 + 2 composable + 1 工具
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

    <LgsStat :stats="lgs.stats" />

    <LgsFilter
      :params="lgs.queryParams"
      :date-range="lgs.dateRange"
      @search="lgs.handleQuery"
      @reset="lgs.handleReset"
    />

    <LgsTbl
      :data="lgs.tableData"
      :loading="lgs.loading"
      :total="lgs.total"
      :query-params="lgs.queryParams"
      @view="lgsProc.handleView"
      @edit="lgsProc.handleEdit"
      @ship="lgsProc.handleShip"
      @update-status="lgsProc.handleUpdateStatus"
      @delete="lgsProc.handleDelete"
      @size-change="lgs.fetchData"
      @current-change="lgs.fetchData"
    />

    <LgsForm
      v-model:visible="lgs.dialogVisible"
      :is-edit="lgs.isEdit"
      :loading="lgs.submitLoading"
      :orders="lgs.orders"
      :form="lgs.formData"
      :rules="lgs.formRules"
      @submit="lgsProc.handleSubmit"
    />

    <LgsDetail
      v-model:visible="lgs.detailDialogVisible"
      :detail="lgs.detailData"
    />

    <LgsStatDlg
      v-model:visible="lgs.statusDialogVisible"
      :form="lgs.statusForm"
      :statuses="lgsProc.availableStatuses"
      @submit="lgsProc.handleStatusSubmit"
    />
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { Plus } from '@element-plus/icons-vue'
import { useLgs } from './composables/useLgs'
import { useLgsProc } from './composables/useLgsProc'
import LgsStat from './components/LgsStat.vue'
import LgsFilter from './components/LgsFilter.vue'
import LgsTbl from './components/LgsTbl.vue'
import LgsForm from './components/LgsForm.vue'
import LgsDetail from './components/LgsDetail.vue'
import LgsStatDlg from './components/LgsStatDlg.vue'

// 业务状态
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

onMounted(() => {
  lgs.fetchData()
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
