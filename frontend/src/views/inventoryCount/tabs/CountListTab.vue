<!--
  CountListTab.vue - 库存盘点列表 Tab
  来源：原 inventoryCount/index.vue 中 列表/统计/过滤内容
  拆分日期：2026-06-15 B3-4
-->
<template>
  <div class="count-list">
    <el-row :gutter="20" class="stats-row">
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon total-icon">
              <el-icon><Document /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">{{ t('inventoryCount.listTab.statLabelTotal') }}</div>
              <div class="stat-value">{{ stats.total }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card warning">
          <div class="stat-content">
            <div class="stat-icon pending-icon">
              <el-icon><Clock /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">{{ t('inventoryCount.listTab.statLabelInProgress') }}</div>
              <div class="stat-value">{{ stats.inProgress }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card success">
          <div class="stat-content">
            <div class="stat-icon approved-icon">
              <el-icon><CircleCheck /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">{{ t('inventoryCount.listTab.statLabelCompleted') }}</div>
              <div class="stat-value">{{ stats.completed }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card highlight">
          <div class="stat-content">
            <div class="stat-icon diff-icon">
              <el-icon><DataAnalysis /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">{{ t('inventoryCount.listTab.statLabelDifference') }}</div>
              <div class="stat-value">{{ stats.difference }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover" class="filter-card">
      <el-form
        :inline="true"
        :model="filterForm"
        class="filter-form"
        :aria-label="t('inventoryCount.listTab.ariaLabelFilter')"
      >
        <el-form-item :label="t('inventoryCount.listTab.labelCountNo')">
          <el-input
            v-model="filterForm.count_no"
            :placeholder="t('inventoryCount.listTab.placeholderCountNo')"
            clearable
          />
        </el-form-item>
        <el-form-item :label="t('inventoryCount.listTab.labelStatus')">
          <el-select
            v-model="filterForm.status"
            :placeholder="t('inventoryCount.listTab.placeholderStatus')"
            clearable
          >
            <el-option :label="t('inventoryCount.listTab.statusInProgress')" value="in_progress" />
            <el-option :label="t('inventoryCount.listTab.statusCompleted')" value="completed" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">{{
            t('inventoryCount.listTab.buttonSearch')
          }}</el-button>
          <el-button @click="handleReset">{{ t('inventoryCount.listTab.buttonReset') }}</el-button>
          <!-- P2-10 修复（批次 82 v1 复审）：补齐 v-permission 按钮权限 -->
          <el-button
            v-permission="'inventory:create'"
            type="primary"
            @click="emit('openForm', 'create', null)"
          >
            <el-icon><Plus /></el-icon>{{ t('inventoryCount.listTab.buttonCreate') }}
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table
        v-loading="loading"
        :data="counts"
        stripe
        :aria-label="t('inventoryCount.listTab.ariaLabelTable')"
      >
        <el-table-column
          prop="count_no"
          :label="t('inventoryCount.listTab.colCountNo')"
          width="160"
          fixed
        />
        <el-table-column
          prop="count_date"
          :label="t('inventoryCount.listTab.colCountDate')"
          width="120"
        />
        <el-table-column
          prop="warehouse_name"
          :label="t('inventoryCount.listTab.colWarehouse')"
          width="120"
        />
        <el-table-column
          prop="status"
          :label="t('inventoryCount.listTab.colStatus')"
          width="100"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)" size="small">
              {{ getStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="created_by_name"
          :label="t('inventoryCount.listTab.colCreatedBy')"
          width="100"
        />
        <el-table-column
          prop="created_at"
          :label="t('inventoryCount.listTab.colCreatedAt')"
          width="160"
        />
        <el-table-column
          prop="completed_at"
          :label="t('inventoryCount.listTab.colCompletedAt')"
          width="160"
        >
          <template #default="{ row }">{{ row.completed_at || '-' }}</template>
        </el-table-column>
        <el-table-column :label="t('inventoryCount.listTab.colAction')" width="200" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="emit('openDetail', row)">{{
              t('inventoryCount.listTab.buttonDetail')
            }}</el-button>
            <el-button
              v-if="row.status === 'in_progress'"
              type="primary"
              link
              size="small"
              @click="emit('openForm', 'edit', row)"
              >{{ t('inventoryCount.listTab.buttonEdit') }}</el-button
            >
            <el-button
              v-if="row.status === 'in_progress'"
              type="success"
              link
              size="small"
              @click="handleComplete(row)"
              >{{ t('inventoryCount.listTab.buttonComplete') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          :aria-label="t('inventoryCount.listTab.ariaLabelPagination')"
        />
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { reactive, watch, defineEmits } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Document, Clock, CircleCheck, DataAnalysis, Plus } from '@element-plus/icons-vue';
import { completeInventoryCount, type InventoryCountEntity } from '@/api/inventoryCount';
// 批次 280：接入 useTableApi，消除手写 counts/loading/total/fetchCounts 重复
import { useTableApi } from '@/composables/useTableApi';

const { t } = useI18n({ useScope: 'global' });

const emit = defineEmits<{
  openForm: [mode: 'create' | 'edit' | 'view', row: InventoryCountEntity | null];
  openDetail: [row: InventoryCountEntity];
}>();

// 批次 280：filterForm 仅保留筛选字段，分页字段由 useTableApi 管理
const filterForm = reactive({
  count_no: '',
  status: '',
});

const stats = reactive({
  total: 0,
  inProgress: 0,
  completed: 0,
  difference: 0,
});

// 批次 280：useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
const {
  data: counts,
  loading,
  page,
  pageSize,
  total,
  refresh: fetchCounts,
  setQueryParam,
} = useTableApi<InventoryCountEntity>({
  url: '/inventory/counts',
  onError: (err: unknown) =>
    ElMessage.error(
      (err instanceof Error ? err.message : String(err)) ||
        t('inventoryCount.listTab.messageFetchFailure')
    ),
});

// 批次 280：同步筛选条件到 useTableApi.queryParams 并刷新
const syncQueryParams = () => {
  setQueryParam('count_no', filterForm.count_no || undefined);
  setQueryParam('status', filterForm.status || undefined);
};

// 批次 280：watch counts 自动更新 stats 统计（原 fetchCounts 内的统计逻辑）
watch(counts, () => {
  stats.total = total.value;
  stats.inProgress = counts.value.filter(c => c.status === 'in_progress').length;
  stats.completed = counts.value.filter(c => c.status === 'completed').length;
  stats.difference = 0; // 实际差异数需在 details 弹窗中累加
});

/** 状态标签函数化：优先 i18n，未知状态回退到原始 status 字符串 */
const getStatusLabel = (status: string) => {
  const key = `inventoryCount.listTab.statusLabel.${status}`;
  const translated = t(key);
  return translated === key ? status : translated;
};
const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    in_progress: 'warning',
    completed: 'success',
  };
  return map[status] || 'info';
};

const handleQuery = () => {
  syncQueryParams();
  page.value = 1;
  fetchCounts();
};
const handleReset = () => {
  filterForm.count_no = '';
  filterForm.status = '';
  handleQuery();
};

const handleComplete = async (row: InventoryCountEntity) => {
  try {
    await ElMessageBox.confirm(
      t('inventoryCount.listTab.messageCompleteConfirm'),
      t('inventoryCount.listTab.titleCompleteConfirm'),
      { type: 'warning' }
    );
    await completeInventoryCount(row.id as number);
    ElMessage.success(t('inventoryCount.listTab.messageSuccess'));
    fetchCounts();
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error((error as Error).message || t('inventoryCount.listTab.messageFailure'));
    }
  }
};

defineExpose({ fetchCounts });
</script>

<style scoped>
.stats-row {
  margin-bottom: 20px;
}
.stat-card {
  border-radius: 12px;
  transition: all 0.3s;
}
.stat-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
}
.stat-card.warning {
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
}
.stat-card.warning :deep(.stat-icon) {
  background: rgba(255, 255, 255, 0.2);
}
.stat-card.success {
  background: linear-gradient(135deg, #43e97b 0%, #38f9d7 100%);
}
.stat-card.success :deep(.stat-icon) {
  background: rgba(255, 255, 255, 0.2);
}
.stat-card.highlight {
  background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
}
.stat-card.highlight :deep(.stat-icon) {
  background: rgba(255, 255, 255, 0.2);
}
.stat-card.warning :deep(.stat-label),
.stat-card.warning :deep(.stat-value),
.stat-card.success :deep(.stat-label),
.stat-card.success :deep(.stat-value),
.stat-card.highlight :deep(.stat-label),
.stat-card.highlight :deep(.stat-value) {
  color: white;
}
:deep(.stat-content) {
  display: flex;
  align-items: center;
  gap: 16px;
}
:deep(.stat-icon) {
  width: 56px;
  height: 56px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 28px;
  color: white;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}
:deep(.stat-icon.total-icon) {
  background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
}
:deep(.stat-icon.pending-icon),
:deep(.stat-icon.approved-icon),
:deep(.stat-icon.diff-icon) {
  background: rgba(255, 255, 255, 0.2);
}
:deep(.stat-info) {
  flex: 1;
}
:deep(.stat-label) {
  font-size: 14px;
  color: #909399;
  margin-bottom: 4px;
}
:deep(.stat-value) {
  font-size: 28px;
  font-weight: 700;
  color: #303133;
  line-height: 1.2;
}
.filter-card {
  margin-bottom: 20px;
}
.table-card {
  margin-bottom: 20px;
}
.pagination-wrapper {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}
</style>
