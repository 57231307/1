<!--
  CompleteDyeBatchDialog.vue - 缸号完工登记对话框（任务 #168）
  完工时强制采集实际产出三值：实际落布重量(kg) / 实际落布长度(米) / 坯布投料量(kg)。
  契约对齐后端 CompleteDyeBatchRequest（handlers/dye_batch_handler.rs）：三值必填、
  为正、最多 2 位小数；缺失或非法后端 400 且状态不推进（失败提示由 api/request.ts
  拦截器统一外显一次，本组件只记日志、保留对话框供用户修正）。
  使用方：views/dye-batch/index.vue、views/fabric/tabs/DyeTab.vue
-->
<template>
  <el-dialog
    :model-value="modelValue"
    :title="t('dyeBatch.complete.title')"
    width="520px"
    :close-on-click-modal="false"
    :aria-label="t('dyeBatch.complete.ariaForm')"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <el-form
      ref="formRef"
      :model="formData"
      :rules="formRules"
      label-width="140px"
      :aria-label="t('dyeBatch.complete.ariaForm')"
    >
      <el-form-item :label="t('dyeBatch.complete.batchLabel')">
        <span>{{ batch?.batch_no ?? '-' }}</span>
      </el-form-item>
      <el-form-item :label="t('dyeBatch.complete.labelActualOutputKg')" prop="actual_output_kg">
        <el-input-number
          v-model="formData.actual_output_kg"
          :precision="2"
          :min="0"
          :controls="false"
          style="width: 100%"
        />
      </el-form-item>
      <el-form-item :label="t('dyeBatch.complete.labelActualOutputM')" prop="actual_output_m">
        <el-input-number
          v-model="formData.actual_output_m"
          :precision="2"
          :min="0"
          :controls="false"
          style="width: 100%"
        />
      </el-form-item>
      <el-form-item :label="t('dyeBatch.complete.labelGreigeInputKg')" prop="greige_input_kg">
        <el-input-number
          v-model="formData.greige_input_kg"
          :precision="2"
          :min="0"
          :controls="false"
          style="width: 100%"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button :disabled="submitting" @click="emit('update:modelValue', false)">{{
        t('dyeBatch.complete.buttonCancel')
      }}</el-button>
      <el-button type="primary" :loading="submitting" @click="handleSubmit">{{
        t('dyeBatch.complete.buttonConfirm')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { reactive, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, type FormInstance, type FormRules } from 'element-plus';
import { completeDyeBatch, type CompleteDyeBatchPayload, type DyeBatch } from '@/api/dye-batch';
import { logger } from '@/utils/logger';

const props = defineProps<{
  modelValue: boolean;
  batch: DyeBatch | null;
}>();

const emit = defineEmits<{
  (e: 'update:modelValue', v: boolean): void;
  (e: 'success', batch: DyeBatch): void;
}>();

const { t } = useI18n({ useScope: 'global' });

const formRef = ref<FormInstance>();
const submitting = ref(false);

// 三值必填：undefined（输入清空）与 <=0 都在提交前由前端规则拒绝；
// 后端 validate_amount_range 为第二道闸（正数/≤10亿/2位小数），文案外显 400。
const formData = reactive({
  actual_output_kg: undefined as number | undefined,
  actual_output_m: undefined as number | undefined,
  greige_input_kg: undefined as number | undefined,
});

const requiredPositive = (labelKey: string) => [
  {
    validator: (_rule: unknown, value: unknown, callback: (err?: Error) => void) => {
      if (value === undefined || value === null || String(value) === '') {
        callback(new Error(t(labelKey)));
        return;
      }
      const num = Number(value);
      if (!Number.isFinite(num) || num <= 0) {
        callback(new Error(t('dyeBatch.complete.rulePositive')));
        return;
      }
      callback();
    },
    trigger: 'blur',
  },
];

const formRules: FormRules = {
  actual_output_kg: requiredPositive('dyeBatch.complete.ruleRequiredKg'),
  actual_output_m: requiredPositive('dyeBatch.complete.ruleRequiredM'),
  greige_input_kg: requiredPositive('dyeBatch.complete.ruleRequiredGreige'),
};

// 每次打开重置，避免上一缸号残值串单
watch(
  () => props.modelValue,
  visible => {
    if (visible) {
      formData.actual_output_kg = undefined;
      formData.actual_output_m = undefined;
      formData.greige_input_kg = undefined;
      formRef.value?.clearValidate();
    }
  }
);

const handleSubmit = async () => {
  if (!props.batch) return;
  try {
    await formRef.value?.validate();
  } catch {
    // 前端规则拒绝：错误文案已随表单项展示，不再重复提示、不提交
    return;
  }
  submitting.value = true;
  try {
    // 显式收窄（非类型断言掩盖）：validate 通过后仍防御性确认三值齐备
    const kg = formData.actual_output_kg;
    const meters = formData.actual_output_m;
    const greige = formData.greige_input_kg;
    if (kg === undefined || meters === undefined || greige === undefined) {
      logger.error(t('dyeBatch.complete.messageFailed'), 'incomplete form values');
      return;
    }
    const payload: CompleteDyeBatchPayload = {
      actual_output_kg: kg,
      actual_output_m: meters,
      greige_input_kg: greige,
    };
    const res = await completeDyeBatch(props.batch.id, payload);
    ElMessage.success(t('dyeBatch.complete.messageSuccess'));
    emit('update:modelValue', false);
    emit('success', res.data);
  } catch (error) {
    // 后端 400（缺值/非正数/精度超限）拒绝原因由 request.ts 拦截器外显一次；
    // 此处只记日志、保持对话框打开，状态未被推进，用户修正后可直接重提
    logger.error(t('dyeBatch.complete.messageFailed'), String(error));
  } finally {
    submitting.value = false;
  }
};
</script>
