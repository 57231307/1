<!--
  SchedulingMachineAdjust.vue - 排产主页调整对话框
  任务编号: P14 批 2 I-3 第 2 批（拆分原 scheduling/index.vue）
  P9-3 批次 F 重构：移除 vue/no-mutating-props 抑制，改用本地 ref 镜像 + watch 防循环
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('scheduling.machineAdjust.title')"
    width="450px"
    :aria-label="t('scheduling.machineAdjust.ariaLabel.dialog')"
    @update:model-value="onVisibleChange"
  >
    <el-form
      :model="localForm"
      label-width="100px"
      :aria-label="t('scheduling.machineAdjust.ariaLabel.form')"
    >
      <el-form-item :label="t('scheduling.machineAdjust.form.orderNo')">
        <span>{{ adjustTask?.order_no }}</span>
      </el-form-item>
      <el-form-item :label="t('scheduling.machineAdjust.form.startTime')">
        <el-date-picker
          v-model="localForm.start_time"
          type="datetime"
          :placeholder="t('scheduling.machineAdjust.placeholder.startTime')"
          style="width: 100%"
        />
      </el-form-item>
      <el-form-item :label="t('scheduling.machineAdjust.form.endTime')">
        <el-date-picker
          v-model="localForm.end_time"
          type="datetime"
          :placeholder="t('scheduling.machineAdjust.placeholder.endTime')"
          style="width: 100%"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="onCancel">{{ t('scheduling.machineAdjust.button.cancel') }}</el-button>
      <el-button type="primary" :loading="adjusting" @click="emit('confirm')">{{
        t('scheduling.machineAdjust.button.confirm')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue';
import { useI18n } from 'vue-i18n';

const { t } = useI18n({ useScope: 'global' });

// 调整任务类型（所有字段可选，兼容空对象）
interface AdjustTask {
  order_no?: string;
  [key: string]: unknown;
}

// 调整表单类型
interface AdjustForm {
  start_time: string;
  end_time: string;
}

// 排产调整对话框属性
const props = defineProps<{
  // 对话框可见性
  visible: boolean;
  // 调整任务
  adjustTask: AdjustTask | null;
  // 调整表单（由父组件管理，子组件通过 emit('update:form') 回写）
  adjustForm: AdjustForm;
  // 调整中
  adjusting: boolean;
}>();

// 定义事件
const emit = defineEmits<{
  // 关闭
  (e: 'update:visible', value: boolean): void;
  // 确认
  (e: 'confirm'): void;
  // 整体回写表单
  (e: 'update:form', form: AdjustForm): void;
}>();

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localForm = ref<AdjustForm>({ ...props.adjustForm });

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false;

// 外部 prop 变化时同步到 local
watch(
  () => props.adjustForm,
  newForm => {
    if (syncing) return;
    syncing = true;
    localForm.value = { ...newForm };
    nextTick(() => {
      syncing = false;
    });
  },
  { deep: true }
);

// 本地变化时通知父组件
watch(
  localForm,
  newForm => {
    if (syncing) return;
    syncing = true;
    emit('update:form', { ...newForm });
    nextTick(() => {
      syncing = false;
    });
  },
  { deep: true }
);

/** 关闭对话框 */
const onVisibleChange = (v: boolean) => {
  emit('update:visible', v);
};

/** 取消 */
const onCancel = () => {
  emit('update:visible', false);
};
</script>
