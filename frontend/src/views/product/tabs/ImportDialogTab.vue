<!--
  ImportDialogTab.vue - 产品导入对话框
  来源：原 product/index.vue 中 导入对话框
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    title="导入产品"
    width="500px"
    :close-on-click-modal="false"
    aria-label="产品导入对话框"
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <div style="margin-bottom: 16px">
      <el-alert type="info" :closable="false">
        <template #title>
          <div>请先下载导入模板，按照模板格式填写数据后上传。</div>
        </template>
      </el-alert>
    </div>
    <div style="margin-bottom: 16px">
      <el-button type="primary" link @click="handleDownloadTemplate">
        <el-icon><Download /></el-icon>
        下载导入模板
      </el-button>
    </div>
    <el-upload
      ref="uploadRef"
      :auto-upload="false"
      :limit="1"
      accept=".xlsx,.xls,.csv"
      :on-change="handleFileChange"
      drag
    >
      <el-icon class="el-icon--upload"><Upload /></el-icon>
      <div class="el-upload__text">将文件拖到此处，或<em>点击上传</em></div>
      <template #tip>
        <div class="el-upload__tip">支持 .xlsx、.xls、.csv 格式文件</div>
      </template>
    </el-upload>
    <template #footer>
      <el-button @click="emit('update:modelValue', false)">取消</el-button>
      <el-button type="primary" :loading="importLoading" @click="handleSubmit">确定导入</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { ElMessage } from 'element-plus'
import { Download, Upload } from '@element-plus/icons-vue'
import { productApi } from '@/api/product'
import { logger } from '@/utils/logger'

interface Props {
  modelValue: boolean
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void
  (e: 'submitted'): void
}

defineProps<Props>()
const emit = defineEmits<Emits>()

const uploadRef = ref()
const importFile = ref<File | null>(null)
const importLoading = ref(false)

const handleFileChange = (file: { raw?: File }) => {
  if (file.raw) {
    importFile.value = file.raw
  }
}

const handleDownloadTemplate = async () => {
  try {
    await productApi.getImportTemplate()
    ElMessage.success('模板下载成功')
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '模板下载失败')
  }
}

const handleSubmit = async () => {
  if (!importFile.value) {
    ElMessage.warning('请选择要导入的文件')
    return
  }
  importLoading.value = true
  try {
    const res = await productApi.importProducts(importFile.value)
    const data = res.data as { success?: number; failed?: number } | undefined
    ElMessage.success(`导入成功: ${data?.success || 0} 条，失败: ${data?.failed || 0} 条`)
    importFile.value = null
    emit('update:modelValue', false)
    emit('submitted')
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '导入失败')
    logger.error('产品导入失败', err.message)
  } finally {
    importLoading.value = false
  }
}
</script>
