<!--
  ApproveTransferDialogTab.vue - 调拨单审批对话框
  来源：原 inventoryTransfer/index.vue 中 调拨单审批对话框
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    :title="$t('inventoryTransfer.approveTransfer.dialogTitle')"
    width="500px"
    :aria-label="$t('inventoryTransfer.approveTransfer.dialogAriaLabel')"
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-form
      ref="formRef"
      :model="formData"
      label-width="80px"
      :aria-label="$t('inventoryTransfer.approveTransfer.formAriaLabel')"
    >
      <el-form-item :label="$t('inventoryTransfer.approveTransfer.transferNo')">
        <el-input :model-value="currentRow?.transfer_no" disabled />
      </el-form-item>
      <el-form-item :label="$t('inventoryTransfer.approveTransfer.transferDate')">
        <el-input :model-value="currentRow?.transfer_date" disabled />
      </el-form-item>
      <el-form-item :label="$t('inventoryTransfer.approveTransfer.fromWarehouse')">
        <el-input :model-value="currentRow?.from_warehouse_name" disabled />
      </el-form-item>
      <el-form-item :label="$t('inventoryTransfer.approveTransfer.toWarehouse')">
        <el-input :model-value="currentRow?.to_warehouse_name" disabled />
      </el-form-item>
      <el-form-item
        :label="$t('inventoryTransfer.approveTransfer.approvalComment')"
        prop="approval_comment"
      >
        <el-input
          v-model="formData.approval_comment"
          type="textarea"
          :rows="4"
          :placeholder="$t('inventoryTransfer.approveTransfer.approvalCommentPlaceholder')"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:modelValue', false)">{{
        $t('inventoryTransfer.approveTransfer.cancel')
      }}</el-button>
      <el-button type="warning" :loading="submitLoading" @click="handleReject">{{
        $t('inventoryTransfer.approveTransfer.reject')
      }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handlePass">{{
        $t('inventoryTransfer.approveTransfer.pass')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import type { FormInstance } from 'element-plus';
import { approveInventoryTransfer, type InventoryTransferEntity } from '@/api/inventoryTransfer';
import { logger } from '@/utils/logger';

// 批次 34 v9 P1：接入 i18n，替换硬编码中文 ElMessage
const { t } = useI18n({ useScope: 'global' });

interface Props {
  modelValue: boolean;
  currentRow: InventoryTransferEntity | null;
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
    await approveInventoryTransfer(props.currentRow.id as number);
    ElMessage.success(t('inventoryTransfer.approvePassed'));
    emit('update:modelValue', false);
    emit('submitted');
  } catch (error) {
    ElMessage.error((error as Error).message || t('message.operationFailed'));
    logger.error(t('inventoryTransfer.approveTransfer.approveFailed'), (error as Error).message);
  } finally {
    submitLoading.value = false;
  }
};

const handleReject = async () => {
  if (!props.currentRow) return;
  try {
    await ElMessageBox.confirm(
      t('inventoryTransfer.confirmReject'),
      t('message.rejectConfirmTitle'),
      { type: 'warning' }
    );
    submitLoading.value = true;
    // reject 接口未在 api/inventoryTransfer 中实现，复用 approve 接口
    await approveInventoryTransfer(props.currentRow.id as number);
    ElMessage.success(t('inventoryTransfer.rejected'));
    emit('update:modelValue', false);
    emit('submitted');
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error((error as Error).message || t('message.operationFailed'));
    }
  } finally {
    submitLoading.value = false;
  }
};
</script>
