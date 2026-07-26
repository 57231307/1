<!--
  SalesAnalysisProductRank.vue - 产品销售排名表（按金额/数量切换）
  拆分自 sales-analysis/index.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover">
    <template #header>
      <div class="card-header">
        <span>{{ t('salesAnalysis.productRank.cardTitle') }}</span>
        <el-select
          :model-value="type"
          size="small"
          style="width: 100px"
          @update:model-value="updateType"
        >
          <el-option :label="t('salesAnalysis.productRank.optionByAmount')" value="amount" />
          <el-option :label="t('salesAnalysis.productRank.optionByQuantity')" value="quantity" />
        </el-select>
      </div>
    </template>
    <el-table :data="data" size="small" :aria-label="t('salesAnalysis.productRank.ariaLabelList')">
      <el-table-column
        type="index"
        :label="t('salesAnalysis.productRank.columnRank')"
        width="60"
        align="center"
      />
      <el-table-column
        prop="product_name"
        :label="t('salesAnalysis.productRank.columnProductName')"
        min-width="150"
        show-overflow-tooltip
      />
      <el-table-column
        prop="amount"
        :label="t('salesAnalysis.productRank.columnAmount')"
        width="120"
        align="right"
      >
        <template #default="{ row }">
          {{ formatCurrency(row.amount) }}
        </template>
      </el-table-column>
      <el-table-column
        prop="quantity"
        :label="t('salesAnalysis.productRank.columnQuantity')"
        width="100"
        align="right"
      />
      <el-table-column
        prop="percentage"
        :label="t('salesAnalysis.productRank.columnPercentage')"
        width="80"
        align="center"
      >
        <template #default="{ row }"> {{ row.percentage }}% </template>
      </el-table-column>
    </el-table>
  </el-card>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { ProductRanking } from '@/api/sales-analysis'
import { formatCurrency } from '../composables/saFmts'

const { t } = useI18n({ useScope: 'global' })

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
