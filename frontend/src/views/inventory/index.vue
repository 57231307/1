<!--
  inventory/index.vue - 库存管理主入口（拆分重构版）
  任务编号: P14 批 2 I-3 第 8 批
  拆分：600 行 → ~280 行 + 3 子组件 + 1 工具
  原 899 行已拆为 tabs/ 子组件，本批再拆统计卡片 + 2 个对话框 + 工具
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="inventory-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">{{ t('inventory.page.title') }}</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">{{
            t('inventory.page.home')
          }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ t('inventory.page.warehouseManage') }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ t('inventory.page.stockLedger') }}</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button
          v-permission="PERMISSIONS.INVENTORY_UPDATE"
          type="primary"
          @click="handleAdjustment"
        >
          <el-icon><Edit /></el-icon>
          {{ t('inventory.page.adjustment') }}
        </el-button>
        <el-button v-permission="PERMISSIONS.INVENTORY_TRANSFER" @click="goToTransferPage">
          <el-icon><RefreshRight /></el-icon>
          {{ t('inventory.page.transfer') }}
        </el-button>
        <el-button v-permission="'inventory.print'" @click="handlePrint">
          <el-icon><Printer /></el-icon>
          {{ t('inventory.page.print') }}
        </el-button>
        <el-button v-permission="'inventory.export'" @click="handleExport">
          <el-icon><Download /></el-icon>
          {{ t('inventory.page.export') }}
        </el-button>
      </div>
    </div>

    <StatCards :stats="stats" />

    <el-tabs v-model="activeTab" @tab-change="handleTabChange">
      <el-tab-pane :label="t('inventory.page.tabStock')" name="stock">
        <InventoryStockTab
          :stocks="stocks"
          :total="total"
          :loading="loading"
          :query-params="queryParams"
          :warehouses="warehouses"
          @view="handleView"
          @query="fetchData"
          @reset="handleReset"
          @create="openStockDialog()"
          @edit="openStockDialog"
          @delete="handleDeleteStock"
          @export="handleExportStock"
          @update:query-params="(v: StockQuery) => Object.assign(queryParams, v)"
        />
      </el-tab-pane>

      <el-tab-pane :label="t('inventory.page.tabAlert')" name="alert">
        <InventoryAlertTab :alerts="alerts" @purchase="handlePurchase" />
      </el-tab-pane>

      <el-tab-pane :label="t('inventory.page.tabTransaction')" name="transaction" lazy>
        <InventoryTransactionTab />
      </el-tab-pane>

      <el-tab-pane :label="t('inventory.page.tabReservation')" name="reservation" lazy>
        <InventoryReservationTab />
      </el-tab-pane>

      <el-tab-pane :label="t('inventory.page.tabFabricStock')" name="fabricStock" lazy>
        <FabricStockTab />
      </el-tab-pane>
    </el-tabs>

    <AdjustmentDialog
      v-model:visible="adjustmentDialogVisible"
      :initial-form="adjustmentForm"
      :warehouses="warehouses"
      :products="products"
      @submit="onSubmitAdjustment"
    />

    <!-- 库存记录新建/编辑对话框 -->
    <el-dialog
      v-model="stockDialogVisible"
      :title="stockEditingId ? t('inventory.stockTab.edit') : t('inventory.stockTab.create')"
      width="520px"
    >
      <el-form :model="stockForm" label-width="110px">
        <!-- 编辑走 PUT /inventory/stock/{id}：该端点只接收数量纠偏与库位，
             产品/仓库/批次/色号提交也不会被后端读取——置灰并在表单内如实说明，
             不做"改了会被静默丢弃"的假可编辑。四维/归属变更走库存调整/出入库流程 -->
        <el-alert
          v-if="stockEditingId"
          type="info"
          :closable="false"
          :title="t('inventory.stockDialog.editReadonlyHint')"
          class="stock-edit-hint"
        />
        <el-alert
          v-if="!stockEditingId"
          type="info"
          :closable="false"
          :title="t('inventory.stockDialog.createLocationHint')"
          class="stock-edit-hint"
        />
        <el-form-item :label="t('inventory.stockTab.colProductCode')">
          <el-input-number v-model="stockForm.product_id" :min="1" :disabled="!!stockEditingId" />
        </el-form-item>
        <el-form-item :label="t('inventory.stockTab.colWarehouse')">
          <el-select v-model="stockForm.warehouse_id" filterable :disabled="!!stockEditingId">
            <el-option
              v-for="wh in warehouses"
              :key="wh.id"
              :label="wh.warehouse_name"
              :value="wh.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('inventory.stockTab.colBatchNo')">
          <el-input v-model="stockForm.batch_no" :disabled="!!stockEditingId" />
        </el-form-item>
        <el-form-item :label="t('inventory.stockTab.colColorCode')">
          <el-input v-model="stockForm.color_code" :disabled="!!stockEditingId" />
        </el-form-item>
        <!-- 等级词表唯一来源 constants/stock-grade.ts（一等品/二等品/等外品）；
             PUT 不接收 grade → 编辑态禁用；提交值本身、label 走 i18n 键 -->
        <el-form-item :label="t('inventory.stockTab.colGrade')">
          <el-select v-model="stockForm.grade" :disabled="!!stockEditingId">
            <el-option
              v-for="g in STOCK_GRADE_VALUES"
              :key="g"
              :label="t(STOCK_GRADE_LABEL_KEY[g])"
              :value="g"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('inventory.stockTab.colLocation')">
          <el-input v-model="stockForm.location" :disabled="!stockEditingId" />
        </el-form-item>
        <el-form-item :label="t('inventory.stockTab.colQuantity')">
          <el-input-number v-model="stockForm.quantity" :min="0" :precision="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="stockDialogVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="stockSubmitLoading" @click="submitStock">
          {{ t('common.save') }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Edit, RefreshRight, Download, Printer } from '@element-plus/icons-vue';
