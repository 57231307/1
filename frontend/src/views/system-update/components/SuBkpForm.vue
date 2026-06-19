<!--
  SuBkpForm.vue - 创建系统备份对话框
  拆分自 system-update/index.vue（P14 批 2 I-3 第 1 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-dialog
    :model-value="visible"
    title="创建备份"
    width="500px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form :model="form" label-width="100px">
      <el-form-item label="备份类型" prop="backup_type">
        <el-select
          :model-value="form.backup_type"
          style="width: 100%"
          @update:model-value="(v: 'full' | 'incremental' | 'database' | 'files') => (form.backup_type = v)"
        >
          <el-option label="完整备份" value="full" />
          <el-option label="增量备份" value="incremental" />
          <el-option label="数据库备份" value="database" />
          <el-option label="文件备份" value="files" />
        </el-select>
      </el-form-item>
      <el-form-item label="描述" prop="description">
        <el-input
          :model-value="form.description"
          type="textarea"
          :rows="3"
          placeholder="请输入备份描述"
          @update:model-value="(v: string) => (form.description = v)"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">取消</el-button>
      <el-button
        type="primary"
        :loading="submitLoading"
        @click="emit('submit')"
        >开始备份</el-button
      >
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */

interface BackupForm {
  backup_type: 'full' | 'incremental' | 'database' | 'files'
  description: string
}

/**
 * 创建系统备份对话框组件
 */
const props = defineProps<{
  // 对话框可见性
  visible: boolean
  // 表单数据（reactive 包装以便双向同步）
  form: BackupForm
  // 提交中状态
  submitLoading: boolean
}>()

const emit = defineEmits<{
  // 关闭对话框
  'update:visible': [v: boolean]
  // 提交表单
  submit: []
}>()

void props
</script>
