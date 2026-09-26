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
import { useI18n } from 'vue-i18n';
import { UploadFilled } from '@element-plus/icons-vue';
import type { useSkuMappingImport } from '../composables/useSkuMappingImport';

const { t } = useI18n({ useScope: 'global' });

type ProcState = ReturnType<typeof useSkuMappingImport>;

const props = defineProps<{
  proc: ProcState;
}>();

const onFileChange = (uploadFile: { raw?: File }) => {
  if (uploadFile.raw) {
    props.proc.handleFileChange(uploadFile.raw);
  }
};
</script>

<style scoped>
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
