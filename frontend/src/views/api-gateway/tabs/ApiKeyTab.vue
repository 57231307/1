<!--
  ApiKeyTab.vue - API 网关密钥管理 Tab
  来源：原 api-gateway/index.vue 中 keys tab
  拆分日期：2026-06-17 P1-3-Batch-5
-->
<template>
  <el-card shadow="hover">
    <div class="filter-container">
      <el-input
        v-model="localQuery.keyword"
        :placeholder="t('apiGateway.keyTab.searchPlaceholder')"
        style="width: 200px"
        clearable
        @clear="handleSearch"
        @keyup.enter="handleSearch"
      />
      <el-select
        v-model="localQuery.status"
        :placeholder="t('apiGateway.keyTab.statusPlaceholder')"
        clearable
        style="width: 120px"
      >
        <el-option :label="t('apiGateway.keyTab.statusActive')" value="active" />
        <el-option :label="t('apiGateway.keyTab.statusInactive')" value="inactive" />
      </el-select>
      <el-button type="primary" @click="handleSearch">
        <el-icon><Search /></el-icon>
        {{ t('apiGateway.keyTab.search') }}
      </el-button>
      <el-button type="primary" @click="emit('new-key')">
        <el-icon><Plus /></el-icon>
        {{ t('apiGateway.keyTab.create') }}
      </el-button>
    </div>

    <el-table
      v-loading="loading"
      :data="apiKeys"
      stripe
      :aria-label="t('apiGateway.keyTab.tableAriaLabel')"
    >
      <el-table-column prop="key_name" :label="t('apiGateway.keyTab.columnKeyName')" width="200" />
      <el-table-column prop="app_id" :label="t('apiGateway.keyTab.columnAppId')" width="200" />
      <el-table-column :label="t('apiGateway.keyTab.columnKey')" min-width="200">
        <template #default="{ row }">
          <span class="key-text">{{ maskKey(row.api_key) }}</span>
          <el-button
            type="primary"
            link
            size="small"
            style="margin-left: 8px"
            @click="emit('view-key', row)"
            >{{ t('apiGateway.keyTab.view') }}</el-button
          >
        </template>
      </el-table-column>
      <el-table-column
        prop="expires_at"
        :label="t('apiGateway.keyTab.columnExpiresAt')"
        width="160"
      />
      <el-table-column
        prop="status"
        :label="t('apiGateway.keyTab.columnStatus')"
        width="100"
        align="center"
      >
        <template #default="{ row }">
          <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
            {{ getStatusLabel(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column
        prop="last_used_at"
        :label="t('apiGateway.keyTab.columnLastUsed')"
        width="160"
      />
      <el-table-column :label="t('apiGateway.keyTab.columnOperation')" width="200" fixed="right">
        <template #default="{ row }">
          <el-button
            v-permission="'api_key:update'"
            type="warning"
            link
            size="small"
            @click="emit('toggle-key', row)"
          >
            {{
              row.status === 'active'
                ? t('apiGateway.keyTab.disable')
                : t('apiGateway.keyTab.enable')
            }}
          </el-button>
          <el-button
            v-permission="'api_key:delete'"
            type="danger"
            link
            size="small"
            @click="emit('delete-key', row)"
            >{{ t('apiGateway.keyTab.delete') }}</el-button
          >
        </template>
      </el-table-column>
    </el-table>

    <div class="pagination-container">
      <el-pagination
        :current-page="page"
        :page-size="pageSize"
        :page-sizes="[10, 20, 50]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        :aria-label="t('apiGateway.keyTab.paginationAriaLabel')"
        @current-change="(v: number) => emit('update:page', v)"
        @size-change="(v: number) => emit('update:page-size', v)"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { reactive, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { Search, Plus } from '@element-plus/icons-vue';
import type { ApiKey } from '@/api/api-gateway';

const { t } = useI18n({ useScope: 'global' });

export interface ApiKeyQuery {
  keyword: string;
  status: string;
}

const props = defineProps<{
  apiKeys: ApiKey[];
  loading: boolean;
  total: number;
  page: number;
  pageSize: number;
  // 批次 281：queryParams 类型放宽为 Record<string, unknown>，兼容 useTableApi 的 queryParams
  queryParams: Record<string, unknown>;
}>();

const emit = defineEmits<{
  fetch: [];
  'update:page': [value: number];
  'update:page-size': [value: number];
  'new-key': [];
  'view-key': [row: ApiKey];
  'toggle-key': [row: ApiKey];
  'delete-key': [row: ApiKey];
  'update:queryParams': [value: ApiKeyQuery];
}>();

const localQuery = reactive<ApiKeyQuery>({
  keyword: '',
  status: '',
  ...(props.queryParams as Partial<ApiKeyQuery>),
});

watch(
  () => props.queryParams,
  newQuery => Object.assign(localQuery, newQuery),
  { deep: true }
);

// 批次 281：搜索时先同步筛选条件到父组件 queryParams，再触发 fetch 刷新
const handleSearch = () => {
  emit('update:queryParams', { ...localQuery });
  emit('fetch');
};

const maskKey = (key: string) => {
  if (!key) return '';
  if (key.length <= 8) return '*'.repeat(key.length);
  return key.substring(0, 4) + '*'.repeat(key.length - 8) + key.substring(key.length - 4);
};

// 状态标签映射：返回 t() 调用，确保语言切换响应
const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    active: t('apiGateway.keyTab.statusActive'),
    inactive: t('apiGateway.keyTab.statusInactive'),
  };
  return map[status] || status;
};
</script>

<style scoped>
.filter-container {
  display: flex;
  gap: 12px;
  align-items: center;
  margin-bottom: 16px;
}

.key-text {
  font-family: monospace;
  color: #909399;
}

.pagination-container {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
</style>
