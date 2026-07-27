<!--
  ApproveDialogTab.vue - 审批质量标准对话框
  来源：原 quality/index.vue 中 审批质量标准对话框
  拆分日期：2026-06-15 B3-4
  D05 Batch 8 Group B：接入 useI18n
-->
<template>
  <el-dialog
    :model-value="modelValue"
    :title="t('quality.approveDialogTab.title')"
    width="500px"
    :aria-label="t('quality.approveDialogTab.ariaLabel')"
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-form
      ref="formRef"
      :model="formData"
      :rules="formRules"
      label-width="80px"
      :aria-label="t('quality.approveDialogTab.formAriaLabel')"
    >
      <el-form-item :label="t('quality.approveDialogTab.standardCode')">
        <el-input :model-value="currentRow?.standard_code" disabled />
      </el-form-item>
      <el-form-item :label="t('quality.approveDialogTab.standardName')">
        <el-input :model-value="currentRow?.standard_name" disabled />
      </el-form-item>
      <el-form-item :label="t('quality.approveDialogTab.currentVersion')">
        <el-input :model-value="currentRow?.version" disabled />
      </el-form-item>
      <el-form-item :label="t('quality.approveDialogTab.approvalComment')" prop="approval_comment">
        <el-input
          v-model="formData.approval_comment"
          type="textarea"
          :rows="4"
          :placeholder="t('quality.approveDialogTab.approvalCommentPlaceholder')"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:modelValue', false)">{{
        t('quality.approveDialogTab.buttonCancel')
      }}</el-button>
      <el-button type="warning" :loading="submitLoading" @click="handleReject">{{
        t('quality.approveDialogTab.buttonReject')
      }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handlePass">{{
        t('quality.approveDialogTab.buttonPass')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import { rejectQualityStandard, type QualityStandard } from '@/api/quality';
import { logger } from '@/utils/logger';

interface Props {
  modelValue: boolean;
  currentRow: QualityStandard | null;
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void;
  (e: 'submitted', row: QualityStandard): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();

const { t } = useI18n({ useScope: 'global' });

const formRef = ref<FormInstance>();
const submitLoading = ref(false);

const formData = reactive({ approval_comment: '' });

const formRules: FormRules = {
  approval_comment: [
    {
      required: true,
      message: t('quality.approveDialogTab.validationApprovalCommentRequired'),
      trigger: 'blur',
    },
  ],
};

const resetForm = () => {
  formData.approval_comment = '';
  formRef.value?.clearValidate();
};

watch(
  () => props.modelValue,
  val => {
    if (val) resetForm();
  }
);

const handlePass = async () => {
  if (!formRef.value || !props.currentRow) return;
  await formRef.value.validate(async valid => {
    if (!valid) return;
    submitLoading.value = true;
    try {
      if (props.currentRow) {
        emit('submitted', props.currentRow);
      }
      emit('update:modelValue', false);
    } catch (error) {
      const err = error as Error;
      ElMessage.error(err.message || t('quality.approveDialogTab.messageOperationFailed'));
      logger.error(t('quality.approveDialogTab.messageApproveFailed'), err.message);
    } finally {
      submitLoading.value = false;
    }
  });
};

const handleReject = async () => {
  if (!props.currentRow) return;
  try {
    const reason = await ElMessageBox.prompt(
      t('quality.approveDialogTab.rejectPrompt'),
      t('quality.approveDialogTab.rejectTitle'),
      {
        type: 'warning',
        confirmButtonText: t('quality.approveDialogTab.rejectConfirmButton'),
        cancelButtonText: t('quality.approveDialogTab.rejectCancelButton'),
        inputPlaceholder: t('quality.approveDialogTab.rejectPlaceholder'),
        inputType: 'textarea',
      }
    );
    // 批次 157d-2 修复：接入 rejectQualityStandard API
    submitLoading.value = true;
    await rejectQualityStandard(props.currentRow.id, {
      reject_reason: reason.value || undefined,
    });
    ElMessage.success(t('quality.approveDialogTab.messageRejectSuccess'));
    emit('submitted', props.currentRow);
    emit('update:modelValue', false);
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error;
      ElMessage.error(err.message || t('quality.approveDialogTab.messageOperationFailed'));
      logger.error(t('quality.approveDialogTab.messageRejectFailed'), err.message);
    }
  } finally {
    submitLoading.value = false;
  }
};
</script>
