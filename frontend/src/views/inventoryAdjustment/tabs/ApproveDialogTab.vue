<!--
  ApproveDialogTab.vue - 库存调整审批对话框
  来源：原 inventoryAdjustment/index.vue 中 审批弹窗
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    :title="t('inventoryAdjustment.approveDialogTab.title')"
    width="500px"
    :aria-label="t('inventoryAdjustment.approveDialogTab.ariaLabel')"
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-form
      ref="formRef"
      :model="formData"
      label-width="80px"
      :aria-label="t('inventoryAdjustment.approveDialogTab.ariaLabelForm')"
    >
      <el-form-item :label="t('inventoryAdjustment.approveDialogTab.labelAdjustNo')">
        <el-input :model-value="currentRow?.adjust_no" disabled />
      </el-form-item>
      <el-form-item :label="t('inventoryAdjustment.approveDialogTab.labelAdjustDate')">
        <el-input :model-value="currentRow?.adjust_date" disabled />
      </el-form-item>
      <el-form-item :label="t('inventoryAdjustment.approveDialogTab.labelWarehouse')">
        <el-input :model-value="currentRow?.warehouse_name" disabled />
      </el-form-item>
      <el-form-item :label="t('inventoryAdjustment.approveDialogTab.labelReason')">
        <el-input :model-value="currentRow?.reason" disabled type="textarea" :rows="2" />
      </el-form-item>
      <el-form-item
        :label="t('inventoryAdjustment.approveDialogTab.labelApprovalComment')"
        prop="approval_comment"
      >
        <el-input
          v-model="formData.approval_comment"
          type="textarea"
          :rows="3"
          :placeholder="t('inventoryAdjustment.approveDialogTab.placeholderApprovalComment')"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:modelValue', false)">{{
        t('inventoryAdjustment.approveDialogTab.buttonCancel')
      }}</el-button>
      <el-button type="warning" :loading="submitLoading" @click="handleReject">{{
        t('inventoryAdjustment.approveDialogTab.buttonReject')
      }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handlePass">{{
        t('inventoryAdjustment.approveDialogTab.buttonPass')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import type { FormInstance } from 'element-plus';
import {
  approveInventoryAdjustment,
  rejectInventoryAdjustment,
  type InventoryAdjustmentEntity,
} from '@/api/inventoryAdjustment';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

interface Props {
  modelValue: boolean;
  currentRow: InventoryAdjustmentEntity | null;
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void;
  (e: 'submitted'): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();

const formRef = ref<FormInstance>();
const submitLoading = ref(false);

const formData = reactive({ approval_comment: '' });

const resetForm = () => {
  formData.approval_comment = '';
};

watch(
  () => props.modelValue,
  val => {
    if (val) resetForm();
  }
);

const handlePass = async () => {
  if (!formRef.value || !props.currentRow) return;
  submitLoading.value = true;
  try {
    await approveInventoryAdjustment(props.currentRow.id as number);
    ElMessage.success(t('inventoryAdjustment.approveDialogTab.messagePassSuccess'));
    emit('update:modelValue', false);
    emit('submitted');
  } catch (error) {
    ElMessage.error(
      (error as Error).message || t('inventoryAdjustment.approveDialogTab.messageFailed')
    );
    logger.error(t('inventoryAdjustment.approveDialogTab.approveFailed'), (error as Error).message);
  } finally {
    submitLoading.value = false;
  }
};

const handleReject = async () => {
  if (!props.currentRow) return;
  try {
    await ElMessageBox.confirm(
      t('inventoryAdjustment.approveDialogTab.confirmRejectContent'),
      t('inventoryAdjustment.approveDialogTab.confirmRejectTitle'),
      { type: 'warning' }
    );
    submitLoading.value = true;
    await rejectInventoryAdjustment(props.currentRow.id as number);
    ElMessage.success(t('inventoryAdjustment.approveDialogTab.messageRejectSuccess'));
    emit('update:modelValue', false);
    emit('submitted');
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error(
        (error as Error).message || t('inventoryAdjustment.approveDialogTab.messageFailed')
      );
    }
  } finally {
    submitLoading.value = false;
  }
};
</script>
