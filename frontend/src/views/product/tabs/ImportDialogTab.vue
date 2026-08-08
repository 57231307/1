<!--
  ImportDialogTab.vue - 产品导入对话框
  来源：原 product/index.vue 中 导入对话框
  拆分日期：2026-06-15 B3-4
  D05 Batch 8 Group B：接入 useI18n
-->
<template>
  <el-dialog
    :model-value="modelValue"
    :title="t('product.importDialogTab.title')"
    width="500px"
    :close-on-click-modal="false"
    :aria-label="t('product.importDialogTab.ariaLabel')"
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <div style="margin-bottom: 16px">
      <el-alert type="info" :closable="false">
        <template #title>
          <div>{{ t('product.importDialogTab.alertTemplate') }}</div>
        </template>
      </el-alert>
    </div>
    <div style="margin-bottom: 16px">
      <el-button type="primary" link @click="handleDownloadTemplate">
        <el-icon><Download /></el-icon>
        {{ t('product.importDialogTab.buttonDownloadTemplate') }}
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
      <!-- eslint-disable-next-line vue/no-v-html -- 安全：i18n 翻译文本，无 XSS 风险 -->
      <div class="el-upload__text" v-html="t('product.importDialogTab.uploadText')"></div>
      <template #tip>
        <div class="el-upload__tip">{{ t('product.importDialogTab.uploadTip') }}</div>
      </template>
    </el-upload>
    <template #footer>
      <el-button @click="emit('update:modelValue', false)">{{
        t('product.importDialogTab.buttonCancel')
      }}</el-button>
      <el-button type="primary" :loading="importLoading" @click="handleSubmit">{{
        t('product.importDialogTab.buttonConfirmImport')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { Download, Upload } from '@element-plus/icons-vue';
import { getProductImportTemplate, importProducts } from '@/api/product';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

interface Props {
  modelValue: boolean;
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void;
  (e: 'submitted'): void;
}

defineProps<Props>();
const emit = defineEmits<Emits>();

const uploadRef = ref();
const importFile = ref<File | null>(null);
const importLoading = ref(false);

const handleFileChange = (file: { raw?: File }) => {
  if (file.raw) {
    importFile.value = file.raw;
  }
};

const handleDownloadTemplate = async () => {
  try {
    await getProductImportTemplate();
    ElMessage.success(t('product.importDialogTab.messageTemplateDownloadSuccess'));
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('product.importDialogTab.messageTemplateDownloadFailed'));
  }
};

const handleSubmit = async () => {
  if (!importFile.value) {
    ElMessage.warning(t('product.importDialogTab.messageSelectFile'));
    return;
  }
  importLoading.value = true;
  try {
    const res = await importProducts(importFile.value);
    const data = res.data as { success?: number; failed?: number } | undefined;
    ElMessage.success(
      t('product.importDialogTab.messageImportSuccess', {
        success: data?.success || 0,
        failed: data?.failed || 0,
      })
    );
    importFile.value = null;
    emit('update:modelValue', false);
    emit('submitted');
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('product.importDialogTab.messageImportFailed'));
    logger.error(t('product.importDialogTab.messageImportFailed'), err.message);
  } finally {
    importLoading.value = false;
  }
};
</script>
