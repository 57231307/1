<!--
  SuBkpForm.vue - 创建系统备份对话框
  拆分自 system-update/index.vue（P14 批 2 I-3 第 1 批）
  行为完全保持一致（仅结构重构）
  P9-3 批次 F 重构：移除 vue/no-mutating-props 抑制，改用本地 ref 镜像 + watch 防循环
-->
<template>
  <el-dialog
    :model-value="visible"
    title="创建备份"
    width="500px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form :model="localForm" label-width="100px">
      <el-form-item label="备份类型" prop="backup_type">
        <el-select
          v-model="localForm.backup_type"
          style="width: 100%"
        >
          <el-option label="完整备份" value="full" />
          <el-option label="增量备份" value="incremental" />
          <el-option label="数据库备份" value="database" />
          <el-option label="文件备份" value="files" />
        </el-select>
      </el-form-item>
      <el-form-item label="描述" prop="description">
        <el-input
          v-model="localForm.description"
          type="textarea"
          :rows="3"
          placeholder="请输入备份描述"
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
import { ref, watch, nextTick } from 'vue'

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
  // 备份表单（由父组件管理，子组件通过 emit('update:form') 回写）
  form: BackupForm
  // 提交中状态
  submitLoading: boolean
}>()

const emit = defineEmits<{
  // 关闭对话框
  'update:visible': [v: boolean]
  // 提交表单
  submit: []
  // 整体回写表单
  'update:form': [form: BackupForm]
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localForm = ref<BackupForm>({ ...props.form })

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local
watch(
  () => props.form,
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
</script>