import printJS from 'print-js';
import { useRouter } from 'vue-router';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';

// 接入 i18n，替换硬编码中文文案
const { t } = useI18n({ useScope: 'global' });
import { exportFromBackend } from '@/utils/export';
import type { InventoryStock, StockAlert } from '@/api/inventory';
import type { Warehouse } from '@/api/warehouse';
import type { Product } from '@/api/product';
import InventoryStockTab, { type StockQuery } from './tabs/InventoryStockTab.vue';
import InventoryAlertTab from './tabs/InventoryAlertTab.vue';
import InventoryTransactionTab from './tabs/InventoryTransactionTab.vue';
import InventoryReservationTab from './tabs/InventoryReservationTab.vue';
import FabricStockTab from './tabs/FabricStockTab.vue';
import StatCards from './components/StatCards.vue';
import AdjustmentDialog, { type AdjustmentForm } from './components/AdjustmentDialog.vue';
import { PERMISSIONS } from '@/constants/permissions';
import { STOCK_GRADE, STOCK_GRADE_VALUES, STOCK_GRADE_LABEL_KEY } from '@/constants/stock-grade';
import { formatNumber, getStockStatusLabel } from './composables/invFmts';
import { logger } from '@/utils/logger';

const hasLoaded = createLazyLoader();
const router = useRouter();

const loading = ref(false);
const activeTab = ref('stock');
const stocks = ref<InventoryStock[]>([]);
const alerts = ref<StockAlert[]>([]);
const warehouses = ref<Warehouse[]>([]);
const products = ref<Product[]>([]);
const total = ref(0);

const stats = ref({
  alertCount: 0,
});

const queryParams = reactive<StockQuery>({
  page: 1,
  page_size: 20,
  keyword: '',
  warehouse_id: undefined,
  stock_status: '',
});

const fetchData = async () => {
  loading.value = true;
  try {
    const { getStockList } = await import('@/api/inventory');
    const res = await getStockList(queryParams);
    stocks.value = res.data?.items || [];
    total.value = res.data?.total || 0;
  } catch (error: unknown) {
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) ||
        t('inventory.message.fetchStockFailed')
    );
    stocks.value = [];
    total.value = 0;
  } finally {
    loading.value = false;
  }
};

const fetchAlerts = async () => {
  try {
    const { getStockAlertList, STOCK_ALERT_PAGE_SIZE } = await import('@/api/inventory');
    const res = await getStockAlertList({ page: 1, page_size: STOCK_ALERT_PAGE_SIZE });
    const payload = res.data;
    if (!payload || !Array.isArray(payload.items)) {
      throw new Error('库存预警出参缺少 items 数组');
    }
    alerts.value = payload.items;
    stats.value.alertCount = payload.total;
    if (payload.items.length < payload.total) {
      logger.warn(
        `库存预警仅显示前 ${payload.items.length} 条（共 ${payload.total} 条），请按仓库/产品缩小范围查看剩余告警`
      );
    }
  } catch (error: unknown) {
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) ||
        t('inventory.message.fetchAlertFailed')
    );
    alerts.value = [];
  }
};

