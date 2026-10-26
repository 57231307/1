<template>
  <el-dialog
    :model-value="visible"
    :title="t('greigeFabrics.detail.title')"
    width="880px"
    top="6vh"
    :aria-label="t('greigeFabrics.detail.ariaDialog')"
    data-testid="greige-detail-dialog"
    @update:model-value="emit('update:visible', $event)"
  >
    <div v-if="loading" class="detail-loading">
      <el-icon class="is-loading"><Loading /></el-icon>
      {{ t('greigeFabrics.detail.messageLoading') }}
    </div>
    <template v-else-if="row">
      <el-divider content-position="left">{{ t('greigeFabrics.detail.sectionBasic') }}</el-divider>
      <el-descriptions :column="2" border size="small">
        <el-descriptions-item :label="t('greigeFabrics.index.colFabricCode')">
          {{ row.fabric_no ?? '-' }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.index.colFabricName')">
          {{ row.fabric_name ?? '-' }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.index.colFabricType')">
          {{ row.fabric_type }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.index.colStatus')">
          <el-tag :type="row.status === GREIGE_STATUS.IN_STOCK ? 'success' : 'info'">
            {{ row.status ?? '-' }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.form.labelColorCode')">
          {{ row.color_code ?? '-' }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.detail.labelCreatedAt')">
          {{ row.created_at }}
        </el-descriptions-item>
      </el-descriptions>

      <el-divider content-position="left">{{ t('greigeFabrics.detail.sectionSource') }}</el-divider>
      <el-descriptions :column="2" border size="small">
        <el-descriptions-item :label="t('greigeFabrics.index.colSupplier')">
          {{ supplierLabel }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.form.labelProduct')">
          {{ productLabel }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.index.labelWarehouse')">
          {{ warehouseLabel }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.form.labelLocation')">
          {{ row.location ?? '-' }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.form.labelPurchaseDate')">
          {{ row.purchase_date ?? '-' }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.form.labelProductionDate')">
          {{ row.production_date ?? '-' }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.detail.labelPurchaseOrder')">
          <span v-if="row.purchase_order_id">
            {{ purchaseOrderLabel }}（{{ row.purchase_order_id }}）
          </span>
          <span v-else>-</span>
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.detail.labelPurchaseReceipt')">
          <span v-if="row.purchase_receipt_id">
            {{ purchaseReceiptLabel }}（{{ row.purchase_receipt_id }}）
          </span>
          <span v-else>-</span>
        </el-descriptions-item>
      </el-descriptions>

      <el-divider content-position="left">{{ t('greigeFabrics.detail.sectionSpec') }}</el-divider>
      <el-descriptions :column="2" border size="small">
        <el-descriptions-item :label="t('greigeFabrics.index.colComposition')">
          {{ row.composition ?? '-' }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.form.labelYarnCount')">
          {{ row.yarn_count ?? '-' }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.form.labelDensity')">
          {{ row.density ?? '-' }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.form.labelStructure')">
          {{ row.structure ?? '-' }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.form.labelWidthM')">
          {{ formatDecimal(row.width) }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.form.labelWidthCm')">
          {{ formatDecimal(row.width_cm) }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.form.labelGramWeight')">
          {{ formatDecimal(row.gram_weight) }}
        </el-descriptions-item>
      </el-descriptions>

      <el-divider content-position="left">{{ t('greigeFabrics.detail.sectionStock') }}</el-divider>
      <el-descriptions :column="2" border size="small">
        <el-descriptions-item :label="t('greigeFabrics.index.colWeightKg')">
          {{ formatDecimal(row.weight_kg) }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.index.colLengthM')">
          {{ formatDecimal(row.length_m) }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.form.labelQuantityKg')">
          {{ formatDecimal(row.quantity_kg) }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.form.labelQuantityMeters')">
          {{ formatDecimal(row.quantity_meters) }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.form.labelBatchNo')">
          {{ row.batch_no ?? '-' }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.form.labelQualityGrade')">
          {{ row.quality_grade ?? '-' }}
        </el-descriptions-item>
      </el-descriptions>

      <el-divider content-position="left">{{ t('greigeFabrics.detail.sectionTrace') }}</el-divider>
      <el-descriptions :column="2" border size="small">
        <el-descriptions-item :label="t('greigeFabrics.form.labelColorNo')">
          {{ row.color_no ?? t('greigeFabrics.detail.whiteGreige') }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.form.labelDyeLotNo')">
          {{ row.dye_lot_no ?? '-' }}
        </el-descriptions-item>
      </el-descriptions>

      <!--
        库存策略四列只摊真值：NULL 显未配置而不是 0（缺陷 1.2 语义，库里这四列可空，
        0 与 NULL 是两回事）。读取端点尚未落地，这里不合成任何补货建议、预警状态或
        触发标记；用户看到的就是列里存的值。
      -->
      <el-divider content-position="left">{{ t('greigeFabrics.detail.sectionPolicy') }}</el-divider>
      <el-descriptions :column="2" border size="small">
        <el-descriptions-item :label="t('greigeFabrics.form.labelSafetyStock')">
          {{ formatPolicyValue(row.safety_stock) }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.form.labelReorderPoint')">
          {{ formatPolicyValue(row.reorder_point) }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.form.labelMaxStockPoint')">
          {{ formatPolicyValue(row.max_stock_point) }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('greigeFabrics.form.labelReorderQuantity')">
          {{ formatPolicyValue(row.reorder_quantity) }}
        </el-descriptions-item>
      </el-descriptions>

      <el-divider content-position="left">{{
        t('greigeFabrics.detail.sectionRemarks')
      }}</el-divider>
      <el-descriptions :column="1" border size="small">
        <el-descriptions-item :label="t('greigeFabrics.form.labelRemarks')">
          {{ row.remarks ?? '-' }}
        </el-descriptions-item>
      </el-descriptions>
    </template>
    <div v-else class="detail-empty">{{ t('greigeFabrics.detail.messageNoData') }}</div>

    <template #footer>
      <el-button @click="emit('update:visible', false)">
        {{ t('greigeFabrics.index.buttonCancel') }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { Loading } from '@element-plus/icons-vue';

import { GREIGE_STATUS, getGreigeFabric, type GreigeFabric } from '@/api/greige-fabric';
import { getSupplierById } from '@/api/supplier';
import { getWarehouseById } from '@/api/warehouse';
import { getProductById } from '@/api/product';
import { getPurchaseOrderById } from '@/api/purchase';
import { getPurchaseReceipt } from '@/api/purchase-receipt';
import { decimalWireToNumber } from '@/utils/money';
import { logAuxLoadFailure } from '@/utils/logger';

/**
 * 坯布详情对话框。
 * 功能：把 36 列真实字段按基本信息 来源 规格 库存 追溯 策略 备注 七段摊开，
 * 关联实体（供应商 仓库 产品 采购订单 采购入库单）按主键回查取真名，取不到即显主键而不是猜。
 * 调用方：views/greige-fabrics/index.vue 的行操作详情入口。
 * 入参：visible 控制显隐，rowId 是打开时展示的坯布主键；每次打开按主键重新取数，
 *      避免与列表行的陈旧值错位（列表分页与筛选在父页会重取，这里以详情端点为唯一真相源）。
 * 传给谁：详情 GET /production/greige-fabrics/{id}；关联实体分别走供应商 仓库 产品
 *      各自的 GET /{id}，采购订单与入库单走各自详情端点。
 * 存什么：本对话框持有的行数据与各关联名，加载中标志；不写回父级。
 * 存哪里：组件内响应式状态，关闭即丢弃，不落 store 与浏览器存储。
 */
const props = defineProps<{
  visible: boolean;
  rowId: number | null;
}>();

const emit = defineEmits<{
  (e: 'update:visible', value: boolean): void;
}>();

const { t } = useI18n({ useScope: 'global' });

const row = ref<GreigeFabric | null>(null);
const loading = ref(false);

const supplierName = ref<string | null>(null);
const warehouseName = ref<string | null>(null);
const productName = ref<string | null>(null);
const purchaseOrderNo = ref<string | null>(null);
const purchaseReceiptNo = ref<string | null>(null);

/** Decimal 出参是字符串，展示前 Number 归一，NULL 与 undefined 一律显 '-'，不兜 0 */
const formatDecimal = (value: string | null | undefined): string =>
  value === null || value === undefined ? '-' : String(decimalWireToNumber(value));

/** 库存策略四列：NULL 是「未配置该阈值」而不是 0，与 formatDecimal 区分，显示未配置 */
const formatPolicyValue = (value: string | null | undefined): string =>
  value === null || value === undefined
    ? t('greigeFabrics.detail.policyNotConfigured')
    : formatDecimal(value);

const withIdLabel = (name: string | null, id: number | null | undefined): string => {
  if (id === null || id === undefined) return '-';
  return name ? `${name}（${id}）` : `#${id}`;
};

const supplierLabel = computed(() => withIdLabel(supplierName.value, row.value?.supplier_id));
const warehouseLabel = computed(() => withIdLabel(warehouseName.value, row.value?.warehouse_id));
const productLabel = computed(() => withIdLabel(productName.value, row.value?.product_id));
const purchaseOrderLabel = computed(() => purchaseOrderNo.value ?? '-');
const purchaseReceiptLabel = computed(() => purchaseReceiptNo.value ?? '-');

const loadRelatedNames = async (data: GreigeFabric): Promise<void> => {
  supplierName.value = null;
  warehouseName.value = null;
  productName.value = null;
  purchaseOrderNo.value = null;
  purchaseReceiptNo.value = null;

  const tasks: Promise<void>[] = [];
  if (data.supplier_id) {
    tasks.push(
      getSupplierById(data.supplier_id)
        .then(res => {
          supplierName.value = res.data?.supplier_name ?? null;
        })
        .catch(error => {
          logAuxLoadFailure('[greige.detail] 供应商名称回查失败', error);
        })
    );
  }
  if (data.warehouse_id) {
    tasks.push(
      getWarehouseById(data.warehouse_id)
        .then(res => {
          warehouseName.value = res.data?.warehouse_name ?? null;
        })
        .catch(error => {
          logAuxLoadFailure('[greige.detail] 仓库名称回查失败', error);
        })
    );
  }
  if (data.product_id) {
    tasks.push(
      getProductById(data.product_id)
        .then(res => {
          productName.value = res.data?.product_name ?? null;
        })
        .catch(error => {
          logAuxLoadFailure('[greige.detail] 产品名称回查失败', error);
        })
    );
  }
  if (data.purchase_receipt_id) {
    tasks.push(
      getPurchaseReceipt(data.purchase_receipt_id)
        .then(res => {
          purchaseReceiptNo.value = res.data?.receipt_no ?? null;
        })
        .catch(error => {
          logAuxLoadFailure('[greige.detail] 采购入库单号回查失败', error);
        })
    );
  }
  if (data.purchase_order_id) {
    // 采购订单没有单独的按主键详情端点消费面，走列表回查；取到即取号，取不到显主键而不是猜
    tasks.push(
      getPurchaseOrderById(data.purchase_order_id)
        .then(res => {
          purchaseOrderNo.value = res.data?.order_no ?? null;
        })
        .catch(error => {
          logAuxLoadFailure('[greige.detail] 采购订单号回查失败', error);
        })
    );
  }
  await Promise.all(tasks);
};

const loadDetail = async (id: number): Promise<void> => {
  loading.value = true;
  row.value = null;
  try {
    const res = await getGreigeFabric(id);
    row.value = res.data ?? null;
    if (row.value) {
      await loadRelatedNames(row.value);
    }
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('greigeFabrics.detail.messageLoadFailed'));
    logAuxLoadFailure('[greige.detail] 坯布详情取数失败', error);
  } finally {
    loading.value = false;
  }
};

watch(
  () => [props.visible, props.rowId] as const,
  ([visible, id]) => {
    if (!visible) return;
    if (typeof id === 'number') {
      void loadDetail(id);
    } else {
      row.value = null;
    }
  },
  { immediate: true }
);
</script>

<style scoped>
.detail-loading {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 16px;
  color: var(--el-text-color-secondary);
}

.detail-empty {
  padding: 24px;
  text-align: center;
  color: var(--el-text-color-secondary);
}
</style>
