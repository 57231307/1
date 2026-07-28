<!--
  LogisticsStatDialog.vue - 物流管理更新状态对话框
  拆分自 logistics/index.vue（P14 批 2 I-3 第 4 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('logistics.statusDialog.title')"
    width="400px"
    :aria-label="t('logistics.statusDialog.aria.dialog')"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form
      :model="localForm"
      label-width="80px"
      :aria-label="t('logistics.statusDialog.aria.form')"
    >
      <el-form-item :label="t('logistics.statusDialog.label.currentStatus')">
        <el-tag :type="getStatusTypeFmt(localForm.currentStatus)">
          {{ statusTextFmt(localForm.currentStatus) }}
        </el-tag>
      </el-form-item>
      <el-form-item :label="t('logistics.statusDialog.label.newStatus')">
        <el-select
          v-model="localForm.newStatus"
          :placeholder="t('logistics.statusDialog.placeholder.newStatus')"
        >
          <el-option
            v-for="status in statuses"
            :key="status.value"
            :label="status.label"
            :value="status.value"
          />
        </el-select>
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">{{
        t('logistics.statusDialog.button.cancel')
      }}</el-button>
      <el-button type="primary" @click="emit('submit')">{{
        t('logistics.statusDialog.button.confirm')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue';
import { useI18n } from 'vue-i18n';
import { getStatusType } from '../composables/lgsFmts';
import type { WaybillStatus } from '@/api/logistics';
import type { LgsStatusForm } from '../composables/useLgsProc';

const { t } = useI18n({ useScope: 'global' });

// 状态选项类型
interface StatusOption {
  label: string;
  value: WaybillStatus;
}

/**
 * 物流更新状态对话框组件
 */
const props = defineProps<{
  // 对话框可见性
  visible: boolean;
  // 表单数据（由父组件管理，子组件通过 emit 回写）
  form: LgsStatusForm;
  // 可选状态列表
  statuses: StatusOption[];
}>();

const emit = defineEmits<{
  (e: 'update:visible', v: boolean): void;
  (e: 'submit'): void;
  // 整体回写表单（父组件监听此事件并 Object.assign 到自己的 form）
  (e: 'update:form', form: LgsStatusForm): void;
}>();

// 透传格式化函数
const getStatusTypeFmt = getStatusType;

/** 状态文本：优先 i18n，未知状态回退到原始 status 字符串 */
const statusTextFmt = (status: string): string => {
  const key = `logistics.common.status.${status}`;
  const translated = t(key);
  return translated === key ? status : translated;
};

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localForm = ref<LgsStatusForm>({ ...props.form });

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false;

// 外部 prop 变化时同步到 local（如父组件打开对话框时填充）
watch(
  () => props.form,
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

// 本地变化时通知父组件（用户输入）
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
</script>
