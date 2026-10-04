<!--
  SalesPriceTable.vue - 销售价格列表表格
  拆分自 sales-price/index.vue（P14 批 2 I-3 第 3 批）
  批次 284：接入 useTableApi 模式（page/pageSize props + v-model 绑定分页）
-->
<template>
  <el-card shadow="hover" class="table-card">
    <el-table
      v-loading="loading"
      :data="priceList"
      border
      stripe
      :aria-label="t('salesPrice.table.ariaLabel')"
    >
      <el-table-column
        type="index"
        :label="t('salesPrice.table.columnIndex')"
        width="60"
        align="center"
      />
      <!--
        此处原有"产品名称/客户名称"两列（prop=product_name / customer_name），因后端销售价目
        读链路从不输出这两个键而删除：sales_price::Model（models/sales_price.rs）无
        product_name/product_code/customer_name/remark 列；sales_price_handler.rs 的
        list_prices/get_price/create_price/update_price/get_price_history/list_strategies
        全部直接序列化 Model，sales_price_service.rs::get_prices_list 以 Entity::find() 为主体，
        keyword 筛选仅 LeftJoin(product/customer) 做谓词、不追加 SELECT 列（响应仍是整 Model）；
        信封（utils/response.rs::ApiResponse/PaginatedResponse）只补
        code/data/message/total 与 items/total/page/page_size ⇒ 两列自表存在起恒空白（幽灵列）。
        purchase 侧同名字段是真实的：purchase_price_service.rs::get_prices_list 以
        column_as+LEFT JOIN 产出 PurchasePriceView（product_name/product_code/supplier_name），
        两域接口此前同构抄写即漂移根源。列表因此不再能标识产品/客户，补齐方案
        （后端加 JOIN 输出名列，或改显示 product_id/customer_id）属产品语义决策，
        已交回主编排，当前为待决策项；禁止前端造名填充。
      -->
      <el-table-column
        prop="price"
        :label="t('salesPrice.table.columnPrice')"
        width="120"
        align="right"
      >
        <template #default="{ row }">
          {{ formatCurrency(row.price) }}
        </template>
      </el-table-column>
      <el-table-column
        prop="currency"
        :label="t('salesPrice.table.columnCurrency')"
        width="80"
        align="center"
      />
      <el-table-column
        prop="unit"
        :label="t('salesPrice.table.columnUnit')"
        width="80"
        align="center"
      />
      <el-table-column
        prop="min_order_qty"
        :label="t('salesPrice.table.columnMinOrderQty')"
        width="100"
        align="right"
      />
      <el-table-column
        prop="price_type"
        :label="t('salesPrice.table.columnPriceType')"
        width="100"
        align="center"
      >
        <template #default="{ row }">
          <el-tag>{{ getPriceTypeLabel(row.price_type) }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column
        prop="price_level"
        :label="t('salesPrice.table.columnPriceLevel')"
        width="100"
        align="center"
      />
      <el-table-column
        prop="effective_date"
        :label="t('salesPrice.table.columnEffectiveDate')"
        width="120"
        align="center"
      />
      <el-table-column
        prop="expiry_date"
        :label="t('salesPrice.table.columnExpiryDate')"
        width="120"
        align="center"
      />
      <el-table-column
        prop="status"
        :label="t('salesPrice.table.columnStatus')"
        width="100"
        align="center"
      >
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column
        :label="t('salesPrice.table.columnAction')"
        width="200"
        align="center"
        fixed="right"
      >
        <template #default="{ row }">
          <el-button type="primary" link size="small" @click="emit('view', row as SalesPrice)">{{
            t('salesPrice.table.buttonView')
          }}</el-button>
          <!-- P2-17 修复（批次 86 v2 复审）：编辑按钮补齐 v-permission -->
          <el-button
            v-if="row.status === 'pending'"
            v-permission="PERMISSIONS.SALES_PRICE_UPDATE"
            type="primary"
            link
            size="small"
            @click="emit('edit', row as SalesPrice)"
            >{{ t('salesPrice.table.buttonEdit') }}</el-button
          >
          <el-button
            v-if="row.status === 'pending'"
            type="success"
            link
            size="small"
            @click="emit('approve', row as SalesPrice)"
            >{{ t('salesPrice.table.buttonApprove') }}</el-button
          >
          <el-button type="info" link size="small" @click="emit('history', row as SalesPrice)">{{
            t('salesPrice.table.buttonHistory')
          }}</el-button>
        </template>
      </el-table-column>
    </el-table>

    <div class="pagination-container">
      <el-pagination
        :current-page="page"
        :page-size="pageSize"
        :page-sizes="[10, 20, 50, 100]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        :aria-label="t('salesPrice.table.paginationAriaLabel')"
        @update:current-page="(v: number) => emit('update:page', v)"
        @update:page-size="(v: number) => emit('update:page-size', v)"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { SalesPrice } from '@/api/sales-price';
// 价格类型/状态文案统一走 spFmts（i18n 键 + 未知 token 抛错；词表权威 models/status/sales.rs::price_approval），删除组件局部 map
import {
  formatCurrency,
  getStatusType,
  getStatusLabel,
  getPriceTypeLabel,
} from '../composables/spFmts';
// Batch 462 P0-S24：引入权限码常量，与后端 sales-prices 资源对齐
import { PERMISSIONS } from '@/constants/permissions';

const { t } = useI18n({ useScope: 'global' });

/**
 * 销售价格列表表格组件（批次 284：page/pageSize props + v-model 绑定分页）
 */
defineProps<{
  // 列表数据
  priceList: SalesPrice[];
  // 加载状态
  loading: boolean;
  // 总数
  total: number;
  // 当前页
  page: number;
  // 每页条数
  pageSize: number;
}>();

const emit = defineEmits<{
  view: [row: SalesPrice];
  edit: [row: SalesPrice];
  approve: [row: SalesPrice];
  history: [row: SalesPrice];
  'update:page': [v: number];
  'update:page-size': [v: number];
}>();
</script>

<style scoped>
.table-card {
  margin-bottom: 20px;
}
.pagination-container {
  display: flex;
  justify-content: flex-end;
  margin-top: 20px;
}
</style>
