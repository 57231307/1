<!--
  PiTbl.vue - 采购验货列表
  拆分自 purchase-inspection/index.vue（P14 批 2 I-3 第 5 批）
  批次 286：page/pageSize props + v-model 绑定分页
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card class="table-card">
    <el-table v-loading="loading" :data="data" border stripe aria-label="采购验货列表">
      <el-table-column prop="inspection_no" label="检验单号" min-width="140" />
      <el-table-column prop="receipt_no" label="入库单号" min-width="140" />
      <el-table-column prop="supplier_name" label="供应商" min-width="150" />
      <el-table-column prop="inspection_date" label="检验日期" min-width="120" />
      <el-table-column prop="inspector_name" label="检验员" min-width="100" />
      <el-table-column prop="status" label="状态" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)">
            {{ getStatusText(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="result" label="检验结果" width="100" align="center">
        <template #default="{ row }">
          <el-tag v-if="row.result" :type="getResultType(row.result)">
            {{ getResultText(row.result) }}
          </el-tag>
          <span v-else>-</span>
        </template>
      </el-table-column>
      <el-table-column prop="remark" label="备注" min-width="150" show-overflow-tooltip />
      <el-table-column label="操作" width="200" fixed="right">
        <template #default="{ row }">
          <el-button size="small" @click="emit('view', row as PurchaseInspection)">查看</el-button>
          <el-button
            v-if="row.status === 'draft' || row.status === 'pending'"
            size="small"
            type="primary"
            @click="emit('edit', row as PurchaseInspection)"
          >
            编辑
          </el-button>
          <el-button
            v-if="row.status === 'pending'"
            size="small"
            type="success"
            @click="emit('complete', row as PurchaseInspection)"
          >
            完成
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <el-pagination
      :current-page="page"
      :page-size="pageSize"
      :total="total"
      :page-sizes="[10, 20, 50, 100]"
      layout="total, sizes, prev, pager, next, jumper"
      @update:current-page="(v: number) => emit('update:page', v)"
      @update:page-size="(v: number) => emit('update:page-size', v)"
      aria-label="采购验货列表分页"
    />
  </el-card>
</template>

<script setup lang="ts">
import { getStatusType, getStatusText, getResultType, getResultText } from '../composables/piFmts'
import type { PurchaseInspection } from '@/api/purchase-inspection'

/**
 * 列表组件（批次 286：page/pageSize props + v-model 绑定分页）
 */
defineProps<{
  // 列表数据
  data: PurchaseInspection[]
  // 总数
  total: number
  // 加载状态
  loading: boolean
  // 当前页
  page: number
  // 每页条数
  pageSize: number
}>()

const emit = defineEmits<{
  view: [row: PurchaseInspection]
  edit: [row: PurchaseInspection]
  complete: [row: PurchaseInspection]
  'update:page': [v: number]
  'update:page-size': [v: number]
}>()
</script>

<style scoped>
.table-card {
  margin-bottom: 20px;
}
.el-pagination {
  margin-top: 20px;
  justify-content: flex-end;
}
</style>
