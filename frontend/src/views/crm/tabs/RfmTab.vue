<!--
  RfmTab.vue - CRM 客户分级 (RFM) Tab
  来源：原 crm/index.vue 中 客户分级 (RFM) tab 内容
-->
<template>
  <div class="rfm-tab">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">{{ t('crmRfm.title') }}</h1>
      </div>
    </div>

    <div class="rfm-section">
      <el-row :gutter="20" class="mb-20">
        <el-col v-for="(count, level) in rfmDistribution" :key="level" :span="4">
          <el-card shadow="hover" class="rfm-card">
            <div class="rfm-card-content">
              <span class="rfm-card-level">{{ level }}</span>
              <span class="rfm-card-count">{{ count }} {{ t('crmRfm.countUnit') }}</span>
            </div>
          </el-card>
        </el-col>
      </el-row>

      <el-table
        v-loading="rfmLoading"
        :data="rfmCustomers"
        stripe
        :aria-label="t('crmRfm.table.ariaLabel')"
      >
        <el-table-column prop="customer_code" :label="t('crmRfm.table.customerCode')" width="120" />
        <el-table-column
          prop="customer_name"
          :label="t('crmRfm.table.customerName')"
          min-width="180"
        >
          <template #default="{ row }">
            <el-button type="primary" link @click="viewDetail(row.id)">{{
              row.customer_name
            }}</el-button>
          </template>
        </el-table-column>
        <el-table-column prop="owner_name" :label="t('crmRfm.table.owner')" width="100" />
        <el-table-column
          prop="total_amount"
          :label="t('crmRfm.table.totalAmount')"
          width="120"
          align="right"
        >
          <template #default="{ row }">
            {{ row.total_amount ? formatCurrency(row.total_amount) : '-' }}
          </template>
        </el-table-column>
        <el-table-column
          prop="total_orders"
          :label="t('crmRfm.table.totalOrders')"
          width="80"
          align="center"
        />
        <el-table-column :label="t('crmRfm.table.operation')" width="100" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="viewDetail(row.id)">{{
              t('crmRfm.table.detail')
            }}</el-button>
          </template>
        </el-table-column>
      </el-table>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { logger } from '@/utils/logger';
import { useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { formatCurrency } from '@/utils';
// D14 Batch 5b：原 crmEnhancedApi 对象已转风格 B 函数
import {
  getCustomerList,
  getCustomerRfmDistribution,
  type CustomerWithTags,
} from '@/api/crm-enhanced';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';

const { t } = useI18n({ useScope: 'global' });

const hasLoaded = createLazyLoader();

const router = useRouter();
const rfmLoading = ref(false);
const rfmCustomers = ref<CustomerWithTags[]>([]);
// 档位词表唯一写入方 = backend services/crm/cust.rs::get_rfm_distribution（VIP/重要/一般/低价值四桶），
// 分布卡片直渲其出参键为单源；前端不得另建第二套档位词表（列表端点 items = crm_lead::Model，不含逐客户档位键）。
const rfmDistribution = ref<Record<string, number>>({});

const fetchRfmCustomers = async () => {
  rfmLoading.value = true;
  try {
    const res = await getCustomerList({ page: 1, page_size: 100 });
    rfmCustomers.value = res.data.data;
    fetchRfmDistribution();
  } catch (error) {
    logger.error(t('crmRfm.loadFailed'), error);
    rfmCustomers.value = [];
  } finally {
    rfmLoading.value = false;
  }
};

const fetchRfmDistribution = async () => {
  try {
    const res = await getCustomerRfmDistribution();
    rfmDistribution.value = res.data || {};
  } catch (error) {
    logger.error(t('crmRfm.distributionLoadFailed'), error);
    rfmDistribution.value = {};
  }
};

const viewDetail = (id: number) => {
  router.push(`/crm/detail/${id}`);
};

onMounted(() => {
  loadIfNot('fetchRfmCustomers', fetchRfmCustomers, hasLoaded);
});
</script>
