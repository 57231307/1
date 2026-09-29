<!--
  PurchaseReturnDetail.vue - 采购退货详情对话框
  任务编号: P14 批 2 I-3 第 2 批（拆分原 purchase-return/index.vue）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('purchaseReturn.detail.title')"
    width="900px"
    :aria-label="t('purchaseReturn.detail.aria.dialog')"
    @update:model-value="onVisibleChange"
  >
    <el-descriptions :column="2" border>
      <el-descriptions-item :label="t('purchaseReturn.detail.label.returnNo')">{{
        detailData.return_no
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseReturn.detail.label.purchaseOrderNo')">{{
        detailData.purchase_order_no
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseReturn.detail.label.supplier')">{{
        detailData.supplier_name
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseReturn.detail.label.returnDate')">{{
        detailData.return_date
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseReturn.detail.label.returnAmount')">
        <span class="amount">¥{{ detailData.total_amount }}</span>
      </el-descriptions-item>
      <el-descriptions-item :label="t('purchaseReturn.detail.label.status')">
        <el-tag :type="getStatusType(detailData.return_status)">
          {{ getStatusText(detailData.return_status) }}
        </el-tag>
      </el-descriptions-item>
      <el-descriptions-item :label="t('purchaseReturn.detail.label.reason')" :span="2">
        {{ detailData.reason_detail }}
      </el-descriptions-item>
      <el-descriptions-item :label="t('purchaseReturn.detail.label.remarks')" :span="2">
        {{ detailData.notes }}
      </el-descriptions-item>
    </el-descriptions>

    <el-divider content-position="left">{{ t('purchaseReturn.detail.itemsTitle') }}</el-divider>
    <el-table
      v-loading="itemsLoading"
      :data="serverItems"
      border
      :aria-label="t('purchaseReturn.detail.aria.itemsTable')"
    >
      <el-table-column
        prop="material_name"
        :label="t('purchaseReturn.detail.column.productName')"
        min-width="150"
      />
      <el-table-column
        prop="quantity_returned"
        :label="t('purchaseReturn.detail.column.quantity')"
        width="100"
      />
      <el-table-column
        prop="unit_price"
        :label="t('purchaseReturn.detail.column.unitPrice')"
        width="100"
      />
      <el-table-column
        prop="total_amount"
        :label="t('purchaseReturn.detail.column.amount')"
        width="120"
      />
      <el-table-column
        prop="notes"
        :label="t('purchaseReturn.detail.column.reason')"
        min-width="150"
      />
    </el-table>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { logger } from '@/utils/logger';
import {
  getPurchaseReturnItemList,
  type PurchaseReturn,
  type PurchaseReturnItem,
} from '@/api/purchase-return';
import { getStatusType, getStatusText } from '../composables/prRtnFmts';

const { t } = useI18n({ useScope: 'global' });

const props = defineProps<{
  // 对话框可见性
  visible: boolean;
  // 表头详情数据（get_purchase_return 仅返回表头，不含 items）
  detailData: PurchaseReturn;
}>();

// 定义事件
const emit = defineEmits<{
  // 关闭
  (e: 'update:visible', value: boolean): void;
}>();

// 明细：表头响应不含 items，对话框打开时从 /purchase/returns/{id}/items 异步回源
const serverItems = ref<PurchaseReturnItem[]>([]);
const itemsLoading = ref(false);

watch(
  () => props.visible,
  async val => {
    if (val && props.detailData?.id) {
      itemsLoading.value = true;
      try {
        const res = await getPurchaseReturnItemList(props.detailData.id);
        serverItems.value = res.data;
      } catch (error) {
        logger.error('[purchase-return] 详情明细加载失败', error);
        serverItems.value = [];
      } finally {
        itemsLoading.value = false;
      }
    } else if (!val) {
      serverItems.value = [];
    }
  }
);

/** 关闭对话框 */
const onVisibleChange = (v: boolean) => {
  emit('update:visible', v);
};
</script>

<style scoped>
.amount {
  font-weight: 600;
  color: #f56c6c;
}
</style>
