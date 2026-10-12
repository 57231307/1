<!--
  production/index.vue - 生产计划管理（拆分重构版）
  任务编号: P14 批 2 I-3 第 4 批
  拆分：611 行 → ~150 行 + 4 子组件 + 2 composable + 1 工具
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="production-container">
    <el-card class="header-card">
      <div class="header-content">
        <h2>{{ t('production.index.headerTitle') }}</h2>
        <p>{{ t('production.index.headerSubtitle') }}</p>
      </div>
    </el-card>

    <ProductionFilter
      :form="prd.queryForm"
      @update:form="v => Object.assign(prd.queryForm, v)"
      @search="prd.applyQuery"
      @reset="prd.resetQuery"
    />

    <el-card class="table-card">
      <template #header>
        <div class="card-header">
          <span>{{ t('production.index.cardHeader') }}</span>
          <div class="header-actions">
            <el-button type="primary" @click="openCreate">
              <el-icon><Plus /></el-icon>{{ t('production.index.buttonCreate') }}
            </el-button>
            <el-button @click="prdProc.handlePrint">
              <el-icon><Printer /></el-icon>{{ t('production.index.buttonPrint') }}
            </el-button>
            <el-button @click="prdProc.handleExport">
              <el-icon><Download /></el-icon>{{ t('production.index.buttonExport') }}
            </el-button>
          </div>
        </div>
      </template>

      <ProductionTable
        :data="prd.data"
        :loading="prd.loading"
        :page="prd.page"
        :page-size="prd.pageSize"
        :total="prd.total"
        @page-change="onPageChange"
        @size-change="onSizeChange"
        @view-detail="onViewDetail"
        @open-edit="onOpenEdit"
        @status-change="prdProc.handleStatusChange"
        @delete="prdProc.handleDelete"
        @submit-approval="prdProc.handleSubmitForApproval"
        @approve-order="prdProc.handleApproveOrder"
        @report-progress="prdProc.handleProgressReport"
        @view-logs="prdProc.handleViewLogs"
      />
    </el-card>

    <ProductionForm
      v-model:visible="dialogVisible"
      :form="prd.orderForm"
      :loading="prd.submitLoading"
      :rules="prd.orderRules"
      @update:form="v => Object.assign(prd.orderForm, v)"
      @submit="onSubmitForm"
    />

    <ProductionDetail v-model:visible="detailVisible" :order="currentOrder" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { Plus, Download, Printer } from '@element-plus/icons-vue';
import {
  createProductionOrder,
  updateProductionOrder,
  type CreateProductionOrderPayload,
  type ProductionOrder,
  type UpdateProductionOrderPayload,
} from '@/api/production';
import { usePrd } from './composables/usePrd';
import { usePrdProc } from './composables/usePrdProc';
import ProductionFilter from './components/ProductionFilter.vue';
import ProductionTable from './components/ProductionTable.vue';
import ProductionForm from './components/ProductionForm.vue';
import ProductionDetail from './components/ProductionDetail.vue';

const { t } = useI18n({ useScope: 'global' });

// 业务状态
const prd = usePrd();
const prdProc = usePrdProc({
  data: prd.data,
  refresh: prd.refresh,
  getQueryParams: () => ({
    status: prd.queryParams.status as string | undefined,
    product_id: prd.queryParams.product_id as number | undefined,
  }),
});

// 对话框状态
const dialogVisible = ref(false);
const detailVisible = ref(false);
const currentOrder = ref<ProductionOrder | null>(null);

/** 翻页 */
const onPageChange = (p: number) => {
  prd.page = p;
};

/** 调整每页大小 */
const onSizeChange = (s: number) => {
  prd.pageSize = s;
};

/** 打开新建对话框 */
const openCreate = () => {
  prd.resetOrderForm();
  dialogVisible.value = true;
};

/** 打开编辑对话框 */
const onOpenEdit = (row: ProductionOrder) => {
  prd.resetOrderForm();
  Object.assign(prd.orderForm, row);
  dialogVisible.value = true;
};

/** 查看详情 */
const onViewDetail = (row: ProductionOrder) => {
  currentOrder.value = row;
  detailVisible.value = true;
};

/** 提交表单（创建/更新） */
const onSubmitForm = async () => {
  prd.submitLoading = true;
  try {
    if (!prd.orderForm.id) {
      // 缺陷3：单据号系统生成禁手打——创建剔除 order_no。
      // 空串 '' 序列化后会成为 Some("")，后端 resolve_order_no 会把 "" 当作手输单号
      // 直插（绕过统一取号器）；undefined 经 JSON 序列化剔除 → 后端走
      // utils/number_generator.rs 取号（PO{YYYYMMDD}{3位流水}）。
      const payload = { ...prd.orderForm, order_no: undefined } as CreateProductionOrderPayload;
      await createProductionOrder(payload);
      ElMessage.success(t('production.index.messageCreateSuccess'));
    } else {
      // 三态语义（后端 UpdateProductionOrderPayload DoubleOption）：
      // 编辑对话框已回显原值；可空列（planned_start_date/planned_end_date/work_center_id/remarks）
      // UI 清空 ⇒ 送显式 null（=清空为 NULL），未改动 ⇒ 原值回传；
      // NOT NULL 列 planned_quantity/priority 恒送值（送 null 会被后端拒绝）
      const payload: UpdateProductionOrderPayload = {
        planned_quantity: prd.orderForm.planned_quantity,
        priority: prd.orderForm.priority,
        planned_start_date: prd.orderForm.planned_start_date || null,
        planned_end_date: prd.orderForm.planned_end_date || null,
        work_center_id: prd.orderForm.work_center_id ?? null,
        remarks: prd.orderForm.remarks || null,
      };
      await updateProductionOrder(prd.orderForm.id, payload);
      ElMessage.success(t('production.index.messageUpdateSuccess'));
    }
    dialogVisible.value = false;
    prd.resetOrderForm();
    await prd.refresh();
  } catch (e: unknown) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('production.index.messageOperationFailed'));
  } finally {
    prd.submitLoading = false;
  }
};

onMounted(() => {
  prd.refresh();
});
</script>

<style scoped>
.production-container {
  padding: 20px;
}
.header-card {
  margin-bottom: 16px;
}
.header-content h2 {
  margin: 0 0 4px 0;
  font-size: 22px;
}
.header-content p {
  margin: 0;
  color: #909399;
  font-size: 13px;
}
.table-card {
  margin-top: 16px;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.header-actions {
  display: flex;
  gap: 8px;
}
</style>
