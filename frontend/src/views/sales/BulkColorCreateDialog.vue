<!--
  BulkColorCreateDialog.vue - 从销售订单发起的大货批色申请对话框
  产品/色号候选 = 来源订单明细行（sales_order_item_id 绑定后由后端按该行 product_id/color_no 覆盖回填）；
  客户由来源订单带入只读展示；染缸批次为可搜索选择；提交前强校验订单/明细行/染缸批次。
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('sales.bulkColor.title')"
    width="480px"
    :aria-label="t('sales.bulkColor.dialogAriaLabel')"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form :model="form" :aria-label="t('sales.bulkColor.formAriaLabel')" label-width="110px">
      <el-form-item :label="t('sales.bulkColor.sourceOrder')">
        <el-input :model-value="orderNo" readonly />
      </el-form-item>
      <el-form-item :label="t('sales.bulkColor.customer')">
        <el-input :model-value="customerName" readonly />
      </el-form-item>
      <el-form-item :label="t('sales.bulkColor.item')" required prop="sales_order_item_id">
        <el-select
          v-model="form.sales_order_item_id"
          filterable
          :placeholder="t('sales.bulkColor.itemPlaceholder')"
          style="width: 100%"
        >
          <el-option v-for="it in items" :key="it.id" :label="itemOptionLabel(it)" :value="it.id" />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('sales.bulkColor.dyeBatch')" required prop="dye_batch_id">
        <el-select
          v-model="form.dye_batch_id"
          filterable
          :loading="dyeBatchLoading"
          :placeholder="t('sales.bulkColor.dyeBatchPlaceholder')"
          style="width: 100%"
        >
          <el-option
            v-for="d in dyeBatchOptions"
            :key="d.id"
            :label="dyeBatchLabel(d)"
            :value="d.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('sales.bulkColor.remark')" prop="remark">
        <el-input
          v-model="form.remark"
          type="textarea"
          :rows="2"
          :placeholder="t('sales.bulkColor.remarkPlaceholder')"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">{{ t('common.cancel') }}</el-button>
      <el-button type="primary" :loading="submitting" @click="handleSubmit">
        {{ t('common.save') }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { reactive, ref, computed, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { logAuxLoadFailure } from '@/utils/logger';
import type { SalesOrderItem } from '@/api/sales';
import { getDyeBatchList, type DyeBatch } from '@/api/dye-batch';

const { t } = useI18n({ useScope: 'global' });

/**
 * 提交体：sales_order_id / customer_id 由来源订单上下文带入（父组件经 GET /sales/orders/{id}
 * 回读明细后传入），product_id / color_no 为所选明细行只读带出，后端以 sales_order_item_id
 * 对应的明细行为准覆盖回填 product_id/color_no。
 */
interface BulkColorSubmit {
  sales_order_id: number;
  customer_id: number;
  sales_order_item_id: number;
  dye_batch_id: number;
  product_id: number;
  color_no?: string;
  remark?: string;
}

const props = defineProps<{
  visible: boolean;
  /** 来源销售订单真实 id（>0 是提交硬前提，缺失即组件用法错误） */
  orderId: number;
  orderNo: string;
  /** 来源销售订单客户 id（customer_id 提交必填的来源，不再手输） */
  customerId: number;
  customerName: string;
  /** 来源订单明细行（父组件经 GET /sales/orders/{id} 回读后传入） */
  items: SalesOrderItem[];
  submitting?: boolean;
}>();

const emit = defineEmits<{
  'update:visible': [value: boolean];
  submit: [value: BulkColorSubmit];
}>();

const form = reactive({
  sales_order_item_id: undefined as number | undefined,
  dye_batch_id: undefined as number | undefined,
  remark: '',
});

const dyeBatchOptions = ref<DyeBatch[]>([]);
const dyeBatchLoading = ref(false);

const loadDyeBatchOptions = async () => {
  dyeBatchLoading.value = true;
  try {
    const res = await getDyeBatchList({ page: 1, page_size: 200 });
    const list = res.data.items;
    if (!Array.isArray(list)) {
      throw new Error('GET /production/dye-batches 响应缺少 data.items 数组');
    }
    dyeBatchOptions.value = list;
  } catch (error) {
    dyeBatchOptions.value = [];
    logAuxLoadFailure(t('sales.bulkColor.loadDyeBatchesFailed'), error);
  } finally {
    dyeBatchLoading.value = false;
  }
};

// 对话框跨订单复用（v-model 常驻），每次打开清空上一次输入并刷新染缸候选，
// 防止前一订单选择的明细行/缸号残值被误当作本次预填值
watch(
  () => props.visible,
  v => {
    if (v) {
      form.sales_order_item_id = undefined;
      form.dye_batch_id = undefined;
      form.remark = '';
      void loadDyeBatchOptions();
    }
  }
);

const itemOptionLabel = (it: SalesOrderItem) =>
  it.color_no ? `${it.product_name}（${it.color_no}）` : it.product_name;

const dyeBatchLabel = (d: DyeBatch) => {
  const parts = [d.batch_no, d.dye_lot_no, d.color_no].filter(v => !!v);
  return parts.join(' / ');
};

const selectedItem = computed(
  () => props.items.find(it => it.id === form.sales_order_item_id) ?? null
);

const handleSubmit = () => {
  if (props.orderId <= 0) {
    ElMessage.warning(t('sales.bulkColor.noSourceOrder'));
    return;
  }
  if (!form.sales_order_item_id || !selectedItem.value) {
    ElMessage.warning(t('sales.bulkColor.itemRequired'));
    return;
  }
  if (!form.dye_batch_id) {
    ElMessage.warning(t('sales.bulkColor.dyeBatchRequired'));
    return;
  }
  emit('submit', {
    sales_order_id: props.orderId,
    customer_id: props.customerId,
    sales_order_item_id: form.sales_order_item_id,
    dye_batch_id: form.dye_batch_id,
    product_id: selectedItem.value.product_id,
    color_no: selectedItem.value.color_no,
    remark: form.remark.trim() || undefined,
  });
};
</script>
