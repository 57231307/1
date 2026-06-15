<!--
  trading/index.vue - 交易管理主入口（容器组件）
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-1）：
  原 1034 行"上帝组件"已拆分为以下 5 个独立 Tab 子组件，
  位于 views/trading/tabs/ 目录：

  | Tab       | 子组件                              |
  | --------- | ----------------------------------- |
  | 采购合同  | tabs/PurchaseContractTab.vue        |
  | 采购价格  | tabs/PurchasePriceTab.vue           |
  | 销售合同  | tabs/SalesContractTab.vue           |
  | 销售价格  | tabs/SalesPriceTab.vue              |
  | 销售退货  | tabs/SalesReturnTab.vue             |

  本主入口仅承担：Tab 切换 + 公共样式。
-->
<template>
  <div class="trading-page">
    <el-tabs v-model="activeTab" @tab-change="(tab: string | number) => (activeTab = String(tab))">
      <el-tab-pane label="采购合同" name="purchase-contract">
        <PurchaseContractTab />
      </el-tab-pane>
      <el-tab-pane label="采购价格" name="purchase-price">
        <PurchasePriceTab />
      </el-tab-pane>
      <el-tab-pane label="销售合同" name="sales-contract">
        <SalesContractTab />
      </el-tab-pane>
      <el-tab-pane label="销售价格" name="sales-price">
        <SalesPriceTab />
      </el-tab-pane>
      <el-tab-pane label="销售退货" name="sales-return">
        <SalesReturnTab />
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import PurchaseContractTab from './tabs/PurchaseContractTab.vue'
import PurchasePriceTab from './tabs/PurchasePriceTab.vue'
import SalesContractTab from './tabs/SalesContractTab.vue'
import SalesPriceTab from './tabs/SalesPriceTab.vue'
import SalesReturnTab from './tabs/SalesReturnTab.vue'

const activeTab = ref('purchase-contract')
</script>

<style scoped>
.trading-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
</style>
