<!--
  ReturnDetailDialog.vue - 销售退货详情对话框
  任务编号: P14 批 2 I-3 第 7 批
  拆分原 sales-returns/index.vue 的详情对话框部分
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('salesReturns.detailDialog.dialogTitle')"
    width="800px"
    :aria-label="t('salesReturns.detailDialog.dialogAriaLabel')"
    @update:model-value="onClose"
  >
    <template v-if="currentReturn">
      <el-descriptions :column="2" border>
        <el-descriptions-item :label="t('salesReturns.detailDialog.labelReturnNo')">{{
          currentReturn.return_no
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('salesReturns.detailDialog.labelSalesOrderNo')">{{
          currentReturn.sales_order_no
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('salesReturns.detailDialog.labelCustomerName')">{{
          currentReturn.customer_name
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('salesReturns.detailDialog.labelReturnDate')">{{
          currentReturn.return_date
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('salesReturns.detailDialog.labelReturnAmount')">{{
          formatAmount(currentReturn.total_amount ?? 0)
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('salesReturns.detailDialog.labelStatus')">
          <el-tag :type="getStatusType(currentReturn.status ?? '')">
            {{ getStatusLabel(currentReturn.status ?? '') }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item :label="t('salesReturns.detailDialog.labelReason')" :span="2">{{
          currentReturn.reason
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('salesReturns.detailDialog.labelRemarks')" :span="2">{{
          currentReturn.remarks
        }}</el-descriptions-item>
      </el-descriptions>

      <div style="margin-top: 20px">
        <h4>{{ t('salesReturns.detailDialog.titleReturnDetails') }}</h4>
        <el-table
          v-loading="itemsLoading"
          :data="serverItems"
          border
          size="small"
          :aria-label="t('salesReturns.detailDialog.detailsTableAriaLabel')"
        >
          <el-table-column
            prop="productName"
            :label="t('salesReturns.detailDialog.columnProductName')"
          />
          <el-table-column
            prop="productCode"
            :label="t('salesReturns.detailDialog.columnProductCode')"
          />
          <el-table-column prop="quantity" :label="t('salesReturns.detailDialog.columnQuantity')" />
          <el-table-column
            prop="unit_price"
            :label="t('salesReturns.detailDialog.columnUnitPrice')"
          />
          <el-table-column
            prop="total_amount"
            :label="t('salesReturns.detailDialog.columnAmount')"
          />
          <el-table-column prop="notes" :label="t('salesReturns.detailDialog.columnReason')" />
          <el-table-column
            v-if="editable"
            :label="t('salesReturns.detailDialog.columnOperation')"
            width="180"
            fixed="right"
          >
            <template #default="{ row }">
              <el-button link type="primary" size="small" @click="handleEditItem(row)">{{
                t('salesReturns.detailDialog.buttonEditItem')
              }}</el-button>
              <el-button link type="danger" size="small" @click="handleDeleteItem(row)">{{
                t('salesReturns.detailDialog.buttonDeleteItem')
              }}</el-button>
            </template>
          </el-table-column>
        </el-table>
        <div v-if="editable" class="add-item-bar">
          <el-input-number
            v-model="newItem.productId"
            :min="1"
            :disabled="editingItemId !== null"
            style="width: 140px"
          />
          <el-input-number v-model="newItem.quantity" :min="1" style="width: 120px" />
          <el-input-number
            v-model="newItem.unitPrice"
            :min="0"
            :precision="2"
            style="width: 130px"
          />
          <el-input-number
            v-model="newItem.taxPercent"
            :min="0"
            :max="100"
            :precision="2"
            :controls="false"
            placeholder="税率%(空=取订单税率)"
            style="width: 180px"
          />
          <el-input v-model="newItem.reason" style="width: 160px" placeholder="退货原因" />
          <el-button
            v-if="editingItemId === null"
            type="primary"
            plain
            :loading="itemSaving"
            @click="handleAddItem"
          >
            {{ t('salesReturns.detailDialog.buttonAddItem') }}
          </el-button>
          <template v-else>
            <el-button type="primary" plain :loading="itemSaving" @click="handleUpdateItem">
              {{ t('salesReturns.detailDialog.buttonUpdateItem') }}
            </el-button>
            <el-button plain @click="cancelEditItem">
              {{ t('salesReturns.detailDialog.buttonCancelEdit') }}
            </el-button>
          </template>
        </div>
      </div>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import { logger } from '@/utils/logger';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { getStatusType, formatAmount } from '../composables/srFmts';
import {
  getSalesReturnItemList,
  createSalesReturnItem,
  updateSalesReturnItem,
  deleteSalesReturnItem,
  type SalesReturn,
  type SalesReturnItem,
} from '@/api/sales-return';

const { t } = useI18n({ useScope: 'global' });

const props = defineProps<{
  visible: boolean;
  currentReturn: SalesReturn | null;
}>();

const emit = defineEmits<{
  (e: 'update:visible', val: boolean): void;
}>();

// ===== 明细回源与行级维护（DRAFT 态） =====
const serverItems = ref<SalesReturnItem[]>([]);
const itemsLoading = ref(false);
const itemSaving = ref(false);
const newItem = reactive({
  productId: 1,
  quantity: 1,
  unitPrice: 0,
  // 税率留空（undefined）：不传时由后端按关联销售订单同商品明细的权威税率回填，
  // 绝不在前端默认成 0 掩盖缺失
  taxPercent: undefined as number | undefined,
  reason: '',
});
const editingItemId = ref<number | null>(null);

const refreshServerItems = async () => {
  if (!props.currentReturn?.id) return;
  const res = await getSalesReturnItemList(props.currentReturn.id);
  const data = res.data as unknown;
  serverItems.value = Array.isArray(data)
    ? data
    : ((data as { items?: SalesReturnItem[] })?.items ?? []);
};

const editable = computed(() => (props.currentReturn?.status ?? '') === 'DRAFT');

// 对话框打开时按 ID 回源最新明细
watch(
  () => props.visible,
  async val => {
    if (val && props.currentReturn?.id) {
      itemsLoading.value = true;
      try {
        const res = await getSalesReturnItemList(props.currentReturn.id);
        const data = res.data as unknown;
        serverItems.value = Array.isArray(data)
          ? data
          : ((data as { items?: SalesReturnItem[] })?.items ?? []);
      } catch (error) {
        logger.error(t('salesReturns.detailDialog.itemLoadFailed'), error);
        serverItems.value = [];
      } finally {
        itemsLoading.value = false;
      }
    } else if (!val) {
      serverItems.value = [];
    }
  }
);

const handleAddItem = async () => {
  if (!props.currentReturn?.id) return;
  itemSaving.value = true;
  try {
    await createSalesReturnItem(props.currentReturn.id, {
      // 按后端 CreateSalesReturnItemRequest 的 snake_case 契约发送；
      // tax_percent 为空时不发送，由后端从关联销售订单取权威税率（不默认 0）
      product_id: newItem.productId,
      quantity: newItem.quantity,
      unit_price: newItem.unitPrice,
      tax_percent: newItem.taxPercent ?? undefined,
      reason: newItem.reason || undefined,
    });
    ElMessage.success(t('salesReturns.detailDialog.itemSuccess'));
    await refreshServerItems();
  } catch (e) {
    ElMessage.error((e as Error).message || t('salesReturns.detailDialog.itemFailed'));
  } finally {
    itemSaving.value = false;
  }
};

/** 行编辑：回填添加栏并切换为更新模式 */
const handleEditItem = (row: SalesReturnItem) => {
  editingItemId.value = row.id ?? null;
  newItem.productId = row.product_id ?? 1;
  newItem.quantity = row.quantity ?? 1;
  newItem.unitPrice = row.unit_price ?? 0;
  newItem.taxPercent = row.tax_percent;
  newItem.reason = row.notes ?? '';
};

const handleUpdateItem = async () => {
  if (!props.currentReturn?.id || !editingItemId.value) return;
  itemSaving.value = true;
  try {
    await updateSalesReturnItem(props.currentReturn.id, editingItemId.value, {
      quantity: newItem.quantity,
      unit_price: newItem.unitPrice,
      reason: newItem.reason || undefined,
    });
    ElMessage.success(t('salesReturns.detailDialog.itemSuccess'));
    editingItemId.value = null;
    await refreshServerItems();
  } catch (e) {
    ElMessage.error((e as Error).message || t('salesReturns.detailDialog.itemFailed'));
  } finally {
    itemSaving.value = false;
  }
};

const cancelEditItem = () => {
  editingItemId.value = null;
  newItem.productId = 1;
  newItem.quantity = 1;
  newItem.unitPrice = 0;
  newItem.taxPercent = undefined;
  newItem.reason = '';
};

const handleDeleteItem = async (row: SalesReturnItem) => {
  if (!props.currentReturn?.id || !row.id) return;
  try {
    await ElMessageBox.confirm(
      t('salesReturns.detailDialog.deleteItemConfirm'),
      t('salesReturns.detailDialog.deleteItemTitle'),
      { type: 'warning' }
    );
  } catch {
    return;
  }
  try {
    await deleteSalesReturnItem(props.currentReturn.id, row.id);
    ElMessage.success(t('salesReturns.detailDialog.itemSuccess'));
    await refreshServerItems();
  } catch (e) {
    if (e !== 'cancel') {
      ElMessage.error((e as Error).message || t('salesReturns.detailDialog.itemFailed'));
    }
  }
};

/** 获取退货状态标签（i18n 响应式） */
const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    DRAFT: t('salesReturns.detailDialog.statusDraft'),
    SUBMITTED: t('salesReturns.detailDialog.statusSubmitted'),
    APPROVED: t('salesReturns.detailDialog.statusApproved'),
    REJECTED: t('salesReturns.detailDialog.statusRejected'),
    COMPLETED: t('salesReturns.detailDialog.statusCompleted'),
  };
  return map[status] || status;
};

const onClose = (val: boolean) => {
  emit('update:visible', val);
};
</script>

<style scoped>
.add-item-bar {
  display: flex;
  gap: 8px;
  margin-top: 8px;
}
</style>
