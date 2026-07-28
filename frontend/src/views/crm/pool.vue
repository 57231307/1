<!--
  crm/pool.vue - 客户公海池主入口
  拆分：tabs/ClaimDialogTab.vue / tabs/TransferDialogTab.vue / tabs/ReleaseDialogTab.vue
  本主入口承担：列表 + 工具栏 + 公共样式。
-->
<template>
  <div class="pool-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">{{ t('crmPool.title') }}</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">{{
            t('crmPool.breadcrumb.home')
          }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ t('crmPool.breadcrumb.crm') }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ t('crmPool.breadcrumb.pool') }}</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleClaimSelected">
          <el-icon><Plus /></el-icon>
          {{ t('crmPool.batchClaim') }}
        </el-button>
        <el-button @click="router.push('/crm')">
          <el-icon><Back /></el-icon>
          {{ t('crmPool.back') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form
        :inline="true"
        :model="queryParams"
        class="filter-form"
        :aria-label="t('crmPool.filter.ariaLabel')"
      >
        <el-form-item :label="t('crmPool.filter.keyword')">
          <el-input
            v-model="queryParams.keyword"
            :placeholder="t('crmPool.filter.keywordPlaceholder')"
            clearable
            @clear="handleQuery"
            @keyup.enter="handleQuery"
          />
        </el-form-item>
        <el-form-item :label="t('crmPool.filter.customerType')">
          <el-select
            v-model="queryParams.customer_type"
            :placeholder="t('crmPool.filter.customerTypePlaceholder')"
            clearable
            @change="handleQuery"
          >
            <el-option :label="t('crmPool.customerType.normal')" value="normal" />
            <el-option :label="t('crmPool.customerType.vip')" value="vip" />
            <el-option :label="t('crmPool.customerType.wholesale')" value="wholesale" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('crmPool.filter.daysInPool')">
          <el-select
            v-model="queryParams.daysInPool"
            :placeholder="t('crmPool.filter.daysInPoolPlaceholder')"
            clearable
            @change="handleQuery"
          >
            <el-option :label="t('crmPool.filter.daysWithinWeek')" value="7" />
            <el-option :label="t('crmPool.filter.daysWithinMonth')" value="30" />
            <el-option :label="t('crmPool.filter.daysWithinQuarter')" value="90" />
            <el-option :label="t('crmPool.filter.daysOverQuarter')" value="91" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">
            <el-icon><Search /></el-icon>
            {{ t('crmPool.filter.query') }}
          </el-button>
          <el-button @click="handleReset">
            <el-icon><Refresh /></el-icon>
            {{ t('crmPool.filter.reset') }}
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table
        v-loading="loading"
        :data="poolList"
        border
        stripe
        :aria-label="t('crmPool.table.ariaLabel')"
        @selection-change="handleSelectionChange"
      >
        <el-table-column type="selection" width="55" align="center" />
        <el-table-column type="index" :label="t('crmPool.table.index')" width="60" align="center" />
        <el-table-column
          prop="customer_name"
          :label="t('crmPool.table.customerName')"
          min-width="150"
          show-overflow-tooltip
        />
        <el-table-column
          prop="contact_person"
          :label="t('crmPool.table.contactPerson')"
          width="100"
          show-overflow-tooltip
        />
        <el-table-column
          prop="phone"
          :label="t('crmPool.table.phone')"
          width="120"
          show-overflow-tooltip
        />
        <el-table-column
          prop="customer_type"
          :label="t('crmPool.table.type')"
          width="100"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="getCustomerTypeTag(row.customer_type)" size="small">
              {{ getCustomerTypeLabel(row.customer_type) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="released_at"
          :label="t('crmPool.table.releasedAt')"
          width="160"
          align="center"
        />
        <el-table-column
          prop="released_by_name"
          :label="t('crmPool.table.releasedBy')"
          width="100"
          show-overflow-tooltip
        />
        <el-table-column
          prop="days_in_pool"
          :label="t('crmPool.table.daysInPool')"
          width="100"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="getDaysTag(row.days_in_pool)" size="small">
              {{ row.days_in_pool }} {{ t('crmPool.table.daysUnit') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="release_reason"
          :label="t('crmPool.table.releaseReason')"
          min-width="150"
          show-overflow-tooltip
        />
        <el-table-column
          :label="t('crmPool.table.operation')"
          width="240"
          align="center"
          fixed="right"
        >
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="openClaimDialog(row)">{{
              t('crmPool.table.claim')
            }}</el-button>
            <el-button type="primary" link size="small" @click="openTransferDialog(row)">{{
              t('crmPool.table.transfer')
            }}</el-button>
            <el-button
              v-if="row.previous_owner_id"
              type="warning"
              link
              size="small"
              @click="openReleaseDialog(row)"
              >{{ t('crmPool.table.release') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-container">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          :aria-label="t('crmPool.table.paginationAriaLabel')"
          @size-change="handleSizeChange"
          @current-change="handleCurrentChange"
        />
      </div>
    </el-card>

    <ClaimDialogTab
      v-model="claimDialogVisible"
      :customer-name="currentCustomerName"
      :customer-id="currentCustomerId"
      @submitted="getList"
    />

    <TransferDialogTab
      v-model="transferDialogVisible"
      :customer-name="currentCustomerName"
      :customer-id="currentCustomerId"
      :users="users"
      @submitted="getList"
    />

    <ReleaseDialogTab
      v-model="releaseDialogVisible"
      :customer-name="currentCustomerName"
      :customer-id="currentCustomerId"
      @submitted="getList"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue';
import { useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { Plus, Back, Search, Refresh } from '@element-plus/icons-vue';
import { getUserList, type User } from '@/api/user';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import { logger } from '@/utils/logger';
import { type PoolCustomer } from '@/api/crm-enhanced';
import { useTableApi } from '@/composables/useTableApi';
import ClaimDialogTab from './tabs/ClaimDialogTab.vue';
import TransferDialogTab from './tabs/TransferDialogTab.vue';
import ReleaseDialogTab from './tabs/ReleaseDialogTab.vue';

const { t } = useI18n({ useScope: 'global' });

const hasLoaded = createLazyLoader();

const router = useRouter();
const queryParams = reactive({
  keyword: '',
  customer_type: '',
  daysInPool: '',
});

// 批次 269：接入 useTableApi，消除手写分页重复 + 修复原硬编码参数 bug
const {
  data: poolList,
  loading,
  page,
  pageSize,
  total,
  refresh: getList,
  setQueryParam,
} = useTableApi<PoolCustomer>({
  url: '/crm/pool',
  onError: (e: unknown) => logger.warn(t('crmPool.message.loadFailed'), String(e)),
});

const users = ref<User[]>([]);

const claimDialogVisible = ref(false);
const transferDialogVisible = ref(false);
const releaseDialogVisible = ref(false);
const currentCustomerId = ref<number | null>(null);
const currentCustomerName = ref('');

const fetchUsers = async () => {
  try {
    const res = await getUserList();
    users.value = res.data?.list || [];
  } catch (error) {
    users.value = [];
  }
};

const handleQuery = () => {
  setQueryParam('keyword', queryParams.keyword || undefined);
  setQueryParam('customer_type', queryParams.customer_type || undefined);
  page.value = 1;
  getList();
};

const handleReset = () => {
  queryParams.keyword = '';
  queryParams.customer_type = '';
  queryParams.daysInPool = '';
  handleQuery();
};

const openClaimDialog = (row: { id: number; customer_name: string }) => {
  currentCustomerId.value = row.id;
  currentCustomerName.value = row.customer_name;
  claimDialogVisible.value = true;
};

const openTransferDialog = (row: { id: number; customer_name: string }) => {
  currentCustomerId.value = row.id;
  currentCustomerName.value = row.customer_name;
  transferDialogVisible.value = true;
};

const openReleaseDialog = (row: { id: number; customer_name: string }) => {
  currentCustomerId.value = row.id;
  currentCustomerName.value = row.customer_name;
  releaseDialogVisible.value = true;
};

const handleClaimSelected = () => {
  ElMessage.info(t('crmPool.message.selectToClaim'));
};

const handleSelectionChange = () => {
  // 选区变化
};

const handleSizeChange = (val: number) => {
  pageSize.value = val;
  page.value = 1;
};

const handleCurrentChange = (val: number) => {
  page.value = val;
};

const getCustomerTypeLabel = (type: string) => {
  const labelMap: Record<string, string> = {
    normal: t('crmPool.customerType.normal'),
    vip: t('crmPool.customerType.vip'),
    wholesale: t('crmPool.customerType.wholesale'),
  };
  return labelMap[type] || type;
};

const getCustomerTypeTag = (type: string) => {
  const typeMap: Record<string, string> = { normal: '', vip: 'warning', wholesale: 'success' };
  return typeMap[type] || '';
};

const getDaysTag = (days: number) => {
  if (days > 90) return 'danger';
  if (days > 30) return 'warning';
  return 'success';
};

onMounted(() => {
  loadIfNot('users', fetchUsers, hasLoaded);
});
</script>

<style scoped>
.pool-page {
  padding: 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.header-left {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.page-title {
  margin: 0;
  font-size: 24px;
  font-weight: 600;
}

.header-actions {
  display: flex;
  gap: 10px;
}

.filter-card {
  margin-bottom: 20px;
}

.filter-form {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.table-card {
  margin-bottom: 20px;
}

.pagination-container {
  display: flex;
  justify-content: flex-end;
  margin-top: 20px;
}
</style>
