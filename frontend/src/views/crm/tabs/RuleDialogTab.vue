<!--
  RuleDialogTab.vue - 客户回收规则对话框
  来源：原 crm/assignment.vue 中 新建/编辑规则对话框
  修正：表单对齐后端真实契约（name/days/is_enabled，services/crm/recycle_rule.rs），
        接入 createRecycleRule/updateRecycleRule 真实保存（原为假保存）
-->
<template>
  <el-dialog
    v-model="visible"
    :title="title"
    width="560px"
    :close-on-click-modal="false"
    :aria-label="title"
  >
    <el-form
      ref="formRef"
      :model="formData"
      :rules="formRules"
      label-width="120px"
      :aria-label="t('crmRuleDialog.ariaLabel')"
    >
      <el-form-item :label="t('crmRuleDialog.form.name')" prop="name">
        <el-input v-model="formData.name" :placeholder="t('crmRuleDialog.form.namePlaceholder')" />
      </el-form-item>
      <el-form-item :label="t('crmRuleDialog.form.days')" prop="days">
        <el-input-number
          v-model="formData.days"
          :min="1"
          :max="365"
          style="width: 100%"
          :placeholder="t('crmRuleDialog.form.daysPlaceholder')"
        />
      </el-form-item>
      <el-form-item :label="t('crmRuleDialog.form.enabled')" prop="is_enabled">
        <el-switch v-model="formData.is_enabled" />
        <span class="form-tip">{{ t('crmRuleDialog.form.daysTip') }}</span>
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visible = false">{{ t('crmRuleDialog.form.cancel') }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
        t('crmRuleDialog.form.save')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import { createRecycleRule, updateRecycleRule, type RecycleRule } from '@/api/crm-enhanced';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

interface Props {
  modelValue: boolean;
  title: string;
  rowData: RecycleRule | null;
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

const formData = reactive({
  id: undefined as number | undefined,
  name: '',
  days: 30,
  is_enabled: true,
});

const formRules: FormRules = {
  name: [{ required: true, message: t('crmRuleDialog.validation.nameRequired'), trigger: 'blur' }],
  days: [{ required: true, message: t('crmRuleDialog.validation.daysRequired'), trigger: 'blur' }],
};

watch(
  () => props.modelValue,
  val => {
    visible.value = val;
    if (val) {
      resetForm();
      if (props.rowData) {
        Object.assign(formData, {
          id: props.rowData.id,
          name: props.rowData.name || '',
          days: props.rowData.days || 30,
          is_enabled: props.rowData.is_enabled ?? true,
        });
      }
    }
  }
);

watch(visible, val => {
  emit('update:modelValue', val);
});

const resetForm = () => {
  formData.id = undefined;
  formData.name = '';
  formData.days = 30;
  formData.is_enabled = true;
  formRef.value?.clearValidate();
};

const handleSubmit = async () => {
  if (!formRef.value) return;
  await formRef.value.validate(async valid => {
    if (!valid) return;
    submitLoading.value = true;
    try {
      if (formData.id) {
        await updateRecycleRule(formData.id, {
          name: formData.name,
          days: formData.days,
          is_enabled: formData.is_enabled,
        });
      } else {
        await createRecycleRule({
          name: formData.name,
          days: formData.days,
          is_enabled: formData.is_enabled,
        });
      }
      ElMessage.success(t('crmRuleDialog.message.saveSuccess'));
      visible.value = false;
      emit('submitted');
    } catch (error) {
      const err = error as Error;
      ElMessage.error(err.message || t('crmRuleDialog.message.saveFailed'));
      logger.warn('保存回收规则失败', err.message);
    } finally {
      submitLoading.value = false;
    }
  });
};
</script>

<style scoped>
.form-tip {
  margin-left: 12px;
  color: var(--el-text-color-secondary);
  font-size: 12px;
}
</style>
