<!--
  AmountDialogTab.vue - 客户信用占用/释放对话框
  来源：原 customerCredit/index.vue 中 占用/释放额度对话框
  拆分日期：2026-06-15 B3-3
-->
<template>
  <el-dialog
    v-model="visible"
    :title="
      operationType === 'occupy'
        ? t('customerCredit.amount.title.occupy')
        : t('customerCredit.amount.title.release')
    "
    width="500px"
    :aria-label="t('customerCredit.amount.ariaLabel')"
  >
    <el-form
      ref="formRef"
      :model="form"
      :rules="rules"
      label-width="120px"
      :aria-label="t('customerCredit.amount.formAriaLabel')"
    >
      <el-form-item :label="t('customerCredit.amount.label.amount')" prop="amount">
        <el-input-number v-model="form.amount" :min="0" style="width: 100%" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visible = false">{{ t('customerCredit.amount.button.cancel') }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
        t('customerCredit.amount.button.confirm')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import { occupyCredit, releaseCredit } from '@/api/customer-credit';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

interface Props {
  modelValue: boolean;
  customerId: number | null;
  operationType: 'occupy' | 'release';
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
  amount: 0,
});

const rules: FormRules = {
  amount: [
    {
      required: true,
      message: t('customerCredit.amount.validation.amountRequired'),
      trigger: 'blur',
    },
    {
      // 后端 occupy/release 共用 CreditAmountRequest，绑定 validate_amount_range 强制 amount>0
      // （backend/src/handlers/customer_credit_handler.rs:82-84 + utils/validator.rs:14）；
      // async-validator 的 required 对数字 0 视为有效值，拦不住表单默认的 0。
      validator: (_rule, value, callback) => {
        if (value !== undefined && value !== null && Number(value) <= 0) {
          callback(new Error(t('customerCredit.amount.validation.amountMustPositive')));
        } else {
          callback();
        }
      },
      trigger: 'blur',
    },
  ],
};

watch(
  () => props.modelValue,
  val => {
    visible.value = val;
    if (val) {
      form.amount = 0;
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
    if (props.operationType === 'occupy') {
      await occupyCredit(props.customerId, { amount: form.amount });
    } else {
      // 后端 release_credit 只认 amount（见 api/customer-credit.ts 注释），
      // 这里必须提交表单真实金额，不能再传占位 0。
      await releaseCredit(props.customerId, { amount: form.amount });
    }
    ElMessage.success(t('customerCredit.amount.message.success'));
    visible.value = false;
    emit('submitted');
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('customerCredit.amount.message.failed'));
    logger.warn(t('customerCredit.amount.log.failed'), err.message);
  } finally {
    submitLoading.value = false;
  }
};
</script>
