<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  ElTable,
  ElTableColumn,
  ElButton,
  ElDialog,
  ElInput,
  ElDatePicker,
  ElMessage,
  ElRow,
  ElCol,
  ElCard,
  ElTabs,
  ElTabPane,
  ElStatistic,
  ElDescriptions,
} from 'element-plus';
import { PieChart, Clock, AlarmClock } from '@element-plus/icons-vue';
import { getDashboardStats, type AuditStats, type AuditLog } from '@/api/omni-audit';
import type { ApiResponse } from '@/types/api';
import { useTableApi } from '@/composables/useTableApi';

const { t } = useI18n({ useScope: 'global' });

const activeTab = ref('dashboard');
const stats = ref<AuditStats | null>(null);
// stats 独立加载状态（与 logs 的 useTableApi.loading 分离，避免双 tab 切换时互相干扰）
const statsLoading = ref(false);

// 筛选键唯一真相：omni_audit_query_service::AuditQueryFilter{user_id,event_type,start_date,end_date,keyword,page,page_size,include_sensitive}。
// 原 resource/action/status/start_time/end_time 后端不读（被 Axum 静默丢弃=假筛选），已移除；
// start_date/end_date 为 chrono NaiveDate → 'YYYY-MM-DD'。
// （资源/动作/状态维度过滤属后端缺口，已登记串行清单）
const searchForm = ref({
  user_id: '',
  event_type: '',
  start_date: '',
  end_date: '',
});

// 批次 273：接入 useTableApi，消除手写 logs/total/loading/pagination/loadLogs 重复
// 修复 0-based 分页 bug：原 page-1 传 0 被后端 clamp(1,1000) 修正为 1，page=2 时传 1 offset=0，分页错乱
// useTableApi 使用 1-based 分页，与后端 page.unwrap_or(1).clamp(1,1000) + page.saturating_sub(1)*page_size 一致
const {
  data: logs,
  loading,
  page,
  pageSize,
  total,
  refresh: loadLogs,
  setQueryParam,
} = useTableApi<AuditLog>({
  url: '/finance/audit/search',
  listKey: 'items',
  onError: () => ElMessage.error(t('omniAudit.index.messageLoadLogsFailed')),
});

const viewDialogVisible = ref(false);
const viewData = ref<AuditLog | null>(null);

// 真实出参为 response_status（HTTP 状态码整数；UI 事件行后端缺省 0），
// 不存在字符串 status 列（SUCCESS/FAILED 为臆造词表，按后端缺口登记串行清单）
const getResponseStatusClass = (code: number) => (code >= 400 ? 'status-failed' : 'status-success');

const loadStats = async () => {
  statsLoading.value = true;
  try {
    // v11 批次 146 P1-3 修复：拦截器已返回 ApiResponse 完整对象，
    // res.data 即业务数据（AuditStats），无需 res.data!.data 双层访问
    const res = await getDashboardStats();
    stats.value = (res as ApiResponse<AuditStats> | undefined)?.data ?? null;
  } catch (error) {
    ElMessage.error(t('omniAudit.index.messageLoadStatsFailed'));
  } finally {
    statsLoading.value = false;
  }
};

// 批次 273：同步筛选条件到 useTableApi.queryParams 并刷新
// useTableApi 自动 watch page/pageSize 变化触发重载，无需手动 loadLogs
const syncQueryParams = () => {
  setQueryParam('user_id', searchForm.value.user_id ? Number(searchForm.value.user_id) : undefined);
  setQueryParam('event_type', searchForm.value.event_type || undefined);
  setQueryParam('start_date', searchForm.value.start_date || undefined);
  setQueryParam('end_date', searchForm.value.end_date || undefined);
};

const handleSearch = () => {
  syncQueryParams();
  page.value = 1;
  loadLogs();
};

const handleReset = () => {
  searchForm.value = {
    user_id: '',
    event_type: '',
    start_date: '',
    end_date: '',
  };
  syncQueryParams();
  page.value = 1;
  loadLogs();
};

// 分页（useTableApi 自动 watch page/pageSize 变化触发重载）
const handlePageChange = (p: number) => {
  page.value = p;
};

const handlePageSizeChange = (s: number) => {
  pageSize.value = s;
  page.value = 1;
};

const openViewDialog = (row: AuditLog) => {
  viewData.value = row;
  viewDialogVisible.value = true;
};

loadStats();
// 批次 273：useTableApi 构造时自动初始加载 logs，无需 setup 顶层调用 loadLogs
</script>

