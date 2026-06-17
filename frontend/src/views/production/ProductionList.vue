<!--
  ProductionList.vue - 生产订单列表子组件
  来源：原 production/index.vue 中 列表+筛选区（line 11-64）
  拆分日期：2026-06-17 P1-3-Batch-6
-->
<template>
  <div>
    <el-card class="filter-card">
      <el-form :inline="true" :model="queryForm">
        <el-form-item label="订单编号">
          <el-input v-model="queryForm.order_no" placeholder="请输入订单编号" clearable />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="queryForm.status" placeholder="请选择状态" clearable>
            <el-option
              v-for="(item, key) in PRODUCTION_ORDER_STATUS"
              :key="key"
              :label="item.label"
              :value="key"
            />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="fetchOrders">查询</el-button>
          <el-button @click="resetQuery">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <!-- 操作区 -->
    <el-card class="table-card">
      <template #header>
        <div class="card-header">
          <span>生产订单列表</span>
          <div class="header-actions">
            <el-button type="primary" @click="openDialog('create')">
              <el-icon><Plus /></el-icon>新建订单
            </el-button>
            <el-button @click="handlePrint">
              <el-icon><Printer /></el-icon>打印
            </el-button>
            <el-button @click="handleExport">
              <el-icon><Download /></el-icon>导出
            </el-button>
          </div>
        </div>
      </template>

      <V2Table
        :data="orderList"
        :columns="productionColumns"
        :estimated-row-height="48"
        :loading="loading"
        :total="total"
        :page="queryForm.page"
        :page-size="queryForm.page_size"
        @row-click="handleProductionRowClick"
        @page-change="handlePageChange"
        @size-change="handleSizeChange"
      />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { Search, Plus } from '@element-plus/icons-vue'

defineProps<{
  orders: any[]
  total: number
  loading: boolean
  queryParams: any
  statusTypeMap: any
  statusMap: any
  priorityTypeMap: any
  priorityMap: any
}>()

const emit = defineEmits<{
  search: []
  'update:queryParams': [value: any]
  add: []
  view: [row: any]
  edit: [row: any]
  delete: [row: any]
  'audit': [row: any]
  'update-status': [row: any, status: string]
}>()
</script>
