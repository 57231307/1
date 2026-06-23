<!--
  scheduling/gantt.vue - 排产甘特图（拆分重构版）
  任务编号: P14 批 2 I-3 第 2 批
  拆分：691 行 → ~150 行 + 5 子组件 + 2 composable + 1 工具
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="scheduling-gantt">
    <SchGTool
      :gantt-data="schG.ganttData"
      :date-range="schG.dateRange"
      :date-range-text="schG.dateRangeText"
      :scheduling="schGProc.scheduling"
      @back="onBack"
      @refresh="schG.fetchGanttData"
      @auto-schedule="schG.handleAutoSchedule"
      @date-change="onDateChange"
    />

    <SchGChart
      :gantt-data="schG.ganttData"
      :loading="schG.loading"
      @task-click="schG.handleTaskClick"
    />

    <SchGAuto
      v-model:visible="schG.autoScheduleDialogVisible"
      :schedule-form="schG.scheduleForm"
      :scheduling="schGProc.scheduling"
      @update:form="(v) => (schG.scheduleForm = v)"
      @confirm="schGProc.confirmAutoSchedule"
    />

    <SchGAdj
      v-model:visible="schG.adjustDialogVisible"
      :adjust-task="schG.adjustTask"
      :adjust-form="schG.adjustForm"
      :adjusting="schG.adjusting"
      :work-centers="schG.ganttData.work_centers"
      @update:form="(v) => (schG.adjustForm = v)"
      @confirm="schG.confirmAdjust"
    />

    <SchGConf
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
import SchGTool from './components/SchGTool.vue'
import SchGChart from './components/SchGChart.vue'
import SchGAuto from './components/SchGAuto.vue'
import SchGAdj from './components/SchGAdj.vue'
import SchGConf from './components/SchGConf.vue'

const router = useRouter()

const schG = useSchG()
const schGProc = useSchGProc({
  fetchGanttData: schG.fetchGanttData,
  conflictList: schG.conflictList,
  conflictDialogVisible: schG.conflictDialogVisible,
  scheduleForm: schG.scheduleForm,
  autoScheduleDialogVisible: schG.autoScheduleDialogVisible,
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
