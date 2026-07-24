<!--
  scheduling/index.vue - 排产管理（拆分重构版）
  任务编号: P14 批 2 I-3 第 2 批
  拆分：689 行 → ~150 行 + 5 子组件 + 2 composable + 1 工具
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="scheduling-page">
    <SchedulingMachineTool
      :stats="schM.stats"
      :scheduling="schMProc.scheduling"
      @auto-schedule="schMProc.handleAutoSchedule"
      @goto-gantt="onGotoGantt"
    />

    <el-row :gutter="20">
      <el-col :xs="24" :lg="16">
        <SchedulingMachineTable
          :task-list="schM.taskList"
          :task-loading="schM.taskLoading"
          :total="schM.total"
          v-model:current-page="schM.currentPage"
          v-model:page-size="schM.pageSize"
          :filter-status="schM.filterStatus"
          @update:filter-status="(v) => (schM.filterStatus = v)"
          @adjust="(row) => schM.handleAdjust(row)"
          @conflict-detail="(row) => schM.showConflictDetail(row)"
          @refresh="schM.fetchTasks"
          @filter-change="schM.handleFilterChange"
        />
      </el-col>

      <el-col :xs="24" :lg="8">
        <SchedulingMachineConflict
          :conflict-list="schM.conflictList"
          :conflict-loading="schM.conflictLoading"
          @detect="schM.fetchConflicts"
        />

        <SchedulingMachineParam
          :schedule-params="schM.scheduleParams"
          :date-range="schM.dateRange"
          :scheduling="schMProc.scheduling"
          @auto-schedule="schMProc.handleAutoSchedule"
          @date-change="onDateChange"
        />
      </el-col>
    </el-row>

    <SchedulingMachineAdjust
      v-model:visible="schM.adjustDialogVisible"
      :adjust-task="schM.adjustTask"
      :adjust-form="schM.adjustForm"
      :adjusting="schM.adjusting"
      @update:form="(v) => (schM.adjustForm = v)"
      @confirm="onConfirmAdjust"
    />
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useSchM } from './composables/useSchM'
import { useSchMProc } from './composables/useSchMProc'
import SchedulingMachineTool from './components/SchedulingMachineTool.vue'
import SchedulingMachineTable from './components/SchedulingMachineTable.vue'
import SchedulingMachineConflict from './components/SchedulingMachineConflict.vue'
import SchedulingMachineParam from './components/SchedulingMachineParam.vue'
import SchedulingMachineAdjust from './components/SchedulingMachineAdjust.vue'

const router = useRouter()

const schM = useSchM()
const schMProc = useSchMProc({
  fetchTasks: schM.fetchTasks,
  dateRange: schM.dateRange,
  scheduleParams: schM.scheduleParams,
  setConflictList: (v) => { schM.conflictList = v },
  stats: schM.stats,
})

/** 跳转到甘特图 */
const onGotoGantt = () => {
  router.push('/scheduling/gantt')
}

/** 确认调整 */
const onConfirmAdjust = async () => {
  await schM.confirmAdjust()
}

/** 日期范围变化 */
const onDateChange = (v: [Date, Date] | null) => {
  schM.dateRange = v
}

onMounted(() => {
  schM.initLoad()
})
</script>

<style scoped>
.scheduling-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
</style>
