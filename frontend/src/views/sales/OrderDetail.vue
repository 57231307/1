<!--
  sales/OrderDetail.vue - 销售订单详情页
  供报价单转订单后的跳转查看（/sales/orders/:id），
  展示订单主信息 + 明细行，返回按钮回销售列表
-->
<template>
  <div class="order-detail-page">
    <div class="page-header">
      <h2 class="page-title">{{ t('sales.orderDetail.title') }}</h2>
      <div class="header-actions">
        <!-- 转合同入口：与列表行操作同源门控（已审核起活跃态 + 持 sales-contracts:create）。
             重复转换在对话框内按 preview.existing_contract 提示，这里不预判。 -->
        <el-button
          v-if="canConvertContract"
          type="primary"
          @click="convertDialogVisible = true"
        >
          {{ t('sales.table.convertContract') }}
        </el-button>
        <el-button @click="router.push('/sales')">{{ t('sales.orderDetail.back') }}</el-button>
      </div>
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
        <!-- 审批结论回显：与报价单域同范式（quotations/approval.vue:56-69）——后端出参
             （SalesOrderDetail 的 approval_reason/rejected_reason，Option<String> 键恒存在）
             有值才出行；无值（null）整行不出、不补占位文案、不做 ?? '' 兜底；标签复用既有双语键 -->
        <el-descriptions-item
          v-if="order?.approval_reason"
          :label="t('actionForm.approvalReasonTitle')"
          :span="2"
        >
          {{ order?.approval_reason }}
        </el-descriptions-item>
        <el-descriptions-item
          v-if="order?.rejected_reason"
          :label="t('actionForm.rejectReasonTitle')"
          :span="2"
        >
          {{ order?.rejected_reason }}
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
        <el-table-column
          prop="dye_lot_requirement"
          :label="t('sales.orderDetail.dyeLotNo')"
          width="110"
        />
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
        <el-table-column
          prop="quantity_tolerance_pct"
          :label="t('sales.orderDetail.tolerance')"
          width="100"
          align="right"
        >
          <template #default="{ row }">
            {{
              row.quantity_tolerance_pct != null ? Number(row.quantity_tolerance_pct) + '%' : '-'
            }}
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <ConvertToContractDialog
      v-model:visible="convertDialogVisible"
      :order-id="order?.id ?? null"
      @success="onConvertSuccess"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, ref, onMounted } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { getSalesOrderById, type SalesOrder } from '@/api/sales';
import {
  SALES_ORDER_CONVERTIBLE_TO_CONTRACT_STATUSES,
  salesStatusLabelKey,
  salesStatusTagType,
} from '@/utils/sales-status';
import { logAuxLoadFailure } from '@/utils/logger';
import { useUserStore } from '@/store/user';
import { hasRoutePermission } from '@/router';
import { PERMISSIONS } from '@/constants/permissions';
import ConvertToContractDialog from './ConvertToContractDialog.vue';

const route = useRoute();
const router = useRouter();
const { t } = useI18n({ useScope: 'global' });
const userStore = useUserStore();

const order = ref<SalesOrder | null>(null);
const loading = ref(false);
const convertDialogVisible = ref(false);

/** 状态文案：词表外的值由 utils/sales-status 抛错暴露，不再回显裸枚举；详情未加载时无状态可显示 */
const statusLabel = computed(() => {
  const key = salesStatusLabelKey(order.value?.status);
  return key ? t(key) : '';
});

// 转合同入口可见性与后端转换门同源：仅「已审核起活跃态」且持 sales-contracts:create 才给入口，
// 与列表行操作共用 utils/sales-status 的同一集合常量。重复转换在对话框内按 preview 提示。
const canConvertContract = computed(() => {
  const status = order.value?.status;
  if (!status) return false;
  if (!(SALES_ORDER_CONVERTIBLE_TO_CONTRACT_STATUSES as readonly string[]).includes(status)) {
    return false;
  }
  return hasRoutePermission(
    PERMISSIONS.SALES_CONTRACT_CREATE,
    userStore.userInfo?.permissions || []
  );
});

/** 转换成功（草稿合同已建）：跳到销售合同页继续审阅/提交审批 */
function onConvertSuccess(contractId: number) {
  void contractId;
  router.push('/sales-contract');
}

onMounted(async () => {
  loading.value = true;
  try {
    const res = await getSalesOrderById(Number(route.params.id));
    order.value = res.data ?? null;
  } catch (e: unknown) {
    logAuxLoadFailure('销售订单详情加载失败', e);
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
.header-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}
.items-table {
  margin-top: 16px;
  width: 100%;
}
</style>
