<!--
  AdjustDialogTab.vue - 客户信用额度调整对话框
  来源：原 customerCredit/index.vue 中 调整额度对话框
  拆分日期：2026-06-15 B3-3
-->
<template>
  <el-dialog
    v-model="visible"
    :title="t('customerCredit.adjust.title')"
    width="500px"
    :aria-label="t('customerCredit.adjust.ariaLabel')"
  >
    <el-form
      ref="formRef"
      :model="form"
      :rules="rules"
      label-width="120px"
      :aria-label="t('customerCredit.adjust.formAriaLabel')"
    >
      <el-form-item :label="t('customerCredit.adjust.label.adjustmentType')" prop="adjustmentType">
        <el-radio-group v-model="form.adjustmentType">
          <el-radio value="increase">{{ t('customerCredit.adjust.option.increase') }}</el-radio>
          <el-radio value="decrease">{{ t('customerCredit.adjust.option.decrease') }}</el-radio>
        </el-radio-group>
      </el-form-item>
      <el-form-item :label="t('customerCredit.adjust.label.amount')" prop="amount">
        <el-input-number v-model="form.amount" :min="0" style="width: 100%" />
      </el-form-item>
      <el-form-item :label="t('customerCredit.adjust.label.reason')" prop="reason">
        <el-input v-model="form.reason" type="textarea" :rows="3" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visible = false">{{ t('customerCredit.adjust.button.cancel') }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
        t('customerCredit.adjust.button.confirm')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import { adjustCreditLimit } from '@/api/customer-credit';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

interface Props {
  modelValue: boolean;
  customerId: number | null;
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void;
  (e: 'submitted'): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();

const visible = ref(props.modelValue);
const submitLoading = ref(false);
const formRef = ref<FormInstance>();

const form = reactive({
  adjustmentType: 'increase' as 'increase' | 'decrease',
  amount: 0,
  reason: '',
});

const rules: FormRules = {
  adjustmentType: [
    {
      required: true,
      message: t('customerCredit.adjust.validation.adjustmentTypeRequired'),
      trigger: 'change',
    },
  ],
  amount: [
    {
      required: true,
      message: t('customerCredit.adjust.validation.amountRequired'),
      trigger: 'blur',
    },
  ],
  reason: [
    {
      required: true,
      message: t('customerCredit.adjust.validation.reasonRequired'),
      trigger: 'blur',
    },
  ],
};

watch(
  () => props.modelValue,
  val => {
    visible.value = val;
    if (val) {
      form.adjustmentType = 'increase';
      form.amount = 0;
      form.reason = '';
    }
  }
);

watch(visible, val => {
  emit('update:modelValue', val);
});

const handleSubmit = async () => {
  if (!formRef.value || !props.customerId) return;
  try {
    await formRef.value.validate();
    submitLoading.value = true;
    await adjustCreditLimit(props.customerId, {
      type: form.adjustmentType,
      amount: form.amount,
      reason: form.reason,
    });
    ElMessage.success(t('customerCredit.adjust.message.success'));
    visible.value = false;
    emit('submitted');
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('customerCredit.adjust.message.failed'));
    logger.warn(t('customerCredit.adjust.log.failed'), err.message);
  } finally {
    submitLoading.value = false;
  }
};
</script>
