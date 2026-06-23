<!--
  sales/views/OrderListView.vue - 销售订单列表（拆分重构版）
  任务编号: P14 批 2 I-3 第 3 批
  拆分：644 行 → ~150 行 + 3 子组件 + 2 composable + 1 工具
  行为完全保持一致（仅结构重构）
  说明：本文件保留原路径 views/（B3-1 拆分时已迁入），对话框子组件保留 OrderFormDialog / OrderViewDialog / DeliveryDialog
-->
<template>
  <div class="sales-page">
    <div class="page-header">
      <h2 class="page-title">销售订单管理</h2>
      <el-button type="primary" @click="onCreate">
        <el-icon><Plus /></el-icon> 新建订单
      </el-button>
    </div>

    <OlvStat :stats="olv.stats" />

    <OlvFilter
      :filter-form="olv.filterForm"
      @query="olv.handleQuery"
      @reset="olv.handleReset"
      @update:filter-form="(v) => Object.assign(olv.filterForm, v)"
    />

    <OlvTbl
      :columns="olv.columns"
      :data="olv.data"
      :loading="olv.loading"
      :page="olv.page"
      :page-size="olv.pageSize"
      :total="olv.total"
      @page-change="olv.handlePageChange"
      @size-change="olv.handleSizeChange"
      @view="onView"
      @approve="olvProc.handleApprove"
      @delivery="onDelivery"
      @cancel="olvProc.handleCancel"
    />

    <!-- 拆分后的对话框子组件 -->
    <OrderFormDialog
      v-model:visible="formDialogVisible"
      :title="olv.formDialogTitle"
      :form-data="olv.formData"
      :customers="olv.customers"
      :products="olv.products"
      :submitting="olv.submitting"
      @submit="onFormSubmit"
    />

    <OrderViewDialog v-model:visible="olv.viewDialogVisible" :order="olv.currentOrder" />

    <DeliveryDialog
      v-model:visible="olv.deliveryDialogVisible"
      :form="olv.deliveryForm"
      :warehouses="olv.warehouses"
      @update:form="(v) => Object.assign(olv.deliveryForm, v)"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { Plus } from '@element-plus/icons-vue'
import type { SalesOrder } from '@/api/sales'
import { useOlv } from '../composables/useOlv'
import { useOlvProc } from '../composables/useOlvProc'
import OlvStat from '../components/OlvStat.vue'
import OlvFilter from '../components/OlvFilter.vue'
import OlvTbl from '../components/OlvTbl.vue'
import OrderFormDialog from '../OrderFormDialog.vue'
import OrderViewDialog from '../OrderViewDialog.vue'
import DeliveryDialog from '../DeliveryDialog.vue'

const olv = useOlv()
const olvProc = useOlvProc({
  refresh: olv.refresh,
})

// 表单对话框可见性本地 ref
const formDialogVisible = ref(false)

/** 新建订单 */
const onCreate = () => {
  olv.prepareCreate()
  formDialogVisible.value = true
}

/** 查看详情 */
const onView = (row: SalesOrder) => {
  olv.currentOrder = row
  olv.viewDialogVisible = true
}

/** 打开发货对话框 */
const onDelivery = (row: SalesOrder) => {
  olv.prepareDelivery(row)
  olv.deliveryDialogVisible = true
}

/** 提交订单表单 */
const onFormSubmit = async () => {
  const ok = await olvProc.handleFormSubmit(olv.formData)
  if (ok) formDialogVisible.value = false
}

onMounted(() => {
  olv.initLoad()
})
</script>

<style scoped>
.sales-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
.page-title {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}
</style>
