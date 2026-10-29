<!--
  SkuMappingImportDialog - SKU 对照表批量导入对话框
-->
<template>
  <el-dialog
    :model-value="proc.importDialogVisible.value"
    :title="t('skuMapping.importTitle')"
    width="560px"
    destroy-on-close
    @update:model-value="(v: boolean) => (proc.importDialogVisible.value = v)"
  >
    <div class="import-template-bar">
      <el-button type="primary" link :loading="templateLoading" @click="handleDownloadTemplate">
        <el-icon><download /></el-icon>
        {{ t('skuMapping.buttonDownloadTemplate') }}
      </el-button>
    </div>

    <el-alert
      v-if="templateMeta"
      class="import-columns-tip"
      :title="t('skuMapping.importColumnsTip')"
      type="info"
      :closable="false"
      show-icon
    />
    <el-table
      v-if="templateMeta"
      :data="templateMeta.columns"
      size="small"
      max-height="200"
      class="import-columns"
    >
      <el-table-column prop="title" :label="t('skuMapping.colName')" width="180" />
      <el-table-column :label="t('skuMapping.colRequired')" width="90">
        <template #default="{ row }">
          {{ row.required ? t('skuMapping.colRequiredYes') : t('skuMapping.colRequiredNo') }}
        </template>
      </el-table-column>
      <el-table-column prop="example" :label="t('skuMapping.colExample')" />
    </el-table>

    <el-upload
      drag
      action="#"
      :auto-upload="false"
      accept=".xlsx,.xls,.csv"
      :show-file-list="false"
      :on-change="onFileChange"
    >
      <el-icon class="el-icon--upload"><upload-filled /></el-icon>
      <div class="el-upload__text">{{ t('skuMapping.uploadHint') }}</div>
    </el-upload>

    <div v-if="proc.importing.value" class="import-loading">
      <el-progress :percentage="100" :indeterminate="true" />
      <span>{{ t('skuMapping.importing') }}</span>
    </div>

    <div v-if="proc.importResult.value" class="import-result">
      <el-alert
        :title="
          t('skuMapping.importSummary', {
            success: proc.importResult.value.success_count,
            fail: proc.importResult.value.error_count,
          })
        "
        :type="proc.importResult.value.error_count > 0 ? 'warning' : 'success'"
        :closable="false"
        show-icon
      />
    </div>

    <el-table
      v-if="proc.importErrors.value.length > 0"
      :data="proc.importErrors.value"
      max-height="200"
      class="import-errors"
    >
      <el-table-column prop="row" :label="t('skuMapping.errorRow')" width="80" />
      <el-table-column prop="message" :label="t('skuMapping.errorMessage')" />
    </el-table>

    <template #footer>
      <el-button @click="proc.importDialogVisible.value = false">{{ t('common.close') }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { inject, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { Download, UploadFilled } from '@element-plus/icons-vue';
import {
  downloadImportTemplate,
  getImportTemplateMeta,
  saveTemplateBlob,
  type ImportTemplateMeta,
} from '@/api/import-template';
import type { useSkuMappingImport } from '../composables/useSkuMappingImport';

const { t } = useI18n({ useScope: 'global' });

type ProcState = ReturnType<typeof useSkuMappingImport>;

const proc = inject<ProcState>('skuMappingImportProc')!;

// 列说明与模板文件都取自后端模板登记表，前端不另抄列清单
const templateMeta = ref<ImportTemplateMeta | null>(null);
const templateLoading = ref(false);

const onFileChange = (uploadFile: { raw?: File }) => {
  if (uploadFile.raw) {
    proc.handleFileChange(uploadFile.raw);
  }
};

const loadTemplateMeta = async () => {
  if (templateMeta.value || templateLoading.value) {
    return;
  }
  templateLoading.value = true;
  try {
    const res = await getImportTemplateMeta('sku_mapping');
    templateMeta.value = res.data;
  } catch (e: unknown) {
    const err = e as Error;
    ElMessage.error(err.message || t('skuMapping.messageTemplateDownloadFailed'));
  } finally {
    templateLoading.value = false;
  }
};

watch(
  () => proc.importDialogVisible.value,
  visible => {
    if (visible) {
      void loadTemplateMeta();
    }
  }
);

const handleDownloadTemplate = async () => {
  templateLoading.value = true;
  try {
    const blob = await downloadImportTemplate('sku_mapping');
    saveTemplateBlob(blob, 'sku_mapping_template.xlsx');
    ElMessage.success(t('skuMapping.messageTemplateDownloaded'));
  } catch (e: unknown) {
    const err = e as Error;
    ElMessage.error(err.message || t('skuMapping.messageTemplateDownloadFailed'));
  } finally {
    templateLoading.value = false;
  }
};
</script>

<style scoped>
.import-template-bar {
  display: flex;
  margin-bottom: 12px;
}

.import-columns-tip {
  margin-bottom: 8px;
}

.import-columns {
  margin-bottom: 16px;
}

.import-loading {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 16px;
}

.import-result {
  margin-top: 16px;
}

.import-errors {
  margin-top: 16px;
}
</style>
