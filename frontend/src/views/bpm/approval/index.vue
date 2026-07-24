<!--
  bpm/approval/index.vue - BPM 审批中心（拆分重构版）
  任务编号: P14 批 2 I-3 第 4 批
  拆分：618 行 → ~150 行 + 6 子组件 + 2 composable + 1 工具
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="bpm-approval-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">审批中心</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>审批管理</el-breadcrumb-item>
          <el-breadcrumb-item>审批中心</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
    </div>

    <BpmApprovalStat :stats="bpmAp.stats" />

    <el-tabs v-model="activeTab" @tab-change="handleTabChange">
      <el-tab-pane label="待办任务" name="pending">
        <BpmApprovalPendingTable
          :tasks="bpmAp.pendingTasks"
          :loading="bpmAp.pendingLoading"
          :total="bpmAp.pendingTotal"
          v-model:page="bpmAp.pendingPage"
          v-model:page-size="bpmAp.pendingPageSize"
          @approve="bpmApProc.handleApprove"
          @reject="bpmApProc.handleReject"
          @transfer="bpmApProc.handleTransfer"
          @view-chain="bpmApProc.handleViewChain"
        />
      </el-tab-pane>

      <el-tab-pane label="已办任务" name="completed">
        <BpmApprovalCompletedTable
          :tasks="bpmAp.completedTasks"
          :loading="bpmAp.completedLoading"
          :total="bpmAp.completedTotal"
          v-model:page="bpmAp.completedPage"
          v-model:page-size="bpmAp.completedPageSize"
          @view-chain="bpmApProc.handleViewChain"
        />
      </el-tab-pane>
    </el-tabs>

    <BpmApprovalApprovalDialog
      v-model:visible="bpmApProc.approveDialogVisible"
      :current-task="bpmApProc.currentTask"
      :action="bpmApProc.approveAction"
      :submit-loading="bpmApProc.submitLoading"
      :approve-form="bpmApProc.approveForm"
      @confirm="bpmApProc.confirmApproval"
      @update:approve-form="(v) => Object.assign(bpmApProc.approveForm, v)"
    />

    <BpmApprovalTransferDialog
      v-model:visible="bpmApProc.transferDialogVisible"
      :current-task="bpmApProc.currentTask"
      :submit-loading="bpmApProc.submitLoading"
      :form="bpmApProc.transferForm"
      :rules="bpmApProc.transferRules"
      @confirm="bpmApProc.confirmTransfer"
      @update:form="(v) => Object.assign(bpmApProc.transferForm, v)"
    />

    <BpmApprovalChainDialog
      v-model:visible="bpmApProc.chainDialogVisible"
      :chain="bpmApProc.approvalChain"
    />
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useBpmAp } from './composables/useBpmAp'
import { useBpmApProc } from './composables/useBpmApProc'
import BpmApprovalStat from './components/BpmApprovalStat.vue'
import BpmApprovalPendingTable from './components/BpmApprovalPendingTable.vue'
import BpmApprovalCompletedTable from './components/BpmApprovalCompletedTable.vue'
import BpmApprovalApprovalDialog from './components/BpmApprovalApprovalDialog.vue'
import BpmApprovalTransferDialog from './components/BpmApprovalTransferDialog.vue'
import BpmApprovalChainDialog from './components/BpmApprovalChainDialog.vue'

// 当前激活的 Tab
const activeTab = ref('pending')

// 主业务 + 流程
const bpmAp = useBpmAp()
const bpmApProc = useBpmApProc({
  fetchPendingTasks: bpmAp.fetchPendingTasks,
})

/** 切换 Tab 重新加载 */
const handleTabChange = (tab: string | number) => {
  if (tab === 'pending') bpmAp.fetchPendingTasks()
  else if (tab === 'completed') bpmAp.fetchCompletedTasks()
}
</script>

<style scoped>
.bpm-approval-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 24px;
}
.header-left .page-title {
  font-size: 28px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 12px 0;
}
</style>
