<!--
  SchedulingGanttAuto.vue - 甘特图自动排程参数对话框
  任务编号: P14 批 2 I-3 第 2 批（拆分原 scheduling/gantt.vue）
  P9-3 批次 F 重构：移除 vue/no-mutating-props 抑制，改用本地 ref 镜像 + watch 防循环
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('scheduling.ganttAuto.title')"
    width="500px"
    :aria-label="t('scheduling.ganttAuto.ariaLabel.dialog')"
    @update:model-value="onVisibleChange"
  >
    <el-form :model="localForm" label-width="120px" :aria-label="t('scheduling.ganttAuto.ariaLabel.form')">
      <el-form-item :label="t('scheduling.ganttAuto.form.startDate')">
        <el-date-picker
          v-model="localForm.start_date"
          type="date"
          :placeholder="t('scheduling.ganttAuto.placeholder.date')"
          style="width: 100%"
        />
      </el-form-item>
      <el-form-item :label="t('scheduling.ganttAuto.form.endDate')">
        <el-date-picker
          v-model="localForm.end_date"
          type="date"
          :placeholder="t('scheduling.ganttAuto.placeholder.date')"
          style="width: 100%"
        />
      </el-form-item>
      <el-form-item :label="t('scheduling.ganttAuto.form.priorityMode')">
        <el-select v-model="localForm.priority_mode" style="width: 100%">
          <el-option :label="t('scheduling.ganttAuto.option.priorityFifo')" value="fifo" />
          <el-option :label="t('scheduling.ganttAuto.option.priorityPriority')" value="priority" />
          <el-option :label="t('scheduling.ganttAuto.option.priorityDueDate')" value="due_date" />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('scheduling.ganttAuto.form.optimizationTarget')">
        <el-select v-model="localForm.optimization_target" style="width: 100%">
          <el-option :label="t('scheduling.ganttAuto.option.targetMinIdle')" value="min_idle" />
          <el-option :label="t('scheduling.ganttAuto.option.targetMinDelay')" value="min_delay" />
          <el-option :label="t('scheduling.ganttAuto.option.targetBalanceLoad')" value="balance_load" />
        </el-select>
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="onCancel">{{ t('scheduling.ganttAuto.button.cancel') }}</el-button>
      <el-button type="primary" :loading="scheduling" @click="emit('confirm')">{{ t('scheduling.ganttAuto.button.startSchedule') }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import type { SchedulingParams } from '@/api/scheduling'

const { t } = useI18n({ useScope: 'global' })

const props = defineProps<{
  // 对话框可见性
  visible: boolean
  // 排程参数（由父组件管理，子组件通过 emit('update:form') 回写）
  scheduleForm: SchedulingParams
  // 排程进行中
  scheduling: boolean
}>()

// 定义事件
const emit = defineEmits<{
  // 关闭
  (e: 'update:visible', value: boolean): void
  // 确认执行
  (e: 'confirm'): void
  // 整体回写表单
  (e: 'update:form', form: SchedulingParams): void
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localForm = ref<SchedulingParams>({ ...props.scheduleForm })

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local
watch(
  () => props.scheduleForm,
  (newForm) => {
    if (syncing) return
    syncing = true
    localForm.value = { ...newForm }
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

// 本地变化时通知父组件
watch(
  localForm,
  (newForm) => {
    if (syncing) return
    syncing = true
    emit('update:form', { ...newForm })
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

/** 关闭对话框 */
const onVisibleChange = (v: boolean) => {
  emit('update:visible', v)
}

/** 取消 */
const onCancel = () => {
  emit('update:visible', false)
}
</script>
