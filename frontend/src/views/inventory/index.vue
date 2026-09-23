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
        <el-button v-permission="PERMISSIONS.INVENTORY_TRANSFER" @click="handleTransfer">
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

      <el-tab-pane :label="t('inventory.page.tabTransfer')" name="transfer">
        <InventoryTransferTab
          :transfers="transfers"
          @new-transfer="handleNewTransfer"
          @view-transfer="handleViewTransfer"
          @approve-transfer="handleApproveTransfer"
        />
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
      @submit="onSubmitAdjustment"
    />

    <!-- 库存记录新建/编辑对话框 -->
    <el-dialog
      v-model="stockDialogVisible"
      :title="stockEditingId ? t('inventory.stockTab.edit') : t('inventory.stockTab.create')"
      width="520px"
    >
      <el-form :model="stockForm" label-width="110px">
        <el-form-item :label="t('inventory.stockTab.colProductCode')">
          <el-input-number v-model="stockForm.product_id" :min="1" />
        </el-form-item>
        <el-form-item :label="t('inventory.stockTab.colWarehouse')">
          <el-select v-model="stockForm.warehouse_id" filterable>
            <el-option
              v-for="wh in warehouses"
              :key="wh.id"
              :label="wh.warehouse_name"
              :value="wh.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('inventory.stockTab.colBatchNo')">
          <el-input v-model="stockForm.batch_no" />
        </el-form-item>
        <el-form-item :label="t('inventory.stockTab.colColorCode')">
          <el-input v-model="stockForm.color_code" />
        </el-form-item>
        <el-form-item :label="t('inventory.stockTab.colLocation')">
          <el-input v-model="stockForm.location" />
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

    <TransferDialog
      v-model:visible="transferDialogVisible"
      :initial-form="transferForm"
      :warehouses="warehouses"
      @add-item="handleAddTransferItem"
      @remove-item="handleRemoveTransferItem"
      @submit="onSubmitTransfer"
    />
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
// V15 P0-S12 修复（Batch 475c）：导出改用后端带水印 xlsx 接口
// 后端 GET /inventory/stock/export 已就绪（含字段级数据权限 + 异步审计日志 + 水印）
import { exportFromBackend } from '@/utils/export';
// v11 批次 160 P2-7 修复：导入具体接口类型替代 any[]
import type { InventoryStock, StockAlert, InventoryTransfer, TransferData } from '@/api/inventory';
import type { Warehouse } from '@/api/warehouse';
import InventoryStockTab, { type StockQuery } from './tabs/InventoryStockTab.vue';
import InventoryAlertTab from './tabs/InventoryAlertTab.vue';
import InventoryTransferTab from './tabs/InventoryTransferTab.vue';
import InventoryTransactionTab from './tabs/InventoryTransactionTab.vue';
import InventoryReservationTab from './tabs/InventoryReservationTab.vue';
import FabricStockTab from './tabs/FabricStockTab.vue';
import StatCards from './components/StatCards.vue';
import AdjustmentDialog, { type AdjustmentForm } from './components/AdjustmentDialog.vue';
import TransferDialog from './components/TransferDialog.vue';
// Batch 468 P0-S28：引入权限码常量，与后端 inventory 资源对齐
import { PERMISSIONS } from '@/constants/permissions';
// 打印与列表共用同一份格式化/取值映射，避免同一状态在纸上和表里两种写法
import { formatNumber, getStockStatusLabel } from './composables/invFmts';
import { logger } from '@/utils/logger';

const hasLoaded = createLazyLoader();
const router = useRouter();

const loading = ref(false);
const activeTab = ref('stock');
// v11 批次 160 P2-7 修复：4 个核心状态从 any[] 改为具体接口类型，恢复类型保护
const stocks = ref<InventoryStock[]>([]);
const alerts = ref<StockAlert[]>([]);
const transfers = ref<InventoryTransfer[]>([]);
const warehouses = ref<Warehouse[]>([]);
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
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
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
    // 预警 tab 无分页控件：按一屏上限取数，仍有剩余时显式提示截断，而不是静默少显示
    const res = await getStockAlertList({ page: 1, page_size: STOCK_ALERT_PAGE_SIZE });
    const payload = res.data;
    if (!payload || !Array.isArray(payload.items)) {
      // 出参形状不符（此前正是这里把 {list,total} 当数组赋值，表格恒空且无人出声）
      throw new Error('库存预警出参缺少 items 数组');
    }
    alerts.value = payload.items;
    // KPI 取后端分层 total（全局值），不是本页行数；与下方"仅显示前 N 条"的截断提示不冲突
    stats.value.alertCount = payload.total;
    if (payload.items.length < payload.total) {
      logger.warn(
        `库存预警仅显示前 ${payload.items.length} 条（共 ${payload.total} 条），请按仓库/产品缩小范围查看剩余告警`
      );
    }
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) ||
        t('inventory.message.fetchAlertFailed')
    );
    alerts.value = [];
  }
};

