<script setup lang="ts">
/**
 * TplExp - 报表导出对话框（纯展示）
 * 任务编号: P13 批 1 B3 I-1（拆分 report/templates.vue 导出对话框）
 */
import { Download } from '@element-plus/icons-vue'

interface ExportFormData {
  template_id: number
  template_name: string
  format: 'pdf' | 'excel'
  date_range: { start: string; end: string }
}

interface Props {
  modelValue: boolean
  exportForm: ExportFormData
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
    title="导出报表"
    width="500px"
    @update:model-value="(v: boolean) => emit('update:modelValue', v)"
  >
    <el-form label-width="100px">
      <el-form-item label="模板名称">
        <el-input v-model="exportForm.template_name" disabled />
      </el-form-item>
      <el-form-item label="导出格式">
        <el-radio-group v-model="exportForm.format">
          <el-radio value="excel">Excel</el-radio>
          <el-radio value="pdf">PDF</el-radio>
        </el-radio-group>
      </el-form-item>
      <el-form-item label="日期范围">
        <el-date-picker
          v-model="exportForm.date_range.start"
          type="date"
          placeholder="开始日期"
          style="width: 45%; margin-right: 10px"
        />
        <el-date-picker
          v-model="exportForm.date_range.end"
          type="date"
          placeholder="结束日期"
          style="width: 45%"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="onCancel">取消</el-button>
      <el-button type="primary" @click="onSubmit">
        <el-icon><Download /></el-icon> 导出
      </el-button>
    </template>
  </el-dialog>
</template>
