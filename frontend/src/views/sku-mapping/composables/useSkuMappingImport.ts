/**
 * useSkuMappingImport - SKU 对照表批量导入 composable
 */
import { ref } from 'vue';
import { msg } from '@/utils/message';
import {
  importSkuMappings,
  type SkuMappingImportResult,
  type SkuMappingImportError,
} from '@/api/sku-mapping';

export function useSkuMappingImport(onSuccess: () => void) {
  const importDialogVisible = ref(false);
  const importing = ref(false);
  const importResult = ref<SkuMappingImportResult | null>(null);
  const importErrors = ref<SkuMappingImportError[]>([]);

  const openImportDialog = () => {
    importResult.value = null;
    importErrors.value = [];
    importDialogVisible.value = true;
  };

  const handleFileChange = async (file: File) => {
    importing.value = true;
    importResult.value = null;
    importErrors.value = [];
    try {
      const res = await importSkuMappings(file);
      importResult.value = res.data;
      importErrors.value = res.data.errors;
      if (res.data.error_count === 0) {
        msg.importOk();
      } else {
        msg.warning('importPartialFail');
      }
      onSuccess();
    } catch (error: unknown) {
      msg.importFail();
      console.error('[sku-mapping] import error:', error);
    } finally {
      importing.value = false;
    }
  };

  return {
    importDialogVisible,
    importing,
    importResult,
    importErrors,
    openImportDialog,
    handleFileChange,
  };
}
