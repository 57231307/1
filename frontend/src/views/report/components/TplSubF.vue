<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
/**
 * TplSubF - 报表订阅表单对话框（纯展示）
 * 任务编号: P13 批 1 B3 I-1（拆分 report/templates.vue 订阅表单对话框）
 */
interface SubFormData {
  id: number
  template_id: number
  template_name: string
  schedule: 'daily' | 'weekly' | 'monthly'
  schedule_time: string
  recipients: string
  format: 'pdf' | 'excel' | 'both'
  active: boolean
}

interface Props {
  modelValue: boolean
  subForm: SubFormData
  onSubmit: () => void
  onCancel: () => void
}

defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
}>()
</script>

<template>
  <el-dialog
    :model-value="modelValue"
    :title="subForm.id ? '编辑订阅' : '新建订阅'"
    width="600px"
    @update:model-value="(v: boolean) => emit('update:modelValue', v)"
  >
    <el-form label-width="100px">
      <el-form-item label="发送频率">
        <el-select v-model="subForm.schedule" style="width: 100%">
          <el-option label="每天" value="daily" />
          <el-option label="每周" value="weekly" />
          <el-option label="每月" value="monthly" />
        </el-select>
      </el-form-item>
      <el-form-item label="发送时间">
        <el-time-picker
          v-model="subForm.schedule_time"
          format="HH:mm"
          value-format="HH:mm"
          style="width: 100%"
        />
      </el-form-item>
      <el-form-item label="接收人邮箱">
        <el-input v-model="subForm.recipients" placeholder="多个邮箱用逗号分隔" />
      </el-form-item>
      <el-form-item label="导出格式">
        <el-radio-group v-model="subForm.format">
          <el-radio value="excel">Excel</el-radio>
          <el-radio value="pdf">PDF</el-radio>
          <el-radio value="both">PDF + Excel</el-radio>
        </el-radio-group>
      </el-form-item>
      <el-form-item label="状态">
        <el-switch v-model="subForm.active" active-text="启用" inactive-text="禁用" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="onCancel">取消</el-button>
      <el-button type="primary" @click="onSubmit">确定</el-button>
    </template>
  </el-dialog>
</template>
