<!--
  PurchaseReturnApproval.vue - 采购退货审批对话框
  任务编号: P14 批 2 I-3 第 2 批（拆分原 purchase-return/index.vue）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('purchaseReturn.approval.title')"
    width="500px"
    :aria-label="t('purchaseReturn.approval.aria.dialog')"
    @update:model-value="onVisibleChange"
  >
    <el-form
      :model="localApproveForm"
      label-width="80px"
      :aria-label="t('purchaseReturn.approval.aria.form')"
    >
      <el-form-item :label="t('purchaseReturn.approval.label.opinion')">
        <el-input
          v-model="localApproveForm.remark"
          type="textarea"
          :rows="3"
          :placeholder="t('purchaseReturn.approval.placeholder.opinion')"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="onCancel">{{ t('purchaseReturn.approval.button.cancel') }}</el-button>
      <el-button type="danger" @click="emit('reject')">{{
        t('purchaseReturn.approval.button.reject')
      }}</el-button>
      <el-button type="success" @click="emit('approve-confirm')">{{
        t('purchaseReturn.approval.button.approve')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue';
import { useI18n } from 'vue-i18n';

const { t } = useI18n({ useScope: 'global' });

// 审批表单类型
interface ApproveForm {
  id: number;
  remark: string;
}

const props = defineProps<{
  // 对话框可见性
  visible: boolean;
  // 审批表单（由父组件管理，子组件通过 emit('update:approveForm') 回写）
  approveForm: ApproveForm;
}>();

// 定义事件
const emit = defineEmits<{
  // 关闭
  (e: 'update:visible', value: boolean): void;
  // 通过
  (e: 'approve-confirm'): void;
  // 拒绝
  (e: 'reject'): void;
  // 整体回写审批表单（父组件监听此事件并 Object.assign 到自己的 approveForm）
  (e: 'update:approveForm', approveForm: ApproveForm): void;
}>();

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localApproveForm = ref<ApproveForm>({ ...props.approveForm });

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false;

// 外部 prop 变化时同步到 local（如父组件打开审批对话框时填充）
watch(
  () => props.approveForm,
  newForm => {
    if (syncing) return;
    syncing = true;
    localApproveForm.value = { ...newForm };
    nextTick(() => {
      syncing = false;
    });
  },
  { deep: true }
);

// 本地变化时通知父组件（用户输入）
watch(
  localApproveForm,
  newForm => {
    if (syncing) return;
    syncing = true;
    emit('update:approveForm', { ...newForm });
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
