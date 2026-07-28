<!--
  定制订单详情页
  - Tab 切换：基本信息 / 工艺节点 / 质量异常 / 售后
  - 操作：编辑（草稿）/ 取消（草稿）/ 推进状态
  D05 Batch 8 Group B：接入 useI18n
-->
<template>
  <div v-loading="loading" class="custom-order-detail">
    <el-card v-if="order">
      <template #header>
        <div class="card-header">
          <div>
            <span class="title">{{
              t('customOrders.detail.title', { orderNo: order.order_no })
            }}</span>
            <el-tag :type="STATUS_COLORS[order.status] || 'info'" style="margin-left: 12px">
              {{ getStatusLabel(order.status) }}
            </el-tag>
          </div>
          <div>
            <el-button @click="$router.push('/custom-orders')">{{
              t('customOrders.detail.buttonBack')
            }}</el-button>
            <el-button
              v-if="order.status === 'draft'"
              type="primary"
              @click="$router.push(`/custom-orders/${order.id}/edit`)"
            >
              {{ t('customOrders.detail.buttonEdit') }}
            </el-button>
            <el-button
              v-if="order.status !== 'completed' && order.status !== 'cancelled'"
              type="success"
              @click="handleAdvance"
            >
              {{ t('customOrders.detail.buttonAdvance') }}
            </el-button>
            <el-button v-if="order.status === 'draft'" type="danger" @click="handleCancel">
              {{ t('customOrders.detail.buttonCancel') }}
            </el-button>
          </div>
        </div>
      </template>

      <el-tabs v-model="activeTab">
        <!-- 基本信息 -->
        <el-tab-pane :label="t('customOrders.detail.tabInfo')" name="info">
          <el-descriptions :column="2" border>
            <el-descriptions-item :label="t('customOrders.detail.labelOrderNo')">{{
              order.order_no
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('customOrders.detail.labelCustomerId')">{{
              order.customer_id
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('customOrders.detail.labelProductId')">{{
              order.product_id
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('customOrders.detail.labelColorId')">{{
              order.color_id || '-'
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('customOrders.detail.labelSpec')" :span="2">{{
              order.spec
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('customOrders.detail.labelQuantity')"
              >{{ order.quantity }} {{ order.unit }}</el-descriptions-item
            >
            <el-descriptions-item :label="t('customOrders.detail.labelAmount')">
              {{ order.currency }} {{ order.total_amount || '-' }}
            </el-descriptions-item>
            <el-descriptions-item :label="t('customOrders.detail.labelYarnSpec')">{{
              order.yarn_spec || '-'
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('customOrders.detail.labelDyeMethod')">{{
              order.dye_method || '-'
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('customOrders.detail.labelFinishingMethod')">{{
              order.finishing_method || '-'
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('customOrders.detail.labelExpectedDelivery')">{{
              order.expected_delivery_date || '-'
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('customOrders.detail.labelActualDelivery')">{{
              order.actual_delivery_date || '-'
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('customOrders.detail.labelSalesOrder')">{{
              order.sales_order_id || '-'
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('customOrders.detail.labelCreatedAt')">{{
              order.created_at
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('customOrders.detail.labelUpdatedAt')" :span="2">{{
              order.updated_at
            }}</el-descriptions-item>
            <!-- v3 复审 P1-5：展示订单备注 -->
            <el-descriptions-item :label="t('customOrders.detail.labelNotes')" :span="2">{{
              order.notes || '-'
            }}</el-descriptions-item>
          </el-descriptions>
        </el-tab-pane>

        <!-- 工艺节点 -->
        <el-tab-pane :label="tabProcessNodesLabel" name="nodes">
          <ProcessFlow :nodes="order.process_nodes || []" />
        </el-tab-pane>

        <!-- 质量异常 -->
        <el-tab-pane :label="tabQualityIssuesLabel" name="issues">
          <QualityCheck
            :order-id="order.id"
            :issues="order.quality_issues || []"
            @refresh="loadData"
          />
        </el-tab-pane>

        <!-- 售后 -->
        <el-tab-pane :label="tabAfterSalesLabel" name="aftersales">
          <AfterSalesPanel
            :order-id="order.id"
            :after-sales="order.after_sales || []"
            @refresh="loadData"
          />
        </el-tab-pane>
      </el-tabs>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import { useRoute } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  getCustomOrder,
  advanceCustomOrder,
  cancelCustomOrder,
  CUSTOM_ORDER_STATUS_COLORS as STATUS_COLORS,
} from '@/api/custom-order';
import type { CustomOrderDetail } from '@/api/custom-order';
import ProcessFlow from '@/components/ProcessFlow.vue';
import QualityCheck from '@/components/QualityCheck.vue';
import logger from '@/utils/logger';
import AfterSalesPanel from '@/components/AfterSalesPanel.vue';

// v11 批次 181 P2-1 修复：CustomOrderDetail 已声明 quality_issues 和 after_sales 字段
// 不再需要本地扩展类型，直接使用 CustomOrderDetail

const route = useRoute();
const { t } = useI18n({ useScope: 'global' });
const loading = ref(false);
const order = ref<CustomOrderDetail | null>(null);
const activeTab = ref('info');

// Tab label 计算属性（避免模板中跨行 :label 导致 ESLint 解析错误）
const tabProcessNodesLabel = computed(() =>
  t('customOrders.detail.tabProcessNodes', {
    count: (order.value?.process_nodes || []).length,
  })
);
const tabQualityIssuesLabel = computed(() =>
  t('customOrders.detail.tabQualityIssues', {
    count: (order.value?.quality_issues || []).length,
  })
);
const tabAfterSalesLabel = computed(() =>
  t('customOrders.detail.tabAfterSales', {
    count: (order.value?.after_sales || []).length,
  })
);

// 状态标签映射函数（i18n）
const getStatusLabel = (status: string): string => {
  const map: Record<string, string> = {
    draft: t('customOrders.status.draft'),
    yarn_purchasing: t('customOrders.status.yarnPurchasing'),
    dyeing: t('customOrders.status.dyeing'),
    finishing: t('customOrders.status.finishing'),
    delivery: t('customOrders.status.delivery'),
    after_sales: t('customOrders.status.afterSales'),
    completed: t('customOrders.status.completed'),
    cancelled: t('customOrders.status.cancelled'),
  };
  return map[status] || status;
};

async function loadData() {
  const id = Number(route.params.id);
  if (!id) return;
  loading.value = true;
  try {
    const res = await getCustomOrder(id);
    order.value = (res.data || res) as unknown as CustomOrderDetail;
  } catch (e) {
    logger.error(t('customOrders.detail.messageLoadFailed'), e);
    ElMessage.error(t('customOrders.detail.messageLoadFailed'));
  } finally {
    loading.value = false;
  }
}

async function handleAdvance() {
  if (!order.value) return;
  try {
    await ElMessageBox.confirm(
      t('customOrders.detail.messageAdvanceConfirm'),
      t('customOrders.detail.messageAdvanceTitle'),
      { type: 'warning' }
    );
    await advanceCustomOrder(order.value.id, {
      operator_id: 1,
      notes: t('customOrders.detail.messageAdvanceNotes'),
    });
    ElMessage.success(t('customOrders.detail.messageAdvanceSuccess'));
    loadData();
  } catch (e: unknown) {
    if (e !== 'cancel') {
      const msg = e instanceof Error ? e.message : String(e);
      ElMessage.error(msg || t('customOrders.detail.messageAdvanceFailed'));
    }
  }
}

async function handleCancel() {
  if (!order.value) return;
  try {
    const { value: reason } = await ElMessageBox.prompt(
      t('customOrders.detail.messageCancelPrompt'),
      t('customOrders.detail.messageCancelTitle'),
      {
        inputPattern: /\S+/,
        inputErrorMessage: t('customOrders.detail.messageReasonRequired'),
      }
    );
    await cancelCustomOrder(order.value.id, reason);
    ElMessage.success(t('customOrders.detail.messageCancelSuccess'));
    loadData();
  } catch (e: unknown) {
    if (e !== 'cancel') {
      const msg = e instanceof Error ? e.message : String(e);
      ElMessage.error(msg || t('customOrders.detail.messageCancelFailed'));
    }
  }
}

watch(() => route.params.id, loadData);
onMounted(loadData);
</script>

<style scoped>
.custom-order-detail {
  padding: 16px;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.title {
  font-size: 18px;
  font-weight: 600;
}
</style>