const fetchWarehouses = async () => {
  try {
    const { getWarehouseList } = await import('@/api/warehouse');
    const res = await getWarehouseList({ page: 1, page_size: 1000 });
    warehouses.value = res.data?.items || [];
  } catch (error: unknown) {
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) ||
        t('inventory.message.fetchWarehouseFailed')
    );
    warehouses.value = [];
  }
};

const fetchProducts = async () => {
  try {
    const { getProductList } = await import('@/api/product');
    const res = await getProductList({ page: 1, page_size: 1000 });
    products.value = res.data?.items || [];
  } catch (error: unknown) {
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) ||
        t('inventory.message.fetchProductFailed')
    );
    products.value = [];
  }
};

const handleReset = () => {
  queryParams.keyword = '';
  queryParams.warehouse_id = undefined;
  queryParams.stock_status = '';
  queryParams.page = 1;
  fetchData();
};

const handleTabChange = (tabName: string) => {
  if (tabName === 'alert') {
    fetchAlerts();
  }
};

const adjustmentDialogVisible = ref(false);
const adjustmentForm = ref<AdjustmentForm>({
  stock_id: null,
  product_id: null,
  warehouse_id: null,
  product_name: '',
  warehouse_name: '',
  current_quantity: 0,
  adjustment_type: 'increase',
  adjustment_quantity: 0,
  reason: '',
});

const handleAdjustment = () => {
  adjustmentForm.value = {
    stock_id: null,
    product_id: null,
    warehouse_id: null,
    product_name: '',
    warehouse_name: '',
    current_quantity: 0,
    adjustment_type: 'increase',
    adjustment_quantity: 0,
    reason: '',
  };
  adjustmentDialogVisible.value = true;
};

const onSubmitAdjustment = async (form: AdjustmentForm) => {
  if (form.product_id === null || form.warehouse_id === null) {
    ElMessage.warning(t('inventory.message.adjustmentProductWarehouseRequired'));
    return;
  }
  if (!form.adjustment_quantity || form.adjustment_quantity <= 0) {
    ElMessage.warning(t('inventory.message.adjustmentQtyInvalid'));
    return;
  }
  if (!form.reason) {
    ElMessage.warning(t('inventory.message.reasonRequired'));
    return;
  }
  try {
    const { createStockAdjustment, getStockList } = await import('@/api/inventory');
    const stockRes = await getStockList({
      page: 1,
      page_size: 1,
      warehouse_id: form.warehouse_id,
      product_id: form.product_id,
    });
    const stockRow = stockRes.data?.items?.[0];
    if (!stockRow) {
      ElMessage.error(t('inventory.message.adjustmentFailed'));
      return;
    }
    await createStockAdjustment({
      warehouse_id: form.warehouse_id,
      adjustment_date: new Date().toISOString(),
      adjustment_type: form.adjustment_type,
      reason_type: form.reason,
      reason_description: form.reason,
      items: [{ stock_id: stockRow.id, quantity: String(form.adjustment_quantity) }],
    });
    ElMessage.success(t('inventory.message.adjustmentSuccess'));
    adjustmentDialogVisible.value = false;
    fetchData();
  } catch (error: unknown) {
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) ||
        t('inventory.message.adjustmentFailed')
    );
  }
};

const goToTransferPage = () => {
  router.push({ name: 'InventoryTransfer' });
};

// 库存记录新建/编辑对话框（createStock / updateStock）
const stockDialogVisible = ref(false);
const stockEditingId = ref<number | null>(null);
const stockSubmitLoading = ref(false);
// 编辑整行引用：version（乐观锁）等只读上下文取自 GET 出参真实列，禁止假值
const stockEditingRow = ref<InventoryStock | null>(null);
const stockForm = reactive({
  product_id: undefined as number | undefined,
  warehouse_id: undefined as number | undefined,
  batch_no: '',
  color_code: '',
  // 等级取值域唯一来源 constants/stock-grade.ts（NOT NULL 列，新建必须显式给值）
  grade: STOCK_GRADE.first as string,
  location: '',
  quantity: 0,
});

