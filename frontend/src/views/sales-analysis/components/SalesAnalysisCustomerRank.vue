<!--
  SalesAnalysisCustomerRank.vue - 客户销售排名表（按金额/订单数切换）
  拆分自 sales-analysis/index.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover">
    <template #header>
      <div class="card-header">
        <span>{{ t('salesAnalysis.customerRank.cardTitle') }}</span>
        <el-select
          :model-value="type"
          size="small"
          style="width: 100px"
          @update:model-value="updateType"
        >
          <el-option :label="t('salesAnalysis.customerRank.optionByAmount')" value="amount" />
          <el-option :label="t('salesAnalysis.customerRank.optionByOrders')" value="orders" />
        </el-select>
      </div>
    </template>
    <el-table :data="data" size="small" :aria-label="t('salesAnalysis.customerRank.ariaLabelList')">
      <el-table-column
        type="index"
        :label="t('salesAnalysis.customerRank.columnRank')"
        width="60"
        align="center"
      />
      <el-table-column
        prop="customer_name"
        :label="t('salesAnalysis.customerRank.columnCustomerName')"
        min-width="150"
        show-overflow-tooltip
      />
      <el-table-column
        prop="amount"
        :label="t('salesAnalysis.customerRank.columnAmount')"
        width="120"
        align="right"
      >
        <template #default="{ row }">
          {{ formatCurrency(row.amount) }}
        </template>
      </el-table-column>
      <el-table-column
        prop="order_count"
        :label="t('salesAnalysis.customerRank.columnOrderCount')"
        width="80"
        align="right"
      />
      <el-table-column
        prop="percentage"
        :label="t('salesAnalysis.customerRank.columnPercentage')"
        width="80"
        align="center"
      >
        <template #default="{ row }"> {{ row.percentage }}% </template>
      </el-table-column>
    </el-table>
  </el-card>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { CustomerRanking } from '@/api/sales-analysis';
import { formatCurrency } from '../composables/saFmts';

const { t } = useI18n({ useScope: 'global' });

// 排名类型（v-model 通过 model-value + update:model-value 实现）
const emit = defineEmits<{ 'update:type': [v: string] }>();
defineProps<{
  data: CustomerRanking[];
  type: string;
}>();

const updateType = (v: string) => emit('update:type', v);
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
