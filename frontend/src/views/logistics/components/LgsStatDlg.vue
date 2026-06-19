<!--
  LgsStatDlg.vue - 物流管理更新状态对话框
  拆分自 logistics/index.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-dialog
    :model-value="visible"
    title="更新运单状态"
    width="400px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form :model="form" label-width="80px">
      <el-form-item label="当前状态">
        <el-tag :type="(getStatusTypeFmt(form.currentStatus) as any)">
          {{ getStatusTextFmt(form.currentStatus) }}
        </el-tag>
      </el-form-item>
      <el-form-item label="新状态">
        <el-select
          :model-value="form.newStatus"
          placeholder="选择新状态"
          @update:model-value="(v: string) => (form.newStatus = v)"
        >
          <el-option
            v-for="status in statuses"
            :key="status.value"
            :label="status.label"
            :value="status.value"
          />
        </el-select>
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">取消</el-button>
      <el-button type="primary" @click="emit('submit')">确定</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import { getStatusType, getStatusText } from '../composables/lgsFmts'

// 状态选项类型
interface StatusOption {
  label: string
  value: string
}

// 表单字段类型
interface LgsStatusForm {
  id: number
  currentStatus: string
  newStatus: string
}

/**
 * 物流更新状态对话框组件
 */
defineProps<{
  // 对话框可见性
  visible: boolean
  // 表单数据
  form: LgsStatusForm
  // 可选状态列表
  statuses: StatusOption[]
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
  submit: []
}>()

// 透传格式化函数
const getStatusTypeFmt = getStatusType
const getStatusTextFmt = getStatusText
</script>
