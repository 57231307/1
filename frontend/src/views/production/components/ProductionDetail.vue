<!--
  ProductionDetail.vue - 生产管理订单详情
  拆分自 production/index.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('production.detail.title')"
    width="800px"
    destroy-on-close
    :aria-label="t('production.detail.ariaLabel')"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <div v-if="order" class="detail-content">
      <el-descriptions :column="2" border>
        <el-descriptions-item :label="t('production.detail.labelOrderNo')">{{
          order.order_no
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('production.detail.labelProductId')">{{
          order.product_id
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('production.detail.labelProductName')">{{
          order.product_name || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('production.detail.labelWorkCenter')">{{
          order.work_center_id || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('production.detail.labelPlannedQuantity')">{{
          order.planned_quantity
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('production.detail.labelActualQuantity')">{{
          order.actual_quantity || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('production.detail.labelScheduledStart')">{{
          order.scheduled_start_date?.substring(0, 10) || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('production.detail.labelScheduledEnd')">{{
          order.scheduled_end_date?.substring(0, 10) || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('production.detail.labelActualStart')">{{
          order.actual_start_date?.substring(0, 10) || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('production.detail.labelActualEnd')">{{
          order.actual_end_date?.substring(0, 10) || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('production.detail.labelStatus')">
          <el-tag :type="statusTagType">{{ statusLabel(order.status) }}</el-tag>
        </el-descriptions-item>
        <el-descriptions-item :label="t('production.detail.labelPriority')">{{
          order.priority
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('production.detail.labelCreateTime')" :span="2">
          {{ order.created_at || '-' }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('production.detail.labelRemark')" :span="2">{{
          order.remark || '-'
        }}</el-descriptions-item>
      </el-descriptions>
    </div>
    <template #footer>
      <el-button @click="emit('update:visible', false)">{{
        t('production.detail.buttonClose')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { getProductionOrder, type ProductionOrder } from '@/api/production';
import { getStatusLabel, getStatusType } from '../composables/prdFmts';

const { t } = useI18n({ useScope: 'global' });

const props = defineProps<{
  visible: boolean;
  order: ProductionOrder | null;
}>();

const emit = defineEmits<{
  'update:visible': [v: boolean];
}>();

// 详情回源：打开时按 ID 拉取最新订单，失败保留行数据
const freshOrder = ref<ProductionOrder | null>(null);

watch(
  () => props.visible,
  async val => {
    if (val && props.order?.id) {
      try {
        const res = await getProductionOrder(props.order.id);
        if (res.data) freshOrder.value = res.data;
      } catch (e) {
        ElMessage.error((e as Error).message || '获取生产订单详情失败');
      }
    } else if (!val) {
      freshOrder.value = null;
    }
  }
);

// 模板优先展示回源后的最新数据，回源失败回退行数据
const order = computed(() => freshOrder.value ?? props.order);

const statusLabel = getStatusLabel;

// 状态配色与列表同源（prdFmts 取后端真实状态机取值）
const statusTagType = computed(() => getStatusType(props.order?.status || ''));
</script>