const openStockDialog = (row?: InventoryStock) => {
  stockEditingId.value = row ? row.id : null;
  stockEditingRow.value = row ?? null;
  stockForm.product_id = row?.product_id;
  stockForm.warehouse_id = row?.warehouse_id;
  stockForm.batch_no = row?.batch_no || '';
  stockForm.color_code = row?.color_no || '';
  stockForm.grade = row?.grade || STOCK_GRADE.first;
  stockForm.location = row?.bin_location || '';
  // 新建的「数量」按 POST 契约进主计量 quantity_meters；编辑按 PUT 纠偏口径读写在库量
  // quantity_on_hand（该端点不接收 quantity_meters，两列在后端各自独立，不混写）
  stockForm.quantity = row ? Number(row.quantity_on_hand) : 0;
  stockDialogVisible.value = true;
};

const submitStock = async () => {
  if (!stockForm.product_id || !stockForm.warehouse_id) {
    ElMessage.warning(t('inventory.message.productWarehouseRequired'));
    return;
  }
  stockSubmitLoading.value = true;
  try {
    if (stockEditingRow.value) {
      // 后端 PUT /inventory/stock/{id}（update_stock, handlers/inventory_stock_handler.rs）
      // 走乐观锁：UpdateStockWithVersionRequest.version 必填，真值取自该行 GET 出参的
      // version 列（StockResponse.version 直映 inventory_stocks.version）。
      // 若后端出参缺失 version（旧部署），row.version 为 undefined → JSON 键被丢弃 →
      // serde 必填校验在请求边界拒绝并显式报错，绝不以 0 等假值蒙混乐观锁。
      // 产品/仓库/批次/色号不在该端点入参内（编辑态已禁用），四维/归属变更走调整/出入库。
      const { updateStock } = await import('@/api/inventory');
      await updateStock(stockEditingRow.value.id, {
        quantity_on_hand: String(stockForm.quantity),
        // bin_location 为 Option 入参：后端只在 Some 时写入，空串即用户「清空库位」的
        // 显式意图（null/省略键都无法表达清除），原样提交表单当前值
        bin_location: stockForm.location,
        version: stockEditingRow.value.version,
      });
      ElMessage.success(t('common.success'));
      stockDialogVisible.value = false;
      fetchData();
      return;
    }
    const { createStock } = await import('@/api/inventory');
    // POST /inventory/stock（CreateStockFabricRequest）不接收 bin_location——
    // 库位只能在保存后经编辑 PUT 写入（表单已在创建态禁用该输入并提示）；
    // grade 取 constants/stock-grade.ts 词表值（此前硬编码 'A' 落在取值域之外）
    await createStock({
      warehouse_id: stockForm.warehouse_id,
      product_id: stockForm.product_id,
      batch_no: stockForm.batch_no,
      color_no: stockForm.color_code,
      grade: stockForm.grade,
      quantity_meters: stockForm.quantity,
    });
    ElMessage.success(t('common.success'));
    stockDialogVisible.value = false;
    fetchData();
  } catch (error: unknown) {
    ElMessage.error((error instanceof Error ? error.message : String(error)) || t('common.failed'));
  } finally {
    stockSubmitLoading.value = false;
  }
};

// 删除库存记录（deleteStock，确认后执行）
const handleDeleteStock = async (row: InventoryStock) => {
  try {
    await ElMessageBox.confirm(t('inventory.message.deleteStockConfirm'), t('common.delete'), {
      type: 'warning',
    });
  } catch {
    return;
  }
  try {
    const { deleteStock } = await import('@/api/inventory');
    await deleteStock(row.id);
    ElMessage.success(t('common.success'));
    fetchData();
  } catch (error: unknown) {
    ElMessage.error((error instanceof Error ? error.message : String(error)) || t('common.failed'));
  }
};

// 导出库存 xlsx（exportStock，Blob 下载）
const handleExportStock = async () => {
  try {
    const { exportStock } = await import('@/api/inventory');
    const res = await exportStock(queryParams);
    const blob = res as unknown as Blob;
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `inventory-${new Date().toISOString().split('T')[0]}.xlsx`;
    a.click();
    URL.revokeObjectURL(url);
  } catch (error: unknown) {
    ElMessage.error((error instanceof Error ? error.message : String(error)) || t('common.failed'));
  }
};

