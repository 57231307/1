<!--
  api-gateway/index.vue - API 网关页面入口
  功能：承载「端点启停管理」列表与「网关统计」两块业务，作为 /api-gateway 路由的页面容器。
  调用方：路由懒加载本视图；子组件 ApiEndpointTab/ApiEndpointForm 通过 props/emit 与本入口交互。
  数据源：useApiEp 走 /api-gateway/endpoints，loadStats 走 getApiStats(/api-gateway/stats)。
-->
<template>
  <div class="api-gateway-page">
    <div class="page-header">
      <h2 class="page-title">{{ t('apiGateway.index.title') }}</h2>
    </div>

    <el-tabs v-model="activeTab" @tab-change="onTabChange">
      <el-tab-pane :label="t('apiGateway.index.tabEndpoints')" name="endpoints">
        <ApiEndpointTab
          v-model:page="ep.page"
          v-model:page-size="ep.pageSize"
          :endpoints="ep.endpoints"
          :loading="ep.endpointLoading"
          :total="ep.endpointTotal"
          :query-params="ep.endpointQuery"
          :method-type-map="ep.methodTypeMap"
          :status-type-map="ep.endpointStatusTypeMap"
          :status-map="ep.endpointStatusMap"
          @fetch="ep.fetchEndpoints"
          @new-endpoint="ep.openEndpointDialog()"
          @edit-endpoint="ep.openEndpointDialog"
          @delete-endpoint="ep.handleDeleteEndpoint"
          @update:query-params="(v: EndpointQuery) => Object.assign(ep.endpointQuery, v)"
        />
      </el-tab-pane>

      <el-tab-pane :label="t('apiGateway.index.tabStats')" name="stats" lazy>
        <div v-loading="statsLoading" class="stats-grid">
          <el-card v-for="item in statCards" :key="item.label" class="stat-card">
            <div class="stat-label">{{ item.label }}</div>
            <div class="stat-value">{{ item.value }}</div>
          </el-card>
        </div>
        <el-button v-if="stats" plain @click="loadStats">
          {{ t('apiGateway.index.buttonRefreshStats') }}
        </el-button>
      </el-tab-pane>
    </el-tabs>

    <ApiEndpointForm
      v-model:visible="ep.endpointDialogVisible"
      v-model:form-ref="ep.endpointFormRef"
      v-model:authorization-text="ep.authorizationText"
      v-model:request-schema-text="ep.requestSchemaText"
      v-model:response-schema-text="ep.responseSchemaText"
      :form="ep.endpointForm"
      :submit-loading="ep.endpointSubmitLoading"
      :rules="ep.endpointRules"
      @submit="ep.handleEndpointSubmit"
      @update:form="v => Object.assign(ep.endpointForm, v)"
    />
  </div>
</template>

<script setup lang="ts">
// API 网关页面入口，组合 useApiEp（端点管理）与网关统计（getApiStats）。
// 端点列表/分页由 useApiEp 内部的 useTableApi 承载，进入页面即自动加载，无需 onMounted。

import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useApiEp } from './composables/useApiEp';
import { getApiStats, type ApiStats } from '@/api/api-gateway';
import ApiEndpointForm from './components/ApiEndpointForm.vue';
import ApiEndpointTab, { type EndpointQuery } from './tabs/ApiEndpointTab.vue';

const { t } = useI18n({ useScope: 'global' });

const activeTab = ref('endpoints');

const ep = useApiEp();

// ===== 网关统计（getApiStats） =====
const stats = ref<ApiStats | null>(null);
const statsLoading = ref(false);

const statCards = computed(() => {
  const s = stats.value;
  if (!s) return [];
  return [
    { label: t('apiGateway.stats.totalEndpoints'), value: s.total_endpoints },
    { label: t('apiGateway.stats.activeEndpoints'), value: s.active_endpoints },
    { label: t('apiGateway.stats.inactiveEndpoints'), value: s.inactive_endpoints },
  ];
});

async function loadStats() {
  statsLoading.value = true;
  try {
    const res = await getApiStats();
    stats.value = res.data ?? null;
  } finally {
    statsLoading.value = false;
  }
}

// 统计 tab 首次展开时加载
const onTabChange = async (name: string | number) => {
  if (String(name) === 'stats' && !stats.value) await loadStats();
};
</script>

<style scoped>
.api-gateway-page {
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
  font-weight: 600;
  margin: 0;
}
.filter-container {
  display: flex;
  gap: 12px;
  margin-bottom: 16px;
}
.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 12px;
  margin-bottom: 12px;
}
.stat-card {
  text-align: center;
}
.stat-label {
  color: #909399;
  font-size: 13px;
}
.stat-value {
  font-size: 22px;
  font-weight: 600;
  margin-top: 6px;
}
</style>
