<!--
  PurchaseReturnApproval.vue - 采购退货审批动作选择对话框
  只承载「通过 / 拒绝 / 取消」三个动作出口；理由采集统一走父级流程
  （usePrRtnProc → useActionPrompts 采集器：通过选填、拒绝必填）。
  原「审批意见」输入框采而不发（后端 approve 只认通过理由），属假采集，已移除。
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('purchaseReturn.approval.title')"
    width="500px"
    :aria-label="t('purchaseReturn.approval.aria.dialog')"
    @update:model-value="onVisibleChange"
  >
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
import { useI18n } from 'vue-i18n';

const { t } = useI18n({ useScope: 'global' });

defineProps<{
  // 对话框可见性
  visible: boolean;
}>();

// 定义事件
const emit = defineEmits<{
  // 关闭
  (e: 'update:visible', value: boolean): void;
  // 通过（父级经统一采集器采选填理由后提交）
  (e: 'approve-confirm'): void;
  // 拒绝（父级经统一采集器采必填理由后提交）
  (e: 'reject'): void;
}>();

/** 关闭对话框 */
const onVisibleChange = (v: boolean) => {
  emit('update:visible', v);
};

/** 取消 */
const onCancel = () => {
  emit('update:visible', false);
};
</script>
