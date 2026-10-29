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
            prop="product_name"
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
        <el-form
          v-if="editable"
          ref="itemFormRef"
          :model="newItem"
          :rules="itemRules"
          :inline="true"
          class="add-item-bar"
        >
          <el-form-item prop="productId">
            <el-input
              v-model="newItem.productId"
              inputmode="numeric"
              placeholder="产品ID"
              :disabled="editingItemId !== null"
              style="width: 140px"
            />
          </el-form-item>
          <el-form-item prop="quantity">
            <el-input
              v-model="newItem.quantity"
              inputmode="numeric"
              placeholder="数量"
              style="width: 120px"
            />
          </el-form-item>
          <el-form-item prop="unitPriceWithTax">
            <el-input
              v-model="newItem.unitPriceWithTax"
              inputmode="decimal"
              :placeholder="t('salesReturns.detailDialog.placeholderUnitPriceWithTax')"
              style="width: 130px"
            />
          </el-form-item>
          <el-form-item prop="taxPercent">
            <el-input
              v-model="newItem.taxPercent"
              inputmode="decimal"
              :placeholder="t('salesReturns.detailDialog.placeholderTaxPercent')"
              style="width: 180px"
            />
          </el-form-item>
          <el-form-item>
            <el-input v-model="newItem.reason" style="width: 160px" placeholder="退货原因" />
          </el-form-item>
          <el-form-item>
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
          </el-form-item>
        </el-form>
      </div>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import { logger } from '@/utils/logger';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus';
import { isDialogDismissal, rethrowNonDismissal } from '@/utils/monitor';
import { numberInputRules, numericView } from '@/utils/number-input';
import { netFromInclusive, roundToCent } from '@/utils/money';
import { formatAmount } from '../composables/srFmts';
import {
  salesReturnStatusLabelKey,
  salesReturnStatusTagType as getStatusType,
} from '@/utils/sales-return-status';
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
// 明细维护栏手写录入文本模型（数字约束收进 numberInputRules 单源校验器，不靠步进按钮）；
// 含税单价是录入事实（主输入），不含税单价提交前按税率除法反算派生，不手输
const newItem = reactive({
  productId: '1',
  quantity: '1',
  // 含税单价初值留空由人工填写（0 只能由人工显式敲出）
  unitPriceWithTax: '',
  // 税率与含税单价同行必填：含税事实要靠税率除法反算净额，缺税率前端算不出钩稽值，
  // 送后端必然被单价钩稽门点名拒绝，不在录入途中掩盖缺失
  taxPercent: '',
  reason: '',
});
const itemFormRef = ref<FormInstance>();
const itemRules = computed<FormRules>(() => ({
  productId: numberInputRules({ label: '产品ID', required: true, min: 1, integer: true }),
  quantity: numberInputRules({ label: '退货数量', required: true, min: 1 }),
  unitPriceWithTax: numberInputRules({
    label: t('salesReturns.detailDialog.labelUnitPriceWithTax'),
    required: true,
    min: 0,
    precision: 2,
  }),
  taxPercent: numberInputRules({
    label: t('salesReturns.detailDialog.labelTaxPercent'),
    required: true,
    min: 0,
    max: 100,
    precision: 2,
  }),
}));

/**
 * 提交前构造行金额载荷（含税口径单源腿）。
 * 入参，含税单价与税率文本已由 itemRules 判为合法非空；出参是 unit_price 派生净额与
 * unit_price_with_tax 原文两键，净额按 money.ts 的 netFromInclusive 除法反算，
 * 与后端 utils/tax.rs 反算值分位一致，后端 ensure_unit_price_reconciles 复核必然通过。
 * 传给谁：handleAddItem 的 POST 载荷与 handleUpdateItem 的 PUT 载荷。
 * 本函数不存在 乘 (1 + 税率) 造含税 或 乘税率造税额 的第二套公式。
 */