const fetchTransfers = async () => {
  try {
    const { getInventoryTransferList } = await import('@/api/inventory');
    const res = await getInventoryTransferList(queryParams);
    transfers.value = res.data;
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) ||
        t('inventory.message.fetchTransferFailed')
    );
    transfers.value = [];
  }
};

const fetchWarehouses = async () => {
  try {
    const { getWarehouseList } = await import('@/api/warehouse');
    const res = await getWarehouseList({ page: 1, page_size: 1000 });
    warehouses.value = res.data?.items || [];
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) ||
        t('inventory.message.fetchWarehouseFailed')
    );
    warehouses.value = [];
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
  } else if (tabName === 'transfer') {
    fetchTransfers();
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

const transferDialogVisible = ref(false);
const transferForm = ref({
  from_warehouse_id: null as number | null,
  to_warehouse_id: null as number | null,
  items: [{ product_id: null as number | null, quantity: 0 }],
  remark: '',
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

// v11 批次 164 P2-1 修复：form: any 改为具体类型
const onSubmitAdjustment = async (form: AdjustmentForm) => {
  if (!form.adjustment_quantity || form.adjustment_quantity <= 0) {
    ElMessage.warning(t('inventory.message.adjustmentQtyInvalid'));
    return;
  }
  if (!form.reason) {
    ElMessage.warning(t('inventory.message.reasonRequired'));
    return;
  }
  try {
    const { createStockAdjustment } = await import('@/api/inventory');
    await createStockAdjustment({
      warehouse_id: form.warehouse_id!,
      product_id: form.product_id!,
      adjustment_type: form.adjustment_type,
      adjustment_quantity: form.adjustment_quantity,
      reason: form.reason,
    });
    ElMessage.success(t('inventory.message.adjustmentSuccess'));
    adjustmentDialogVisible.value = false;
    fetchData();
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) ||
        t('inventory.message.adjustmentFailed')
    );
  }
};

const handleTransfer = () => {
  transferForm.value = {
    from_warehouse_id: null,
    to_warehouse_id: null,
    items: [{ product_id: null, quantity: 0 }],
    remark: '',
  };
  transferDialogVisible.value = true;
};

const handleAddTransferItem = () => {
  transferForm.value.items.push({ product_id: null, quantity: 0 });
};
const handleRemoveTransferItem = (index: number) => {
  if (transferForm.value.items.length > 1) {
    transferForm.value.items.splice(index, 1);
  }
};
const onSubmitTransfer = async (form: typeof transferForm.value) => {
  if (!form.from_warehouse_id || !form.to_warehouse_id) {
    ElMessage.warning(t('inventory.message.warehouseRequired'));
    return;
  }
  try {
    const { createInventoryTransfer } = await import('@/api/inventory');
    const transferData: TransferData = {
      from_warehouse_id: form.from_warehouse_id,
      to_warehouse_id: form.to_warehouse_id,
      items: form.items
        .filter(item => item.product_id !== null)
        .map(item => ({
          product_id: item.product_id as number,
          quantity: item.quantity,
        })),
      remark: form.remark,
    };
    await createInventoryTransfer(transferData);
    ElMessage.success(t('inventory.message.transferCreated'));
    transferDialogVisible.value = false;
    if (activeTab.value === 'transfer') {
      fetchTransfers();
    }
  } catch (error: unknown) {
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) ||
        t('inventory.message.transferCreateFailed')
    );
  }
};

