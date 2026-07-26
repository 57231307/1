<!--
  SchedulingMachineParam.vue - 排产主页参数侧栏
  任务编号: P14 批 2 I-3 第 2 批（拆分原 scheduling/index.vue）
-->
<template>
  <el-card shadow="hover" class="param-card">
    <template #header>
      <div class="card-header">
        <span>{{ t('scheduling.machineParam.title') }}</span>
      </div>
    </template>
    <el-form :model="scheduleParams" label-width="90px" size="small" :aria-label="t('scheduling.machineParam.ariaLabel.form')">
      <el-form-item :label="t('scheduling.machineParam.form.scheduleRange')">
        <el-date-picker
          :model-value="dateRange"
          type="daterange"
          :range-separator="t('scheduling.machineParam.to')"
          :start-placeholder="t('scheduling.machineParam.placeholder.start')"
          :end-placeholder="t('scheduling.machineParam.placeholder.end')"
          style="width: 100%"
          @update:model-value="onDateChange"
        />
      </el-form-item>
      <el-form-item :label="t('scheduling.machineParam.form.priorityMode')">
        <el-select v-model="scheduleParams.priority_mode" style="width: 100%">
          <el-option :label="t('scheduling.machineParam.option.priorityFifo')" value="fifo" />
          <el-option :label="t('scheduling.machineParam.option.priorityPriority')" value="priority" />
          <el-option :label="t('scheduling.machineParam.option.priorityDueDate')" value="due_date" />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('scheduling.machineParam.form.optimizationTarget')">
        <el-select v-model="scheduleParams.optimization_target" style="width: 100%">
          <el-option :label="t('scheduling.machineParam.option.targetMinIdle')" value="min_idle" />
          <el-option :label="t('scheduling.machineParam.option.targetMinDelay')" value="min_delay" />
          <el-option :label="t('scheduling.machineParam.option.targetBalanceLoad')" value="balance_load" />
        </el-select>
      </el-form-item>
      <el-form-item>
        <el-button
          type="primary"
          :loading="scheduling"
          style="width: 100%"
          @click="emit('auto-schedule')"
        >
          <el-icon><Cpu /></el-icon>
          {{ t('scheduling.machineParam.button.execute') }}
        </el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const { t } = useI18n({ useScope: 'global' })

// 排程参数类型
interface ScheduleParams {
  start_date: string
  end_date: string
  priority_mode: string
  optimization_target: string
}

// 排产参数侧栏属性
defineProps<{
  // 排程参数
  scheduleParams: ScheduleParams
  // 日期范围
  dateRange: [Date, Date] | null
  // 自动排程进行中
  scheduling: boolean
}>()

// 定义事件
const emit = defineEmits<{
  // 自动排程
  (e: 'auto-schedule'): void
  // 日期变化
  (e: 'date-change', value: [Date, Date] | null): void
}>()

/** 日期变化 */
const onDateChange = (v: [Date, Date] | null) => {
  emit('date-change', v)
}
</script>

<style scoped>
.param-card {
  margin-bottom: 20px;
  border-radius: 12px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.card-header span {
  font-size: 16px;
  font-weight: 600;
  color: #303133;
}
</style>
