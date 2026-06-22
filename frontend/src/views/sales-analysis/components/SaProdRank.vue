<!--
  SaProdRank.vue - 产品销售排名表（按金额/数量切换）
  拆分自 sales-analysis/index.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover">
    <template #header>
      <div class="card-header">
        <span>产品销售排名</span>
        <el-select
          :model-value="type"
          size="small"
          style="width: 100px"
          @update:model-value="updateType"
        >
          <el-option label="按金额" value="amount" />
          <el-option label="按数量" value="quantity" />
        </el-select>
      </div>
    </template>
    <el-table :data="data" size="small">
      <el-table-column type="index" label="排名" width="60" align="center" />
      <el-table-column prop="product_name" label="产品名称" min-width="150" show-overflow-tooltip />
      <el-table-column prop="amount" label="销售额" width="120" align="right">
        <template #default="{ row }">
          {{ formatCurrency(row.amount) }}
        </template>
      </el-table-column>
      <el-table-column prop="quantity" label="销售数量" width="100" align="right" />
      <el-table-column prop="percentage" label="占比" width="80" align="center">
        <template #default="{ row }"> {{ row.percentage }}% </template>
      </el-table-column>
    </el-table>
  </el-card>
</template>

<script setup lang="ts">
import type { ProductRanking } from '@/api/sales-analysis'
import { formatCurrency } from '../composables/saFmts'

// 排名类型（v-model 通过 model-value + update:model-value 实现）
const emit = defineEmits<{ 'update:type': [v: string] }>()
defineProps<{
  data: ProductRanking[]
  type: string
}>()

const updateType = (v: string) => emit('update:type', v)
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
