<!--
  CountDetailDialogTab.vue - 盘点单详情对话框
  来源：原 inventoryCount/index.vue 中 盘点单详情弹窗
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    :title="t('inventoryCount.detailDialogTab.titleDetail')"
    width="800px"
    :aria-label="t('inventoryCount.detailDialogTab.ariaLabelDetail')"
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-descriptions v-if="currentRow" :column="2" border>
      <el-descriptions-item :label="t('inventoryCount.detailDialogTab.labelCountNo')">{{
        currentRow.count_no
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('inventoryCount.detailDialogTab.labelCountDate')">{{
        currentRow.count_date
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('inventoryCount.detailDialogTab.labelWarehouse')">{{
        currentRow.warehouse_name
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('inventoryCount.detailDialogTab.labelStatus')">
        <el-tag :type="currentRow.status === 'completed' ? 'success' : 'warning'" size="small">
          {{ getStatusLabel(currentRow.status) }}
        </el-tag>
      </el-descriptions-item>
      <el-descriptions-item :label="t('inventoryCount.detailDialogTab.labelCreatedBy')">{{
        currentRow.created_by_name
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('inventoryCount.detailDialogTab.labelCreatedAt')">{{
        currentRow.created_at
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('inventoryCount.detailDialogTab.labelCompletedAt')" :span="2">
        {{ currentRow.completed_at || '-' }}
      </el-descriptions-item>
    </el-descriptions>
    <template #footer>
      <el-button @click="emit('update:modelValue', false)">{{
        t('inventoryCount.detailDialogTab.buttonClose')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { InventoryCountEntity } from '@/api/inventoryCount';

const { t } = useI18n({ useScope: 'global' });

interface Props {
  modelValue: boolean;
  currentRow: InventoryCountEntity | null;
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void;
}

defineProps<Props>();
const emit = defineEmits<Emits>();

/** 状态标签函数化：优先 i18n，未知状态回退到原始 status 字符串 */
const getStatusLabel = (status: string) => {
  const key = `inventoryCount.detailDialogTab.statusLabel.${status}`;
  const translated = t(key);
  return translated === key ? status : translated;
};
</script>