<template>
  <div class="app-container">
    <ElTabs v-model="activeTab" @tab-change="activeTab === 'dashboard' ? loadStats() : loadLogs()">
      <ElTabPane :label="t('omniAudit.index.tabDashboard')" name="dashboard">
        <div class="stats-grid">
          <ElCard class="stat-card">
            <div class="stat-icon total">
              <PieChart />
            </div>
            <ElStatistic
              :title="t('omniAudit.index.statTotalEvents')"
              :value="stats?.total_events || 0"
            />
          </ElCard>
          <ElCard class="stat-card">
            <div class="stat-icon today">
              <Clock />
            </div>
            <ElStatistic
              :title="t('omniAudit.index.statTodayEvents')"
              :value="stats?.today_events || 0"
            />
          </ElCard>
          <ElCard class="stat-card">
            <div class="stat-icon error">
              <AlarmClock />
            </div>
            <ElStatistic
              :title="t('omniAudit.index.statErrorCount')"
              :value="stats?.error_count || 0"
            />
          </ElCard>
          <ElCard class="stat-card">
            <div class="stat-icon avg">
              <Clock />
            </div>
            <ElStatistic
              :title="t('omniAudit.index.statAvgDuration')"
              :value="stats?.avg_duration_ms || 0"
            />
          </ElCard>
        </div>

        <ElRow :gutter="20">
          <ElCol :span="12">
            <ElCard :title="t('omniAudit.index.cardTopResources')" class="chart-card">
              <ElTable
                :data="stats?.top_resources || []"
                border
                style="width: 100%"
                :aria-label="t('omniAudit.index.ariaTopResources')"
              >
                <ElTableColumn prop="name" :label="t('omniAudit.index.colResourceName')" />
                <ElTableColumn
                  prop="count"
                  :label="t('omniAudit.index.colAccessCount')"
                  align="right"
                />
              </ElTable>
            </ElCard>
          </ElCol>
          <ElCol :span="12">
            <ElCard :title="t('omniAudit.index.cardTopUsers')" class="chart-card">
              <ElTable
                :data="stats?.top_users || []"
                border
                style="width: 100%"
                :aria-label="t('omniAudit.index.ariaTopUsers')"
              >
                <ElTableColumn prop="name" :label="t('omniAudit.index.colUserName')" />
                <ElTableColumn
                  prop="count"
                  :label="t('omniAudit.index.colOperationCount')"
                  align="right"
                />
              </ElTable>
            </ElCard>
          </ElCol>
        </ElRow>
      </ElTabPane>

      <ElTabPane :label="t('omniAudit.index.tabLogs')" name="logs">
        <div class="filter-container">
          <!-- 筛选项与后端 AuditQueryFilter 逐键对齐：user_id/event_type/start_date/end_date。
               原 resource/action/status 输入与日期时间(datetime)控件已移除——
               后端不读这些键（发送即被静默丢弃=假筛选），资源/动作/状态过滤登记串行清单 -->
          <ElRow :gutter="20">
            <ElCol :span="6">
              <ElInput
                v-model="searchForm.user_id"
                :placeholder="t('omniAudit.index.placeholderUserId')"
                class="filter-item"
                @keyup.enter="handleSearch"
              />
            </ElCol>
            <ElCol :span="6">
              <ElInput
                v-model="searchForm.event_type"
                :placeholder="t('omniAudit.index.colEventType')"
                class="filter-item"
                @keyup.enter="handleSearch"
              />
            </ElCol>
            <ElCol :span="10">
              <ElDatePicker
                v-model="searchForm.start_date"
                type="date"
                value-format="YYYY-MM-DD"
                :placeholder="t('omniAudit.index.placeholderStartTime')"
                class="filter-item"
              />
            </ElCol>
            <ElCol :span="4">
              <div class="filter-actions">
                <ElButton type="primary" @click="handleSearch">{{
                  t('omniAudit.index.buttonQuery')
                }}</ElButton>
                <ElButton @click="handleReset">{{ t('omniAudit.index.buttonReset') }}</ElButton>
              </div>
            </ElCol>
          </ElRow>
          <ElRow :gutter="20" style="margin-top: 10px">
            <ElCol :span="10">
              <ElDatePicker
                v-model="searchForm.end_date"
                type="date"
                value-format="YYYY-MM-DD"
                :placeholder="t('omniAudit.index.placeholderEndTime')"
                class="filter-item"
              />
            </ElCol>
          </ElRow>
        </div>

        <ElTable
          :data="logs"
          :loading="loading"
          border
          fit
          highlight-current-row
          style="width: 100%"
          :aria-label="t('omniAudit.index.ariaLogsTable')"
        >
          <ElTableColumn prop="id" :label="t('omniAudit.index.colId')" width="80" />
          <ElTableColumn prop="username" :label="t('omniAudit.index.colUser')" width="100" />
          <ElTableColumn prop="module" :label="t('omniAudit.index.colEventType')" width="120" />
          <ElTableColumn prop="action" :label="t('omniAudit.index.colAction')" width="100" />
          <ElTableColumn prop="resource_name" :label="t('omniAudit.index.colResource')" width="120" />
          <ElTableColumn prop="description" :label="t('omniAudit.index.colEventName')" width="150" />
          <ElTableColumn prop="response_status" :label="t('omniAudit.index.colStatus')" width="100">
            <template #default="scope">
              <span :class="['status-tag', getResponseStatusClass(scope.row.response_status)]">
                {{ scope.row.response_status }}
              </span>
            </template>
          </ElTableColumn>
          <ElTableColumn
            prop="duration_ms"
            :label="t('omniAudit.index.colDuration')"
            width="100"
            align="right"
          />
          <ElTableColumn prop="created_at" :label="t('omniAudit.index.colTime')" width="180" />
          <ElTableColumn :label="t('omniAudit.index.colOperation')" width="100" align="center">
            <template #default="scope">
              <ElButton size="small" @click="openViewDialog(scope.row as AuditLog)">{{
                t('omniAudit.index.buttonDetail')
              }}</ElButton>
            </template>
          </ElTableColumn>
        </ElTable>

        <div class="pagination-wrapper" style="margin-top: 16px; text-align: right">
          <ElPagination
            v-model:current-page="page"
            v-model:page-size="pageSize"
            :page-sizes="[10, 20, 50, 100]"
            :total="total"
            layout="total, sizes, prev, pager, next, jumper"
            :aria-label="t('omniAudit.index.ariaLogsPagination')"
            @size-change="handlePageSizeChange"
            @current-change="handlePageChange"
          />
        </div>
      </ElTabPane>
    </ElTabs>

    <ElDialog
      :title="t('omniAudit.index.titleDetail')"
      v-model="viewDialogVisible"
      width="800px"
      :aria-label="t('omniAudit.index.ariaDetailDialog')"
      @close="viewDialogVisible = false"
    >
      <div v-if="viewData">
        <!-- 字段键按后端 row_to_json 真实出参改写（username/module/resource_name/
             response_status/request_path 等）；payload/error_msg/user_name 为臆造键已移除 -->
        <ElDescriptions :column="2" border>
          <ElDescriptionsItem :label="t('omniAudit.index.colId')">{{
            viewData.id
          }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="t('omniAudit.index.labelTraceId')">{{
            viewData.trace_id
          }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="t('omniAudit.index.labelUserId')">{{
            viewData.user_id
          }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="t('omniAudit.index.colUser')">{{
            viewData.username
          }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="t('omniAudit.index.colEventType')">{{
            viewData.module
          }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="t('omniAudit.index.colResource')">{{
            viewData.resource_name || '-'
          }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="t('omniAudit.index.colAction')">{{
            viewData.action
          }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="t('omniAudit.index.colStatus')">{{
            viewData.response_status
          }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="t('omniAudit.index.colDuration')">{{
            viewData.duration_ms
          }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="t('omniAudit.index.labelCreatedAt')">{{
            viewData.created_at
          }}</ElDescriptionsItem>
          <ElDescriptionsItem :label="t('omniAudit.index.titleRequestParams')" :span="2">{{
            viewData.request_method ? viewData.request_method + ' ' + viewData.request_path : '-'
          }}</ElDescriptionsItem>
        </ElDescriptions>
      </div>
    </ElDialog>
  </div>
</template>

<style scoped>
.app-container {
  padding: 20px;
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 20px;
  margin-bottom: 20px;
}

.stat-card {
  display: flex;
  align-items: center;
  gap: 20px;
}

.stat-icon {
  width: 60px;
  height: 60px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
}

.stat-icon.total {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
}

.stat-icon.today {
  background: linear-gradient(135deg, #11998e 0%, #38ef7d 100%);
  color: white;
}

.stat-icon.error {
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
  color: white;
}

.stat-icon.avg {
  background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
  color: white;
}

.chart-card {
  margin-bottom: 20px;
}

.filter-container {
  margin-bottom: 20px;
}

.filter-item {
  width: 100%;
}

.filter-actions {
  display: flex;
  gap: 10px;
}

.status-tag {
  display: inline-block;
  padding: 4px 12px;
  border-radius: 20px;
  font-size: 12px;
}

.status-success {
  background: #f0f9eb;
  color: #67c23a;
}

.status-failed {
  background: #fef0f0;
  color: #f56c6c;
}

.payload-pre {
  background: #f5f7fa;
  padding: 15px;
  border-radius: 4px;
  font-size: 12px;
  max-height: 300px;
  overflow-y: auto;
}

.error-box {
  background: #fef0f0;
  padding: 15px;
  border-radius: 4px;
  color: #f56c6c;
  font-size: 14px;
}
</style>