const buildPricePayload = (): { unit_price: number; unit_price_with_tax: string } => {
  const inclusive = Number(newItem.unitPriceWithTax);
  const rate = Number(newItem.taxPercent);
  return {
    unit_price: netFromInclusive(inclusive, rate),
    unit_price_with_tax: newItem.unitPriceWithTax.trim(),
  };
};
const validateItemForm = async (): Promise<boolean> => {
  let ok = false;
  await itemFormRef.value?.validate(valid => {
    ok = valid;
  });
  return ok;
};
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
  const valid = await validateItemForm();
  if (!valid) return;
  itemSaving.value = true;
  try {
    const price = buildPricePayload();
    await createSalesReturnItem(props.currentReturn.id, {
      // 按后端 CreateSalesReturnItemRequest 的 snake_case 契约发送；
      // 含税单价是录入事实随 unit_price_with_tax 送，不含税单价是除法反算的派生值，
      // 税率同行必填（0 = 显式无票声明），后端 ensure_unit_price_reconciles 逐分复核
      product_id: Number(newItem.productId),
      quantity: Number(newItem.quantity),
      unit_price: price.unit_price,
      unit_price_with_tax: price.unit_price_with_tax,
      tax_percent: Number(newItem.taxPercent),
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

/**
 * 行编辑：回填添加栏并切换为更新模式
 * 后端 quantity unit_price tax_percent 是 rust_decimal 出参走线为字符串（形如 "10.0000"）
 * 先经 Number() 归一再按录入框形态格式化 缺值留空串 由必填与范围规则点名是哪个字段
 * 含税单价回显只有除法还原一条腿，无折扣行按 成交含税 ÷ 数量，还原不出留空要求按实录入，
 * 绝不按 不含税单价 乘 一加税率 倒造
 */
const handleEditItem = (row: SalesReturnItem) => {
  editingItemId.value = row.id ?? null;
  newItem.productId = numericView(row.product_id);
  newItem.quantity = row.quantity == null ? '' : numericView(Number(row.quantity), 2);
  const qty = Number(row.quantity);
  const recoverable = Number(row.discount_amount) === 0 && Number.isFinite(qty) && qty > 0;
  newItem.unitPriceWithTax = recoverable
    ? numericView(roundToCent(Number(row.total_amount) / qty), 2)
    : '';
  newItem.taxPercent = row.tax_percent == null ? '' : numericView(Number(row.tax_percent), 2);
  newItem.reason = row.notes ?? '';
};

const handleUpdateItem = async () => {
  if (!props.currentReturn?.id || !editingItemId.value) return;
  const valid = await validateItemForm();
  if (!valid) return;
  itemSaving.value = true;
  try {
    const price = buildPricePayload();
    // 三态契约（后端 UpdateReturnItemRequest DoubleOption）：quantity/unit_price 为
    // NOT NULL 列恒送值；含税单价是主输入事实恒送覆盖，后端按它反算并逐分复核单价；
    // reason 映射 DB 可空列 notes，清空 ⇒ 显式 null（不塌成省略）
    await updateSalesReturnItem(props.currentReturn.id, editingItemId.value, {
      quantity: Number(newItem.quantity),
      unit_price: price.unit_price,
      unit_price_with_tax: price.unit_price_with_tax,
      reason: newItem.reason || null,
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
  newItem.productId = '1';
  newItem.quantity = '1';
  newItem.unitPriceWithTax = '';
  newItem.taxPercent = '';
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
  } catch (error: unknown) {
    if (isDialogDismissal(error)) return;
    rethrowNonDismissal('salesReturns.handleDeleteItem', error);
  }
  try {
    await deleteSalesReturnItem(props.currentReturn.id, row.id);
    ElMessage.success(t('salesReturns.detailDialog.itemSuccess'));
    await refreshServerItems();
  } catch (e) {
    if (!isDialogDismissal(e)) {
      ElMessage.error((e as Error).message || t('salesReturns.detailDialog.itemFailed'));
    }
  }
};

/** 退货状态标签：词表与标签键的单一来源在 utils/sales-return-status.ts */
const getStatusLabel = (status: string) => {
  const key = salesReturnStatusLabelKey(status);
  return key ? t(key) : '';
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
.add-item-bar .el-form-item {
  margin-bottom: 0;
}
</style>
