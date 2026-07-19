<!--
  DiTplUpload.vue - 数据导入文件上传对话框
  拆分自 data-import/index.vue（P14 批 2 I-3 第 5 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    title="导入数据"
    width="500px"
    aria-label="导入数据对话框"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-upload
      ref="uploadRef"
      :auto-upload="false"
      :limit="1"
      :on-exceed="handleExceed"
      :on-change="handleFileChange"
      accept=".xlsx,.csv,.json"
    >
      <template #trigger>
        <el-button type="primary">选择文件</el-button>
      </template>
      <template #tip>
        <div class="el-upload__tip">只能上传 .xlsx/.csv/.json 文件，且不超过 10MB</div>
      </template>
    </el-upload>
    <template #footer>
      <el-button @click="emit('update:visible', false)">取消</el-button>
      <el-button type="primary" :loading="loading" @click="emit('submit')">开始导入</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { ElMessage } from 'element-plus'

/**
 * 文件上传对话框
 */
defineProps<{
  // 可见性
  visible: boolean
  // 上传加载状态
  loading: boolean
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
  submit: []
  // 文件超出限制
  exceed: []
  // 文件变化
  'file-change': [file: { raw?: File }]
}>()

// 上传组件 ref
const uploadRef = ref<{ clearFiles: () => void } | null>(null)

// 暴露给父组件访问
defineExpose({ uploadRef })

/** 文件超出限制 */
const handleExceed = () => {
  ElMessage.warning('只能上传一个文件')
  emit('exceed')
}

/** 文件变化 */
const handleFileChange = (file: { raw?: File }) => {
  emit('file-change', file)
}
</script>
