<!--
  material-shortage/index.vue - 物料短缺管理（拆分重构版）
  任务编号: P14 批 2 I-3 第 5 批
  拆分：590 行 → ~80 行 + 3 子组件 + 2 composable + 1 工具
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="material-shortage-page">
    <div class="page-header">
      <h2 class="page-title">物料缺料管理</h2>
    </div>

    <MsStat :summary="ms.summary" />

    <MsSevCard :summary="ms.summary" />

    <MsTbl
      :data="ms.shortageList"
      :loading="ms.tableLoading"
      :total="ms.total"
      :checking="ms.checking"
      :current-page="ms.currentPage"
      :page-size="ms.pageSize"
      :filter-severity="ms.filterSeverity"
      :filter-status="ms.filterStatus"
      @filter-change="msProc.handleFilterChange"
      @check="msProc.handleCheck"
      @notify="msProc.handleNotify"
      @resolve="msProc.handleResolve"
      @update:page="(v: number) => (ms.currentPage = v)"
      @update:size="(v: number) => (ms.pageSize = v)"
      @update:filter-severity="(v: string) => (ms.filterSeverity = v)"
      @update:filter-status="(v: string) => (ms.filterStatus = v)"
    />

  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { useMs } from './composables/useMs'
import { useMsProc } from './composables/useMsProc'
import MsStat from './components/MsStat.vue'
import MsSevCard from './components/MsSevCard.vue'
import MsTbl from './components/MsTbl.vue'

// 业务状态
const ms = useMs()
const msProc = useMsProc({
  currentPage: ms.currentPage,
  pageSize: ms.pageSize,
  total: ms.total,
  filterSeverity: ms.filterSeverity,
  filterStatus: ms.filterStatus,
  tableLoading: ms.tableLoading,
  checking: ms.checking,
  summary: ms.summary,
  shortageList: ms.shortageList,
  fetchSummary: ms.fetchSummary,
  fetchShortages: ms.fetchShortages,
  syncFilterToQuery: ms.syncFilterToQuery,
})

// 列表由 useTableApi setup 自动加载，onMounted 仅加载汇总
onMounted(() => {
  ms.fetchSummary()
})
</script>

<style scoped>
.material-shortage-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
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
</style>
