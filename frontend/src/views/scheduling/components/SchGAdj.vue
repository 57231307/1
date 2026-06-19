<!-- eslint-disable vue/no-mutating-props -->
<!--
  SchGAdj.vue - 排产调整对话框
  任务编号: P14 批 2 I-3 第 2 批（拆分原 scheduling/gantt.vue）
-->
<template>
  <el-dialog
    :model-value="visible"
    title="调整排程时间"
    width="450px"
    @update:model-value="onVisibleChange"
  >
    <el-form :model="adjustForm" label-width="100px">
      <el-form-item label="工单号">
        <span>{{ adjustTask.order_no }}</span>
      </el-form-item>
      <el-form-item label="工作中心">
        <el-select v-model="adjustForm.work_center_id" style="width: 100%">
          <el-option
            v-for="wc in workCenters"
            :key="wc.id"
            :label="wc.name"
            :value="wc.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item label="开始时间">
        <el-date-picker
          v-model="adjustForm.start_time"
          type="datetime"
          placeholder="选择开始时间"
          style="width: 100%"
        />
      </el-form-item>
      <el-form-item label="结束时间">
        <el-date-picker
          v-model="adjustForm.end_time"
          type="datetime"
          placeholder="选择结束时间"
          style="width: 100%"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="onCancel">取消</el-button>
      <el-button type="primary" :loading="adjusting" @click="emit('confirm')">确认调整</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
// 工作中心类型
interface WorkCenter {
  id: number
  name: string
}

// 调整任务类型（所有字段可选，兼容空对象）
interface AdjustTask {
  order_no?: string
  [key: string]: any
}

// 调整表单类型
interface AdjustForm {
  work_center_id: number
  start_time: string
  end_time: string
}

// 排产调整对话框属性
defineProps<{
  // 对话框可见性
  visible: boolean
  // 调整任务
  adjustTask: AdjustTask
  // 调整表单
  adjustForm: AdjustForm
  // 调整中
  adjusting: boolean
  // 工作中心列表
  workCenters: WorkCenter[]
}>()

// 定义事件
const emit = defineEmits<{
  // 关闭
  (e: 'update:visible', value: boolean): void
  // 确认
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
