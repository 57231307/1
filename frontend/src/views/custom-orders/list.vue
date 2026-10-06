<!--
  定制订单列表页
  - 筛选（客户/状态/关键词）
  - V2Table 表格 + 分页
  - 行操作：查看 / 跟踪 / 推进 / 取消
  D05 Batch 8 Group B：接入 useI18n
-->
<template>
  <div class="custom-order-list">
    <el-card>
      <template #header>
        <div class="card-header">
          <span class="title">{{ t('customOrders.list.title') }}</span>
          <el-button type="primary" @click="$router.push('/custom-orders/new')">
            <el-icon><Plus /></el-icon>
            {{ t('customOrders.list.createButton') }}
          </el-button>
        </div>
      </template>

      <!-- 筛选区 -->
      <el-form
        :inline="true"
        :model="filters"
        class="filter-form"
        :aria-label="t('customOrders.list.filterAriaLabel')"
      >
        <el-form-item :label="t('customOrders.list.labelStatus')">
          <el-select
            v-model="filters.status"
            clearable
            :placeholder="t('customOrders.list.placeholderStatus')"
            style="width: 180px"
          >
            <el-option
              v-for="value in statusOptions"
              :key="value"
              :label="getStatusLabel(value)"
              :value="value"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('customOrders.list.labelKeyword')">
          <el-input
            v-model="filters.keyword"
            :placeholder="t('customOrders.list.placeholderKeyword')"
            clearable
            style="width: 200px"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleSearch">{{
            t('customOrders.list.buttonSearch')
          }}</el-button>
          <el-button @click="handleReset">{{ t('customOrders.list.buttonReset') }}</el-button>
        </el-form-item>
      </el-form>

      <!-- 列表 -->
      <el-table
        v-loading="loading"
        :data="orders"
        stripe
        border
        style="width: 100%"
        :empty-text="t('customOrders.list.emptyText')"
        :aria-label="t('customOrders.list.tableAriaLabel')"
      >
        <el-table-column prop="order_no" :label="t('customOrders.list.colOrderNo')" width="180" />
        <el-table-column
          prop="spec"
          :label="t('customOrders.list.colSpec')"
          min-width="150"
          show-overflow-tooltip
        />
        <el-table-column :label="t('customOrders.list.colQuantity')" width="100" align="right">
          <template #default="{ row }"> {{ row.quantity }} {{ row.unit }} </template>
        </el-table-column>
        <el-table-column :label="t('customOrders.list.colAmount')" width="140" align="right">
          <template #default="{ row }">
            <span v-if="row.total_amount"
              >{{ row.currency }} {{ formatAmount(row.total_amount) }}</span
            >
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column :label="t('customOrders.list.colStatus')" width="120" align="center">
          <template #default="{ row }">
            <el-tag :type="customOrderStatusTagType(row.status)">
              {{ getStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('customOrders.list.colExpectedDelivery')" width="120">
          <template #default="{ row }">
            {{ row.expected_delivery_date || '-' }}
          </template>
        </el-table-column>
        <!-- v3 复审 P2-4：新增备注列，使用 show-overflow-tooltip 处理长文本 -->
        <el-table-column
          prop="notes"
          :label="t('customOrders.list.colNotes')"
          min-width="160"
          show-overflow-tooltip
        >
          <template #default="{ row }">
            {{ row.notes || '-' }}
          </template>
        </el-table-column>
        <el-table-column
          prop="created_at"
          :label="t('customOrders.list.colCreatedAt')"
          width="170"
        />
        <el-table-column :label="t('customOrders.list.colActions')" width="240" fixed="right">
          <template #default="{ row }">
            <el-button size="small" link @click="goDetail(row.id)">{{
              t('customOrders.list.buttonDetail')
            }}</el-button>
            <el-button size="small" link type="primary" @click="goTracking(row.id)">{{
              t('customOrders.list.buttonTracking')
            }}</el-button>
            <el-button
              v-if="row.status !== 'completed' && row.status !== 'cancelled'"
              size="small"
              link
              type="success"
              @click="handleAdvance(row)"
            >
              {{ t('customOrders.list.buttonAdvance') }}
            </el-button>
            <el-button
              v-if="row.status === 'draft'"
              size="small"
              link
              type="danger"
              @click="handleCancel(row)"
            >
              {{ t('customOrders.list.buttonCancel') }}
            </el-button>
          </template>
        </el-table-column>
      </el-table>

      <!-- 分页 -->
      <el-pagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :total="total"
        :page-sizes="[10, 20, 50, 100]"
        layout="total, sizes, prev, pager, next, jumper"
        style="margin-top: 16px; text-align: right"
        :aria-label="t('customOrders.list.paginationAriaLabel')"
        @current-change="handleCurrentChange"
        @size-change="handleSizeChange"
      />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue';
import { useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import { advanceCustomOrder, cancelCustomOrder } from '@/api/custom-order';
import type { CustomOrderListItem } from '@/api/custom-order';
import {
  CUSTOM_ORDER_STATUSES,
  customOrderStatusLabelKey,
  customOrderStatusTagType,
} from '@/utils/custom-order-status';
import logger from '@/utils/logger';
import { useTableApi } from '@/composables/useTableApi';

const router = useRouter();
const { t } = useI18n({ useScope: 'global' });
// 批次 94 P2-12 修复：获取用户 store 以读取当前登录用户 ID
const filters = ref({ status: '', keyword: '' });

// 批次 274：接入 useTableApi，消除手写 orders/loading/pagination.total + loadData 重复
// useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
const {
  data: orders,
  loading,
  page,
  pageSize,
  total,
  refresh: loadData,
  setQueryParam,
} = useTableApi<CustomOrderListItem>({
  url: '/custom-orders',
  listKey: 'items',
  onError: () => {
    logger.error(t('customOrders.list.messageLoadFailed'));
    ElMessage.error(t('customOrders.list.messageLoadFailed'));
  },
});

// 状态筛选选项：由后端权威同源的词表派生（utils/custom-order-status，11 态），
// 不再取自残缺手写 map——旧实现缺 lab_dip/quotation/change_pending，
// 这三态的行既显示不对也筛不出来
const statusOptions = computed(() => [...CUSTOM_ORDER_STATUSES]);

// 状态标签映射函数（i18n）：labelKey 由权威词表给出；
// 词表外取值由 normalizeCustomOrderStatus 抛错显式暴露，
// 禁止旧实现 `map[status] || status` 把裸英文兜底回显给用户
const getStatusLabel = (status: string): string => {
  const key = customOrderStatusLabelKey(status);
  if (key === undefined) {
    throw new Error('定制订单状态缺失：custom_orders.status 为 NOT NULL 列，取值不得为空');
  }
  return t(key);
};

function formatAmount(val: number | string | null | undefined) {
  if (val === null || val === undefined) return '0.00';
  return Number(val).toFixed(2);
}

// 批次 274：同步筛选条件到 useTableApi.queryParams 并刷新
// useTableApi 自动 watch page/pageSize 变化触发重载，无需手动 loadData
function syncQueryParams() {
  setQueryParam('status', filters.value.status || undefined);
  setQueryParam('keyword', filters.value.keyword || undefined);
}

function handleSearch() {
  syncQueryParams();
  page.value = 1;
  loadData();
}

function handleReset() {
  filters.value = { status: '', keyword: '' };
  syncQueryParams();
  page.value = 1;
  loadData();
}

// 分页（useTableApi 自动 watch page/pageSize 变化触发重载）
function handleSizeChange(s: number) {
  pageSize.value = s;
  page.value = 1;
}

function handleCurrentChange(p: number) {
  page.value = p;
}

function goDetail(id: number) {
  router.push(`/custom-orders/${id}`);
}

function goTracking(id: number) {
  router.push(`/custom-orders/${id}/track`);
}

async function handleAdvance(row: CustomOrderListItem) {
  try {
    await ElMessageBox.confirm(
      t('customOrders.list.messageAdvanceConfirm', { orderNo: row.order_no }),
      t('customOrders.list.messageAdvanceTitle'),
      { type: 'warning' }
    );
    // 操作人身份由后端按会话（AuthContext.user_id）派生，前端不采集、不传 operator_id。
    await advanceCustomOrder(row.id, {
      notes: t('customOrders.list.messageAdvanceNotes'),
    });
    ElMessage.success(t('customOrders.list.messageAdvanceSuccess'));
    loadData();
  } catch (e: unknown) {
    if (e !== 'cancel') {
      const msg = e instanceof Error ? e.message : String(e);
      ElMessage.error(msg || t('customOrders.list.messageAdvanceFailed'));
    }
  }
}

async function handleCancel(row: CustomOrderListItem) {
  try {
    const { value: reason } = await ElMessageBox.prompt(
      t('customOrders.list.messageCancelPrompt'),
      t('customOrders.list.messageCancelTitle'),
      {
        confirmButtonText: t('customOrders.list.buttonConfirm'),
        cancelButtonText: t('customOrders.list.buttonCancel'),
        inputPattern: /\S+/,
        inputErrorMessage: t('customOrders.list.messageReasonRequired'),
      }
    );
    await cancelCustomOrder(row.id, reason);
    ElMessage.success(t('customOrders.list.messageCancelSuccess'));
    loadData();
  } catch (e: unknown) {
    if (e !== 'cancel') {
      const msg = e instanceof Error ? e.message : String(e);
      ElMessage.error(msg || t('customOrders.list.messageCancelFailed'));
    }
  }
}

// 批次 274：useTableApi 构造时自动初始加载，无需 onMounted 调用 loadData
</script>

<style scoped>
.custom-order-list {
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
.filter-form {
  margin-bottom: 16px;
}
</style>
