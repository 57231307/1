<!--
  purchase-return/index.vue - 采购退货（拆分重构版）
  任务编号: P14 批 2 I-3 第 2 批
  拆分：695 行 → ~150 行 + 5 子组件 + 2 composable + 1 工具
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="purchase-return">
    <div class="page-header">
      <h2>采购退货</h2>
      <el-button type="primary" @click="onCreate">
        <el-icon><Plus /></el-icon>
        新建退货单
      </el-button>
    </div>

    <!-- 统计卡片 -->
    <el-row :gutter="20" class="stats-row">
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="stat-item">
            <div class="stat-label">总退货单数</div>
            <div class="stat-value">{{ prRtn.stats.total || 0 }}</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="stat-item">
            <div class="stat-label">待审批</div>
            <div class="stat-value text-warning">{{ prRtn.stats.pending || 0 }}</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="stat-item">
            <div class="stat-label">已审批</div>
            <div class="stat-value text-success">{{ prRtn.stats.approved || 0 }}</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="stat-item">
            <div class="stat-label">退货金额</div>
            <div class="stat-value text-danger">¥{{ prRtn.stats.amount || 0 }}</div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <PrRtnFilter
      :query-params="prRtn.queryParams"
      :suppliers="prRtn.suppliers"
      :date-range="prRtn.dateRange"
      @query="prRtn.handleQuery"
      @reset="prRtn.handleReset"
      @date-change="onDateChange"
      @update:query-params="(v) => Object.assign(prRtn.queryParams, v)"
    />

    <PrRtnTbl
      :table-data="prRtn.tableData"
      :loading="prRtn.loading"
      :total="prRtn.total"
      :query-params="prRtn.queryParams"
      @view="onView"
      @edit="onEdit"
      @submit="prRtnProc.handleSubmit"
      @approve="prRtnProc.openApprove"
      @delete="prRtnProc.handleDelete"
      @size-change="prRtn.handleSizeChange"
      @current-change="prRtn.handleCurrentChange"
    />

    <PrRtnForm
      v-model:visible="dialogVisible"
      :is-edit="isEdit"
      :form-data="prRtn.formData"
      :form-rules="prRtn.formRules"
      :purchase-orders="prRtn.purchaseOrders"
      :products="prRtn.products"
      @submit="onSubmitForm"
      @order-change="prRtn.handleOrderChange"
      @product-change="prRtn.handleProductChange"
      @add-item="prRtn.handleAddItem"
      @remove-item="prRtn.handleRemoveItem"
      @update:form-data="(v) => Object.assign(prRtn.formData, v)"
    />

    <PrRtnDetail
      v-model:visible="detailDialogVisible"
      :detail-data="prRtn.detailData"
    />

    <PrRtnApr
      v-model:visible="prRtnProc.approveDialogVisible"
      :approve-form="prRtnProc.approveForm"
      @approve-confirm="prRtnProc.handleApproveConfirm"
      @reject="prRtnProc.handleReject"
      @update:approve-form="(v) => Object.assign(prRtnProc.approveForm, v)"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { Plus } from '@element-plus/icons-vue'
import type { PurchaseReturn } from '@/api/purchase-return'
import { usePrRtn } from './composables/usePrRtn'
import { usePrRtnProc } from './composables/usePrRtnProc'
import PrRtnFilter from './components/PrRtnFilter.vue'
import PrRtnTbl from './components/PrRtnTbl.vue'
import PrRtnForm from './components/PrRtnForm.vue'
import PrRtnDetail from './components/PrRtnDetail.vue'
import PrRtnApr from './components/PrRtnApr.vue'

const prRtn = usePrRtn()
const prRtnProc = usePrRtnProc({
  fetchData: prRtn.fetchData,
})

// 对话框可见性
const dialogVisible = ref(false)
const isEdit = ref(false)
const detailDialogVisible = ref(false)

/** 新建 */
const onCreate = () => {
  isEdit.value = false
  prRtn.prepareCreate()
  dialogVisible.value = true
}

/** 编辑 */
const onEdit = (row: PurchaseReturn) => {
  isEdit.value = true
  prRtn.prepareEdit(row)
  dialogVisible.value = true
}

/** 提交表单 */
const onSubmitForm = async () => {
  const ok = await prRtn.handleFormSubmit(isEdit.value)
  if (ok) dialogVisible.value = false
}

/** 查看详情 */
const onView = async (row: PurchaseReturn) => {
  await prRtn.fetchDetail(row.id!)
  detailDialogVisible.value = true
}

/** 日期范围变化 */
const onDateChange = (v: [Date, Date] | null) => {
  prRtn.dateRange = v
  prRtn.handleQuery()
}

onMounted(() => {
  prRtn.initLoad()
})
</script>

<style scoped>
.purchase-return {
  padding: 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.page-header h2 {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
}

.stats-row {
  margin-bottom: 20px;
}

.stat-item {
  text-align: center;
}

.stat-label {
  font-size: 14px;
  color: #909399;
  margin-bottom: 10px;
}

.stat-value {
  font-size: 28px;
  font-weight: 600;
}

.text-warning {
  color: #e6a23c;
}

.text-success {
  color: #67c23a;
}

.text-danger {
  color: #f56c6c;
}
</style>
