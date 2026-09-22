<!--
  sales/OrderDetail.vue - 销售订单详情页
  供报价单转订单后的跳转查看（/sales/orders/:id），
  展示订单主信息 + 明细行，返回按钮回销售列表
-->
<template>
  <div class="order-detail-page">
    <div class="page-header">
      <h2 class="page-title">{{ t('sales.orderDetail.title') }}</h2>
      <el-button @click="router.push('/sales')">{{ t('sales.orderDetail.back') }}</el-button>
    </div>

    <el-card v-loading="loading">
      <el-descriptions :column="3" border>
        <el-descriptions-item :label="t('sales.orderDetail.orderNo')">
          {{ order?.order_no }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('sales.orderDetail.customer')">
          {{ order?.customer_name }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('sales.orderDetail.status')">
          <el-tag size="small" :type="salesStatusTagType(order?.status)">{{ statusLabel }}</el-tag>
        </el-descriptions-item>
        <el-descriptions-item :label="t('sales.orderDetail.orderDate')">
          {{ order?.order_date?.slice(0, 10) }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('sales.orderDetail.requiredDate')">
          {{ order?.required_date?.slice(0, 10) }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('sales.orderDetail.totalAmount')">
          {{ Number(order?.total_amount ?? 0).toFixed(2) }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('sales.orderDetail.contactPerson')">
          {{ order?.contact_person || '-' }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('sales.orderDetail.contactPhone')">
          {{ order?.contact_phone || '-' }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('sales.orderDetail.shippingAddress')">
          {{ order?.shipping_address || '-' }}
        </el-descriptions-item>
      </el-descriptions>

      <el-table :data="order?.items || []" border class="items-table">
        <el-table-column
          prop="product_code"
          :label="t('sales.orderDetail.productCode')"
          width="140"
        />
        <el-table-column
          prop="product_name"
          :label="t('sales.orderDetail.product')"
          min-width="160"
        />
        <el-table-column :label="t('sales.orderDetail.colorNo')" width="110">
          <template #default="{ row }">{{
            row.color_no || t('sales.orderDetail.greige')
          }}</template>
        </el-table-column>
        <el-table-column prop="dye_lot_no" :label="t('sales.orderDetail.dyeLotNo')" width="110" />
        <el-table-column
          prop="quantity"
          :label="t('sales.orderDetail.quantity')"
          width="100"
          align="right"
        />
        <el-table-column
          prop="unit_price"
          :label="t('sales.orderDetail.unitPrice')"
          width="100"
          align="right"
        />
        <el-table-column
          prop="subtotal"
          :label="t('sales.orderDetail.subtotal')"
          width="120"
          align="right"
        />
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, onMounted } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { getSalesOrderById, type SalesOrder } from '@/api/sales';
import { salesStatusLabelKey, salesStatusTagType } from '@/utils/sales-status';

const route = useRoute();
const router = useRouter();
const { t } = useI18n({ useScope: 'global' });

const order = ref<SalesOrder | null>(null);
const loading = ref(false);

/** 状态文案：词表外的值由 utils/sales-status 抛错暴露，不再回显裸枚举；详情未加载时无状态可显示 */
const statusLabel = computed(() => {
  const key = salesStatusLabelKey(order.value?.status);
  return key ? t(key) : '';
});

onMounted(async () => {
  loading.value = true;
  try {
    const res = await getSalesOrderById(Number(route.params.id));
    order.value = res.data ?? null;
  } finally {
    loading.value = false;
  }
});
</script>

<style scoped>
.order-detail-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}
.page-title {
  font-size: 20px;
  color: #303133;
  margin: 0;
}
.items-table {
  margin-top: 16px;
  width: 100%;
}
</style>
