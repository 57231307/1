<!--
  CountFormDialogTab.vue - 盘点单编辑对话框
  来源：原 inventoryCount/index.vue 中 盘点单编辑对话框
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    :title="
      formData.id
        ? t('inventoryCount.formDialogTab.titleEdit')
        : t('inventoryCount.formDialogTab.titleCreate')
    "
    width="600px"
    :aria-label="
      mode === 'view'
        ? t('inventoryCount.formDialogTab.ariaLabelDetail')
        : formData.id
          ? t('inventoryCount.formDialogTab.ariaLabelEdit')
          : t('inventoryCount.formDialogTab.ariaLabelCreate')
    "
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-form
      ref="formRef"
      :model="formData"
      label-width="100px"
      :disabled="mode === 'view'"
      :aria-label="t('inventoryCount.formDialogTab.ariaLabelForm')"
    >
      <el-form-item :label="t('inventoryCount.formDialogTab.labelCountNo')" prop="count_no">
        <el-input v-model="formData.count_no" :disabled="!!formData.id" />
      </el-form-item>
      <el-form-item :label="t('inventoryCount.formDialogTab.labelCountDate')" prop="count_date">
        <el-date-picker
          v-model="formData.count_date"
          type="date"
          value-format="YYYY-MM-DD"
          style="width: 100%"
        />
      </el-form-item>
      <el-form-item :label="t('inventoryCount.formDialogTab.labelWarehouse')" prop="warehouse_id">
        <el-select v-model="formData.warehouse_id" style="width: 100%">
          <el-option
            v-for="wh in warehouses"
            :key="wh.id"
            :label="wh.warehouse_name"
            :value="wh.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('inventoryCount.formDialogTab.labelRemark')" prop="notes">
        <el-input
          v-model="formData.notes"
          type="textarea"
          :rows="3"
          :placeholder="t('inventoryCount.formDialogTab.placeholderRemark')"
        />
      </el-form-item>
    </el-form>
    <template v-if="mode !== 'view'" #footer>
      <el-button @click="emit('update:modelValue', false)">{{
        t('inventoryCount.formDialogTab.buttonCancel')
      }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
        t('inventoryCount.formDialogTab.buttonSave')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import type { FormInstance } from 'element-plus';
import {
  createInventoryCount,
  updateInventoryCount,
  generateInventoryCountNo,
  type InventoryCountEntity,
} from '@/api/inventory-count';
import type { Warehouse } from '@/api/warehouse';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

interface Props {
  modelValue: boolean;
  currentRow: InventoryCountEntity | null;
  warehouses: Warehouse[];
  mode: 'create' | 'edit' | 'view';
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void;
  (e: 'submitted'): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();

const formRef = ref<FormInstance>();
const submitLoading = ref(false);

const formData = reactive({
  id: 0,
  /** 单号由后端 generate_count_no 生成，建单入参不含该字段，仅展示 */
  count_no: '',
  /** 日期控件按 YYYY-MM-DD 取值，提交前转 RFC3339（后端 count_date 解析为 DateTime<Utc>） */
  count_date: new Date().toISOString().split('T')[0],
  warehouse_id: undefined as number | undefined,
  /** 备注（后端 CreateCountPayload.notes / UpdateCountPayload.notes） */
  notes: '',
});

const resetForm = () => {
  formData.id = 0;
  formData.count_no = '';
  formData.count_date = new Date().toISOString().split('T')[0];
  formData.warehouse_id = undefined;
  formData.notes = '';
};

const generateNo = async () => {
  try {
    const res = await generateInventoryCountNo();
    formData.count_no = res.data?.count_no || '';
  } catch (error) {
    logger.error(
      t('inventoryCount.formDialogTab.messageGenerateNoFailure'),
      (error as Error).message
    );
  }
};

watch(
  () => props.modelValue,
  async val => {
    if (val) {
      if (props.currentRow) {
        Object.assign(formData, props.currentRow);
      } else {
        resetForm();
        await generateNo();
      }
    }
  }
);

onMounted(() => {
  if (props.modelValue && !props.currentRow) {
    generateNo();
  }
});

const handleSubmit = async () => {
  if (!formData.warehouse_id) {
    ElMessage.warning(t('inventoryCount.formDialogTab.warehouseRequired'));
    return;
  }
  submitLoading.value = true;
  try {
    const count_date = new Date(formData.count_date).toISOString();
    if (formData.id) {
      // UpdateCountPayload 只有 count_date / notes 两字段
      await updateInventoryCount(formData.id, { count_date, notes: formData.notes });
    } else {
      await createInventoryCount({
        warehouse_id: formData.warehouse_id,
        count_date,
        notes: formData.notes,
      });
    }
    ElMessage.success(t('inventoryCount.formDialogTab.messageSuccess'));
    emit('update:modelValue', false);
    emit('submitted');
  } catch (error) {
    ElMessage.error((error as Error).message || t('inventoryCount.formDialogTab.messageFailure'));
    logger.error(t('inventoryCount.formDialogTab.messageSaveFailure'), (error as Error).message);
  } finally {
    submitLoading.value = false;
  }
};
</script>
