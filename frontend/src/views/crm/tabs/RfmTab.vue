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
          :label="t('crmRfm.table.segment')"
          width="120"
          class-name="rfm-segment-cell"
        >
          <template #default="{ row }">{{ segmentText(row.converted_customer_id) }}</template>
        </el-table-column>
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
import { logger, logAuxLoadFailure } from '@/utils/logger';
import { useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
// D14 Batch 5b：原 crmEnhancedApi 对象已转风格 B 函数
import {
  getCustomerList,
  getCustomerRfmDistribution,
  getCustomerRfmSegments,
  type CustomerWithTags,
  type RfmSegmentItem,
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
// 按客户主键（converted_customer_id，非线索 id）索引的批量档位联显表：key=customer_id，value=后端单行结论。
// 只承载「该行是否可见档位 + 展示 token」，四桶判断词表在后端，本表不复制词表逻辑。
const rfmSegmentMap = ref<Map<number, RfmSegmentItem>>(new Map());

// 档位列展示口径：仅 access==='visible' 直显后端 segment 原文；
// 未转化线索（converted_customer_id 为 null）、后端回 not_found/no_permission、或联显请求失败/未命中
// 一律走「无档位」占位，绝不把非 visible 渲染成分级（后端契约保证非 visible 的 segment 恒 null）。
const segmentText = (convertedCustomerId: number | null): string => {
  if (convertedCustomerId == null) return t('crmRfm.table.noSegment');
  const item = rfmSegmentMap.value.get(convertedCustomerId);
  if (!item || item.access !== 'visible') return t('crmRfm.table.noSegment');
  return item.segment ?? t('crmRfm.table.noSegment');
};

// 列表就绪后按当前页收集 converted_customer_id（真实客户主键）一次性批量取档位，按 customer_id 建 Map。
// 非逐行请求（禁 N+1）；页内无已转化客户则不发起请求。失败按 logAuxLoadFailure 降级为告警，
// 并清空映射使档位列统一走占位——不静默吞成空数据造成「全部无档位」的假绿。
const fetchRfmSegments = async (customers: CustomerWithTags[]) => {
  const customerIds = Array.from(
    new Set(
      customers
        .map(c => c.converted_customer_id)
        .filter((id): id is number => typeof id === 'number')
    )
  );
  if (customerIds.length === 0) {
    rfmSegmentMap.value = new Map();
    return;
  }
  try {
    const items = await getCustomerRfmSegments(customerIds);
    rfmSegmentMap.value = new Map(items.map(item => [item.customer_id, item]));
  } catch (error) {
    logAuxLoadFailure(t('crmRfm.segmentsLoadFailed'), error);
    rfmSegmentMap.value = new Map();
  }
};

const fetchRfmCustomers = async () => {
  rfmLoading.value = true;
  try {
    const res = await getCustomerList({ page: 1, page_size: 100 });
    rfmCustomers.value = res.data.data;
    fetchRfmDistribution();
    // 列表加载成功即触发一次批量档位联显（页内 converted_customer_id 集合，独立于分布聚合面）
    fetchRfmSegments(rfmCustomers.value);
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
    logAuxLoadFailure(t('crmRfm.distributionLoadFailed'), error);
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
