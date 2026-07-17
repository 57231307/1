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
        <h2>生产计划管理 (MRP)</h2>
        <p>管理和跟踪生产订单，制定和执行生产计划</p>
      </div>
    </el-card>

    <PrdFilter
      :form="prd.queryForm"
      @update:form="(v) => Object.assign(prd.queryForm, v)"
      @search="prd.applyQuery"
      @reset="prd.resetQuery"
    />

    <el-card class="table-card">
      <template #header>
        <div class="card-header">
          <span>生产订单列表</span>
          <div class="header-actions">
            <el-button type="primary" @click="openCreate">
              <el-icon><Plus /></el-icon>新建订单
            </el-button>
            <el-button @click="prdProc.handlePrint">
              <el-icon><Printer /></el-icon>打印
            </el-button>
            <el-button @click="prdProc.handleExport">
              <el-icon><Download /></el-icon>导出
            </el-button>
          </div>
        </div>
      </template>

      <PrdTbl
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
      />
    </el-card>

    <PrdForm
      v-model:visible="dialogVisible"
      :form="prd.orderForm"
      :loading="prd.submitLoading"
      :rules="prd.orderRules"
      @update:form="(v) => Object.assign(prd.orderForm, v)"
      @submit="onSubmitForm"
    />

    <PrdDetail v-model:visible="detailVisible" :order="currentOrder" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { Plus, Download, Printer } from '@element-plus/icons-vue'
import {
  createProductionOrder,
  updateProductionOrder,
  type ProductionOrder,
} from '@/api/production'
import { usePrd } from './composables/usePrd'
import { usePrdProc } from './composables/usePrdProc'
import PrdFilter from './components/PrdFilter.vue'
import PrdTbl from './components/PrdTbl.vue'
import PrdForm from './components/PrdForm.vue'
import PrdDetail from './components/PrdDetail.vue'

// 业务状态
const prd = usePrd()
// V15 P0-S12 修复（Batch 475c）：传入 getQueryParams，导出时传递列表筛选条件
const prdProc = usePrdProc({
  data: prd.data,
  refresh: prd.refresh,
  getQueryParams: () => ({
    status: prd.queryParams.status as string | undefined,
    product_id: prd.queryParams.product_id as number | undefined,
  }),
})

// 对话框状态
const dialogVisible = ref(false)
const detailVisible = ref(false)
const currentOrder = ref<ProductionOrder | null>(null)

/** 翻页 */
const onPageChange = (p: number) => {
  prd.page = p
}

/** 调整每页大小 */
const onSizeChange = (s: number) => {
  prd.pageSize = s
}

/** 打开新建对话框 */
const openCreate = () => {
  prd.resetOrderForm()
  dialogVisible.value = true
}

/** 打开编辑对话框 */
const onOpenEdit = (row: ProductionOrder) => {
  prd.resetOrderForm()
  Object.assign(prd.orderForm, row)
  dialogVisible.value = true
}

/** 查看详情 */
const onViewDetail = (row: ProductionOrder) => {
  currentOrder.value = row
  detailVisible.value = true
}

/** 提交表单（创建/更新） */
const onSubmitForm = async () => {
  prd.submitLoading = true
  try {
    if (!prd.orderForm.id) {
      await createProductionOrder(prd.orderForm as Partial<ProductionOrder>)
      ElMessage.success('创建成功')
    } else {
      await updateProductionOrder(prd.orderForm.id, prd.orderForm as Partial<ProductionOrder>)
      ElMessage.success('更新成功')
    }
    dialogVisible.value = false
    prd.resetOrderForm()
    await prd.refresh()
  } catch (e: unknown) {
    const err = e as { message?: string }
    ElMessage.error(err.message || '操作失败')
  } finally {
    prd.submitLoading = false
  }
}

onMounted(() => {
  prd.refresh()
})
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