const handleView = async (row: InventoryStock) => {
  try {
    const { getStockById } = await import('@/api/inventory');
    const res = await getStockById(row.id);
    const d = res.data;
    if (!d) {
      ElMessage.warning(t('inventory.message.stockDetailNotFound'));
      return;
    }
    const lines = [
      t('inventory.stockDetail.productCode', { value: d.product_code }),
      t('inventory.stockDetail.productName', { value: d.product_name }),
      t('inventory.stockDetail.warehouse', { value: d.warehouse_name }),
      t('inventory.stockDetail.batchNo', { value: d.batch_no || '-' }),
      t('inventory.stockDetail.color', { value: d.color_no || '-' }),
      t('inventory.stockDetail.dyeLot', { value: d.dye_lot_no || '-' }),
      t('inventory.stockDetail.qtyMeters', { value: d.quantity_meters }),
      t('inventory.stockDetail.qtyKg', { value: d.quantity_kg }),
      t('inventory.stockDetail.status', { value: getStockStatusLabel(d.stock_status, t) }),
      t('inventory.stockDetail.location', { value: d.bin_location || '-' }),
    ];
    await ElMessageBox.alert(lines.join('\n'), t('inventory.stockDetail.title'), {
      confirmButtonText: t('inventory.stockDetail.close'),
    });
  } catch (error) {
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) ||
        t('inventory.message.fetchStockDetailFailed')
    );
  }
};

const handlePurchase = (row: StockAlert) => {
  router.push({ name: 'Purchase', query: { product_name: row.product_name || '' } });
};

const handlePrint = () => {
  printJS({
    printable: stocks.value.map(row => ({
      product_code: row.product_code ?? '-',
      product_name: row.product_name ?? '-',
      warehouse_name: row.warehouse_name ?? '-',
      batch_no: row.batch_no,
      color_no: row.color_no,
      dye_lot_no: row.dye_lot_no ?? '-',
      grade: row.grade,
      quantity_on_hand: formatNumber(Number(row.quantity_on_hand)),
      quantity_available: formatNumber(Number(row.quantity_available)),
      stock_status: getStockStatusLabel(row.stock_status, t),
      quality_status: row.quality_status,
      bin_location: row.bin_location ?? '-',
    })),
    properties: [
      { field: 'product_code', displayName: t('inventory.stockTab.colProductCode') },
      { field: 'product_name', displayName: t('inventory.stockTab.colProductName') },
      { field: 'warehouse_name', displayName: t('inventory.stockTab.colWarehouse') },
      { field: 'batch_no', displayName: t('inventory.stockTab.colBatchNo') },
      { field: 'color_no', displayName: t('inventory.stockTab.colColorCode') },
      { field: 'dye_lot_no', displayName: t('inventory.stockTab.colDyeLot') },
      { field: 'grade', displayName: t('inventory.stockTab.colGrade') },
      { field: 'quantity_on_hand', displayName: t('inventory.stockTab.colQuantity') },
      { field: 'quantity_available', displayName: t('inventory.stockTab.colAvailable') },
      { field: 'stock_status', displayName: t('inventory.stockTab.colStatus') },
      { field: 'quality_status', displayName: t('inventory.stockTab.colQualityStatus') },
      { field: 'bin_location', displayName: t('inventory.stockTab.colLocation') },
    ],
    type: 'json',
    header: t('inventory.printHeader'),
  });
};

const handleExport = async () => {
  if (stocks.value.length === 0) {
    ElMessage.warning(t('inventory.message.noExportData'));
    return;
  }
  const params: Record<string, unknown> = {
    warehouse_id: queryParams.warehouse_id,
    keyword: queryParams.keyword,
    stock_status: queryParams.stock_status,
  };
  await exportFromBackend('/inventory/stock/export', params, 'inventory_stock_export');
  ElMessage.success(t('inventory.message.exportSuccess'));
};

const initPage = () => {
  loadIfNot('fetchData', fetchData, hasLoaded);
  loadIfNot('fetchWarehouses', fetchWarehouses, hasLoaded);
  loadIfNot('fetchProducts', fetchProducts, hasLoaded);
};

onMounted(() => {
  initPage();
});
</script>

<style scoped>
.inventory-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 24px;
}

.header-left .page-title {
  font-size: 28px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 12px 0;
}

.stock-edit-hint {
  margin-bottom: 16px;
}

.header-actions {
  display: flex;
  gap: 12px;
}
</style>