const handleNewTransfer = () => handleTransfer();
// 批次 157a P1-1 修复：调拨单详情无独立 API，直接展示列表行数据
const handleViewTransfer = (row: InventoryTransfer) => {
  const lines = [
    t('inventory.transferDetail.transferNo', { value: row.transfer_no }),
    t('inventory.transferDetail.fromWarehouse', { value: row.from_warehouse_name || '-' }),
    t('inventory.transferDetail.toWarehouse', { value: row.to_warehouse_name || '-' }),
    t('inventory.transferDetail.totalQty', { value: row.total_quantity }),
    t('inventory.transferDetail.status', { value: row.status }),
    t('inventory.transferDetail.creator', { value: row.creator_name || '-' }),
    t('inventory.transferDetail.createdAt', { value: row.created_at }),
  ];
  ElMessageBox.alert(lines.join('\n'), t('inventory.transferDetail.title'), {
    confirmButtonText: t('inventory.transferDetail.close'),
  });
};
// 批次 157a P1-1 修复：接入 approveTransfer API 完成调拨审批
const handleApproveTransfer = async (row: InventoryTransfer) => {
  try {
    await ElMessageBox.confirm(
      t('inventory.message.approveConfirm', { no: row.transfer_no }),
      t('inventory.message.approveTitle'),
      { type: 'info' }
    );
    const { approveInventoryTransfer } = await import('@/api/inventory');
    await approveInventoryTransfer(row.id, { approved: true });
    ElMessage.success(t('inventory.message.approveSuccess'));
    fetchTransfers();
  } catch (error) {
    if (error !== 'cancel') {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      ElMessage.error(
        (error instanceof Error ? error.message : String(error)) ||
          t('inventory.message.approveFailed')
      );
    }
  }
};
// 库存记录新建/编辑对话框（createStock / updateStock）
const stockDialogVisible = ref(false);
const stockEditingId = ref<number | null>(null);
const stockSubmitLoading = ref(false);
const stockForm = reactive({
  product_id: undefined as number | undefined,
  warehouse_id: undefined as number | undefined,
  batch_no: '',
  color_code: '',
  location: '',
  quantity: 0,
});

const openStockDialog = (row?: InventoryStock) => {
  stockEditingId.value = row ? row.id : null;
  stockForm.product_id = row?.product_id;
  stockForm.warehouse_id = row?.warehouse_id;
  stockForm.batch_no = row?.batch_no || '';
  stockForm.color_code = row?.color_no || '';
  stockForm.location = row?.bin_location || '';
  stockForm.quantity = Number(row?.quantity_meters ?? 0);
  stockDialogVisible.value = true;
};

const submitStock = async () => {
  if (!stockForm.product_id || !stockForm.warehouse_id) {
    ElMessage.warning(t('inventory.message.productWarehouseRequired'));
    return;
  }
  stockSubmitLoading.value = true;
  try {
    if (stockEditingId.value) {
      const { updateStock } = await import('@/api/inventory');
      await updateStock(stockEditingId.value, stockForm);
    } else {
      const { createStock } = await import('@/api/inventory');
      // 按后端 CreateStockFabricRequest DTO 字段提交：色号/数量映射到 color_no/quantity_meters
      await createStock({
        warehouse_id: stockForm.warehouse_id,
        product_id: stockForm.product_id,
        batch_no: stockForm.batch_no,
        color_no: stockForm.color_code,
        grade: 'A',
        quantity_meters: stockForm.quantity,
      });
    }
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

// 批次 157a P1-1 修复：接入 getStockById API 展示库存详情
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
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) ||
        t('inventory.message.fetchStockDetailFailed')
    );
  }
};
// 批次 157b P1-1 修复：采购按钮跳转到采购页面
const handlePurchase = (row: StockAlert) => {
  router.push({ name: 'Purchase', query: { product_name: row.product_name || '' } });
};
// 打印列与列表/后端导出同一口径。
// 原实现用 `properties: [..., 'quantity']`：quantity 不是后端字段（实际为 quantity_on_hand），
// 该列在纸上恒为空白，四维（批次/色号/缸号/等级）与状态列也没打出来；
// 表头也只会回显英文字段名。现显式给出列与本地化表头，状态按主数据取值映射文案。
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
    // print-js 的列定义项叫 properties（json 模式），不存在的键会被静默忽略
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
// 批次 157b P1-1 修复：导出改为 .xls 格式（规则 3：禁止 CSV 作为最终交付格式）
// V15 P0-S12 修复（Batch 475c）：导出改用后端带水印 xlsx 接口
// 调用后端 GET /inventory/stock/export，传入当前列表筛选条件（warehouse_id），
// 保证导出数据与列表筛选一致；后端注入水印 + 字段级数据权限 + 异步审计日志
const handleExport = async () => {
  if (stocks.value.length === 0) {
    ElMessage.warning(t('inventory.message.noExportData'));
    return;
  }
  // 导出与列表同一筛选口径：只带 warehouse_id 时，关键词与台账状态筛选在导出文件里失效
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

.header-actions {
  display: flex;
  gap: 12px;
}
</style>
