<!--
  ReservationCreateDialog.vue - 从销售订单发起的新建预留对话框
  产品候选 = 来源订单明细行按产品聚合（预留粒度为 product_id，与后端 inventory_reservations 列一致）；
  仓库下拉复用销售列表页已加载的仓库数据；提交前强校验来源订单/产品/仓库/数量。
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('sales.reservation.title')"
    width="480px"
    :aria-label="t('sales.reservation.dialogAriaLabel')"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form :model="form" :aria-label="t('sales.reservation.formAriaLabel')" label-width="110px">
      <el-form-item :label="t('sales.reservation.sourceOrder')">
        <el-input :model-value="orderNo" readonly />
      </el-form-item>
      <el-form-item :label="t('sales.reservation.customer')">
        <el-input :model-value="customerName" readonly />
      </el-form-item>
      <el-form-item :label="t('sales.reservation.product')" required prop="product_id">
        <el-select
          v-model="form.product_id"
          :placeholder="t('sales.reservation.productPlaceholder')"
          style="width: 100%"
        >
          <el-option
            v-for="c in candidates"
            :key="c.product_id"
            :label="productOptionLabel(c)"
            :value="c.product_id"
          />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('sales.reservation.warehouse')" required prop="warehouse_id">
        <el-select
          v-model="form.warehouse_id"
          :placeholder="t('sales.reservation.warehousePlaceholder')"
          style="width: 100%"
        >
          <el-option
            v-for="w in warehouses"
            :key="w.id"
            :label="w.warehouse_name ?? w.name"
            :value="w.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('sales.reservation.quantity')" required prop="quantity">
        <el-input-number
          v-model="form.quantity"
          :min="0.01"
          :max="selectedMax ?? undefined"
          :precision="2"
        />
      </el-form-item>
      <el-form-item :label="t('sales.reservation.remark')" prop="notes">
        <el-input
          v-model="form.notes"
          type="textarea"
          :rows="2"
          :placeholder="t('sales.reservation.remarkPlaceholder')"
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
import { reactive, computed, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import type { SalesOrderItem } from '@/api/sales';

const { t } = useI18n({ useScope: 'global' });

/**
 * 预留产品候选：按 product_id 聚合订单明细行。
 * 预留落库粒度为产品（product_id），同产品多色号行合并为一个候选：
 * color_no 集合用于下拉区分展示，quantitySum 为该产品的订单数量上限。
 */
interface ReservationProductCandidate {
  product_id: number;
  product_name: string;
  colorNos: string[];
  quantitySum: number;
}

/** 提交体：order_id 由父组件带真实来源订单 id（候选与数量在前端已按契约校验） */
interface ReservationSubmit {
  order_id: number;
  product_id: number;
  warehouse_id: number;
  quantity: string;
  notes?: string;
}

const props = defineProps<{
  visible: boolean;
  /** 来源销售订单真实 id（order_id>0 是提交硬前提，缺失即组件用法错误） */
  orderId: number;
  orderNo: string;
  customerName: string;
  /** 来源订单明细行（父组件经 GET /sales/orders/{id} 回读后传入） */
  items: SalesOrderItem[];
  warehouses: { id: number; warehouse_name?: string; name?: string }[];
  submitting?: boolean;
}>();

const emit = defineEmits<{
  'update:visible': [value: boolean];
  submit: [value: ReservationSubmit];
}>();

const form = reactive({
  product_id: undefined as number | undefined,
  warehouse_id: undefined as number | undefined,
  quantity: undefined as number | undefined,
  notes: '',
});

// 对话框组件跨订单复用（v-model 常驻），每次打开清空上一次输入，
// 防止前一订单选择的已提交产品/数量残值被误当作本次预填值
watch(
  () => props.visible,
  v => {
    if (v) {
      form.product_id = undefined;
      form.warehouse_id = undefined;
      form.quantity = undefined;
      form.notes = '';
    }
  }
);

const candidates = computed<ReservationProductCandidate[]>(() => {
  const byProduct = new Map<number, ReservationProductCandidate>();
  for (const it of props.items) {
    const existing = byProduct.get(it.product_id);
    if (existing) {
      existing.quantitySum += it.quantity;
      if (it.color_no && !existing.colorNos.includes(it.color_no)) {
        existing.colorNos.push(it.color_no);
      }
    } else {
      byProduct.set(it.product_id, {
        product_id: it.product_id,
        product_name: it.product_name,
        colorNos: it.color_no ? [it.color_no] : [],
        quantitySum: it.quantity,
      });
    }
  }
  return [...byProduct.values()];
});

const productOptionLabel = (c: ReservationProductCandidate) =>
  c.colorNos.length ? `${c.product_name}（${c.colorNos.join(' / ')}）` : c.product_name;

const selectedCandidate = computed(
  () => candidates.value.find(c => c.product_id === form.product_id) ?? null
);

/** 已选产品的订单数量合计 = 预留数量上限；未选产品时不设上限（由提交校验拦截） */
const selectedMax = computed(() => selectedCandidate.value?.quantitySum ?? null);

const handleSubmit = () => {
  if (props.orderId <= 0) {
    ElMessage.warning(t('sales.reservation.noSourceOrder'));
    return;
  }
  if (!form.product_id) {
    ElMessage.warning(t('sales.reservation.productRequired'));
    return;
  }
  if (!form.warehouse_id) {
    ElMessage.warning(t('sales.reservation.warehouseRequired'));
    return;
  }
  if (!form.quantity || form.quantity <= 0) {
    ElMessage.warning(t('sales.reservation.quantityRequired'));
    return;
  }
  // 候选上限来自真实订单明细数量合计；超限直接拒绝提交，不做静默截断
  const max = selectedMax.value;
  if (max !== null && form.quantity > max) {
    ElMessage.warning(t('sales.reservation.quantityExceed', { max }));
    return;
  }
  emit('submit', {
    order_id: props.orderId,
    product_id: form.product_id,
    warehouse_id: form.warehouse_id,
    quantity: String(form.quantity),
    notes: form.notes.trim() || undefined,
  });
};
</script>
