<!--
  scheduling/gantt.vue - 排产甘特图（拆分重构版）
  任务编号: P14 批 2 I-3 第 2 批
  拆分：691 行 → ~150 行 + 5 子组件 + 2 composable + 1 工具
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="scheduling-gantt">
    <SchedulingGanttTool
      :gantt-data="schG.ganttData"
      :date-range="schG.dateRange"
      :date-range-text="schG.dateRangeText"
      :scheduling="schGProc.scheduling"
      @back="onBack"
      @refresh="schG.fetchGanttData"
      @auto-schedule="schG.handleAutoSchedule"
      @date-change="onDateChange"
    />

    <SchedulingGanttChart
      :gantt-data="schG.ganttData"
      :loading="schG.loading"
      @task-click="schG.handleTaskClick"
    />

    <SchedulingGanttAuto
      v-model:visible="schG.autoScheduleDialogVisible"
      :schedule-form="schG.scheduleForm"
      :scheduling="schGProc.scheduling"
      @update:form="(v) => { schG.scheduleForm = v }"
      @confirm="schGProc.confirmAutoSchedule"
    />

    <SchedulingGanttAdjust
      v-model:visible="schG.adjustDialogVisible"
      :adjust-task="schG.adjustTask"
      :adjust-form="schG.adjustForm"
      :adjusting="schG.adjusting"
      :work-centers="schG.ganttData.work_centers"
      @update:form="(v) => (schG.adjustForm = v)"
      @confirm="schG.confirmAdjust"
    />

    <SchedulingGanttConflict
      v-model:visible="schG.conflictDialogVisible"
      :conflict-list="schG.conflictList"
    />
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useSchG } from './composables/useSchG'
import { useSchGProc } from './composables/useSchGProc'
import SchedulingGanttTool from './components/SchedulingGanttTool.vue'
import SchedulingGanttChart from './components/SchedulingGanttChart.vue'
import SchedulingGanttAuto from './components/SchedulingGanttAuto.vue'
import SchedulingGanttAdjust from './components/SchedulingGanttAdjust.vue'
import SchedulingGanttConflict from './components/SchedulingGanttConflict.vue'

const router = useRouter()

const schG = useSchG()
const schGProc = useSchGProc({
  fetchGanttData: schG.fetchGanttData,
  scheduleForm: schG.scheduleForm,
  setAutoScheduleDialogVisible: (v) => { schG.autoScheduleDialogVisible = v },
  setConflictList: (v) => { schG.conflictList = v },
  setConflictDialogVisible: (v) => { schG.conflictDialogVisible = v },
})

/** 返回排程管理 */
const onBack = () => {
  router.back()
}

/** 日期范围变化 */
const onDateChange = (v: [Date, Date] | null) => {
  schG.dateRange = v
  schG.fetchGanttData()
}

onMounted(() => {
  schG.fetchGanttData()
})
</script>

<style scoped>
.scheduling-gantt {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
</style>
