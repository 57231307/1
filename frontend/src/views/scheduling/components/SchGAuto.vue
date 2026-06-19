<!-- eslint-disable vue/no-mutating-props -->
<!--
  SchGAuto.vue - 自动排程参数对话框
  任务编号: P14 批 2 I-3 第 2 批（拆分原 scheduling/gantt.vue）
-->
<template>
  <el-dialog
    :model-value="visible"
    title="自动排程参数"
    width="500px"
    @update:model-value="onVisibleChange"
  >
    <el-form :model="scheduleForm" label-width="120px">
      <el-form-item label="排程开始日期">
        <el-date-picker
          v-model="scheduleForm.start_date"
          type="date"
          placeholder="选择日期"
          style="width: 100%"
        />
      </el-form-item>
      <el-form-item label="排程结束日期">
        <el-date-picker
          v-model="scheduleForm.end_date"
          type="date"
          placeholder="选择日期"
          style="width: 100%"
        />
      </el-form-item>
      <el-form-item label="优先级模式">
        <el-select v-model="scheduleForm.priority_mode" style="width: 100%">
          <el-option label="先进先出 (FIFO)" value="fifo" />
          <el-option label="优先级优先" value="priority" />
          <el-option label="交期优先" value="due_date" />
        </el-select>
      </el-form-item>
      <el-form-item label="优化目标">
        <el-select v-model="scheduleForm.optimization_target" style="width: 100%">
          <el-option label="最小化空闲时间" value="min_idle" />
          <el-option label="最小化延迟" value="min_delay" />
          <el-option label="均衡负载" value="balance_load" />
        </el-select>
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="onCancel">取消</el-button>
      <el-button type="primary" :loading="scheduling" @click="emit('confirm')">开始排程</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
// 排程参数类型
interface ScheduleForm {
  start_date: string
  end_date: string
  priority_mode: string
  optimization_target: string
}

// 自动排程对话框属性
const props = defineProps<{
  // 对话框可见性
  visible: boolean
  // 排程参数
  scheduleForm: ScheduleForm
  // 排程进行中
  scheduling: boolean
}>()

// 定义事件
const emit = defineEmits<{
  // 关闭
  (e: 'update:visible', value: boolean): void
  // 确认执行
  (e: 'confirm'): void
}>()

/** 关闭对话框 */
const onVisibleChange = (v: boolean) => {
  emit('update:visible', v)
}

/** 取消 */
const onCancel = () => {
  emit('update:visible', false)
}
</script>
