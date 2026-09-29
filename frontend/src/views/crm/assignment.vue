<!--
  crm/assignment.vue - 客户分配规则主入口
  拆分：tabs/RuleDialogTab.vue / tabs/ManualAssignDialogTab.vue
  本主入口承担：列表 + 工具栏 + 公共样式。
-->
<template>
  <div class="assignment-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">{{ t('crmAssignment.title') }}</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">{{
            t('crmAssignment.breadcrumb.home')
          }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ t('crmAssignment.breadcrumb.crm') }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ t('crmAssignment.breadcrumb.assignment') }}</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="openCreateRuleDialog">
          <el-icon><Plus /></el-icon>
          {{ t('crmAssignment.createRule') }}
        </el-button>
      </div>
    </div>

    <el-tabs v-model="activeTab" class="assignment-tabs">
      <el-tab-pane :label="t('crmAssignment.tabs.rules')" name="rules">
        <el-card shadow="hover">
          <el-table
            v-loading="ruleLoading"
            :data="ruleList"
            border
            stripe
            :aria-label="t('crmAssignment.ruleTable.ariaLabel')"
          >
            <el-table-column
              type="index"
              :label="t('crmAssignment.ruleTable.index')"
              width="60"
              align="center"
            />
            <el-table-column
              prop="name"
              :label="t('crmAssignment.ruleTable.name')"
              min-width="150"
              show-overflow-tooltip
            />
            <el-table-column
              prop="days"
              :label="t('crmAssignment.ruleTable.days')"
              width="120"
              align="center"
            />
            <el-table-column
              prop="is_enabled"
              :label="t('crmAssignment.ruleTable.status')"
              width="100"
              align="center"
            >
              <template #default="{ row }">
                <el-tag v-if="row.is_enabled" type="success">{{
                  t('crmAssignment.ruleTable.enabled')
                }}</el-tag>
                <el-tag v-else type="info">{{ t('crmAssignment.ruleTable.disabled') }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column
              prop="created_at"
              :label="t('crmAssignment.ruleTable.createdAt')"
              width="170"
              align="center"
            />
            <el-table-column
              prop="updated_at"
              :label="t('crmAssignment.ruleTable.updatedAt')"
              width="170"
              align="center"
            />
            <el-table-column
              :label="t('crmAssignment.ruleTable.operation')"
              width="200"
              align="center"
              fixed="right"
            >
              <template #default="{ row }">
                <!-- P2-17 修复（批次 86 v2 复审）：编辑/删除按钮补齐 v-permission -->
                <el-button
                  v-permission="'crm_assignment:update'"
                  type="primary"
                  link
                  size="small"
                  @click="openEditRuleDialog(row)"
                  >{{ t('crmAssignment.ruleTable.edit') }}</el-button
                >
                <el-button
                  v-permission="'crm_assignment:delete'"
                  type="danger"
                  link
                  size="small"
                  @click="handleDeleteRule(row)"
                  >{{ t('crmAssignment.ruleTable.delete') }}</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane :label="t('crmAssignment.tabs.manual')" name="manual">
        <el-card shadow="hover">
          <div class="toolbar">
            <el-form
              :inline="true"
              :model="assignQuery"
              class="filter-form"
              :aria-label="t('crmAssignment.manualFilter.ariaLabel')"
            >
              <el-form-item :label="t('crmAssignment.manualFilter.keyword')">
                <el-input
                  v-model="assignQuery.keyword"
                  :placeholder="t('crmAssignment.manualFilter.keywordPlaceholder')"
                  clearable
                  @clear="fetchAssignableCustomers"
                  @keyup.enter="fetchAssignableCustomers"
                />
              </el-form-item>
              <el-form-item>
                <el-button type="primary" @click="fetchAssignableCustomers">{{
                  t('crmAssignment.manualFilter.query')
                }}</el-button>
                <el-button type="success" plain @click="batchAssignVisible = true">
                  {{ t('crmAssignment.batchAssign.title') }}
                </el-button>
              </el-form-item>
            </el-form>
          </div>

          <el-table
            v-loading="assignLoading"
            :data="assignableCustomers"
            border
            stripe
            :aria-label="t('crmAssignment.manualTable.ariaLabel')"
          >
            <el-table-column
              type="index"
              :label="t('crmAssignment.manualTable.index')"
              width="60"
              align="center"
            />
            <el-table-column
              prop="customer_name"
              :label="t('crmAssignment.manualTable.customerName')"
              min-width="150"
              show-overflow-tooltip
            />
            <el-table-column
              prop="contact_person"
              :label="t('crmAssignment.manualTable.contactPerson')"
              width="100"
              show-overflow-tooltip
            />
            <el-table-column
              prop="phone"
              :label="t('crmAssignment.manualTable.phone')"
              width="120"
              show-overflow-tooltip
            />
            <el-table-column
              prop="owner_name"
              :label="t('crmAssignment.manualTable.currentOwner')"
              width="100"
              show-overflow-tooltip
            />
            <el-table-column
              :label="t('crmAssignment.manualTable.operation')"
              width="120"
              align="center"
              fixed="right"
            >
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="openAssignDialog(row)">{{
                  t('crmAssignment.manualTable.assign')
                }}</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>
    </el-tabs>

    <RuleDialogTab
      v-model="ruleDialogVisible"
      :title="ruleDialogTitle"
      :row-data="currentRuleRow"
      :users="users"
      @submitted="fetchRules"
    />

    <ManualAssignDialogTab
      v-model="assignDialogVisible"
      :customer-name="currentCustomerName"
      :customer-id="currentCustomerId"
      :users="users"
      @submitted="fetchAssignableCustomers"
    />

    <!-- 批量分配（batchAssignCustomers：多客户一次分配给同一销售员） -->
    <el-dialog
      v-model="batchAssignVisible"
      :title="t('crmAssignment.batchAssign.title')"
      width="520"
    >
      <el-form :model="batchForm" label-width="110px">
        <el-form-item :label="t('crmAssignment.batchAssign.customerIds')" required>
          <el-input
            v-model="batchForm.customerIdsText"
            type="textarea"
            :rows="3"
            :placeholder="t('crmAssignment.batchAssign.idsPlaceholder')"
          />
        </el-form-item>
        <el-form-item :label="t('crmAssignment.batchAssign.assignTo')" required>
          <el-select v-model="batchForm.assignTo" style="width: 100%" filterable>
            <el-option
              v-for="user in users"
              :key="user.id"
              :label="user.real_name || user.username"
              :value="user.id"
            />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="batchAssignVisible = false">{{
          t('crmAssignment.batchAssign.cancel')
        }}</el-button>
        <el-button type="primary" :loading="batchSaving" @click="handleBatchAssign">
          {{ t('crmAssignment.batchAssign.confirm') }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import type { User } from '@/api/user';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import { logger, logAuxLoadFailure } from '@/utils/logger';
// D14 Batch 5b：原 crmEnhancedApi 对象已转风格 B 函数
import {
  getRecycleRuleList,
  deleteRecycleRule,
  getSalesUserList,
  batchAssignCustomers,
  getCustomerPoolList,
  type AssignableCustomer,
  type RecycleRule,
} from '@/api/crm-enhanced';
import RuleDialogTab from './tabs/RuleDialogTab.vue';
import ManualAssignDialogTab from './tabs/ManualAssignDialogTab.vue';

const { t } = useI18n({ useScope: 'global' });

const hasLoaded = createLazyLoader();

const activeTab = ref('rules');
const ruleLoading = ref(false);
const ruleList = ref<RecycleRule[]>([]);

const assignLoading = ref(false);
const assignableCustomers = ref<AssignableCustomer[]>([]);
const assignQuery = reactive({ keyword: '' });

const users = ref<User[]>([]);

// 规则行对齐后端 RecycleRule 契约（name/days/is_enabled）
const ruleDialogVisible = ref(false);
const ruleDialogTitle = ref('');
const currentRuleRow = ref<RecycleRule | null>(null);
const assignDialogVisible = ref(false);
const currentCustomerId = ref<number | null>(null);
const currentCustomerName = ref('');

const fetchRules = async () => {
  ruleLoading.value = true;
  try {
    // P1-5：调用真实 API 获取分配规则（后端使用 recycle-rules 接口承载规则）
    const res = await getRecycleRuleList();
    // crm API 不嵌套 .data（直接返回 data），保留 ?? 容错
    ruleList.value = (res.data ?? res) as unknown as RecycleRule[];
  } catch (error) {
    const err = error as Error;
    logger.warn(t('crmAssignment.message.loadRulesFailed'), err.message);
  } finally {
    ruleLoading.value = false;
  }
};

const fetchAssignableCustomers = async () => {
  assignLoading.value = true;
  try {
    // P1-5：调用真实 API 获取可分配客户（公海池）
    // 后端 crm_pool_handler.rs::PoolQueryParams 读取 keyword，此前该搜索框的值从未随请求发出
    // （只发 page/page_size），导致筛选框存在却永无效果——现补齐绑定。
    const res = await getCustomerPoolList({
      page: 1,
      page_size: 50,
      keyword: assignQuery.keyword || undefined,
    });
    assignableCustomers.value = res.data.items;
  } catch (error) {
    const err = error as Error;
    logger.warn(t('crmAssignment.message.loadAssignableFailed'), err.message);
  } finally {
    assignLoading.value = false;
  }
};

const fetchUsers = async () => {
  try {
    // 有差异修正：手动分配的分配对象应为销售员（后端 /crm/sales-users），
    // 原实现取全量系统用户（getUserList），语义不符
    const res = await getSalesUserList();
    users.value = (res.data || []) as unknown as User[];
  } catch (error) {
    logAuxLoadFailure(t('crmAssignment.message.loadSalesUsersFailed'), error);
    users.value = [];
  }
};

const openCreateRuleDialog = () => {
  currentRuleRow.value = null;
  ruleDialogTitle.value = t('crmAssignment.ruleDialogTitle.create');
  ruleDialogVisible.value = true;
};

const openEditRuleDialog = (row: RecycleRule) => {
  currentRuleRow.value = row;
  ruleDialogTitle.value = t('crmAssignment.ruleDialogTitle.edit');
  ruleDialogVisible.value = true;
};

const openAssignDialog = (row: { id: number; customer_name: string }) => {
  currentCustomerId.value = row.id;
  currentCustomerName.value = row.customer_name;
  assignDialogVisible.value = true;
};

// ===== 批量分配（batchAssignCustomers） =====
const batchAssignVisible = ref(false);
const batchSaving = ref(false);
const batchForm = reactive({
  customerIdsText: '',
  assignTo: undefined as number | undefined,
});

const handleBatchAssign = async () => {
  const ids = batchForm.customerIdsText
    .split(/[,，;；\s]+/)
    .map(s => Number(s.trim()))
    .filter(n => Number.isInteger(n) && n > 0);
  if (ids.length === 0 || !batchForm.assignTo) {
    ElMessage.warning(t('crmAssignment.batchAssign.required'));
    return;
  }
  batchSaving.value = true;
  try {
    await batchAssignCustomers({
      assignments: ids.map(customer_id => ({
        customer_id,
        assign_to: batchForm.assignTo as number,
      })),
    });
    ElMessage.success(t('crmAssignment.batchAssign.success', { count: ids.length }));
    batchAssignVisible.value = false;
    batchForm.customerIdsText = '';
    await fetchAssignableCustomers();
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('crmAssignment.batchAssign.failed'));
  } finally {
    batchSaving.value = false;
  }
};

const handleDeleteRule = async (row: { id: number; name: string }) => {
  try {
    await ElMessageBox.confirm(
      t('crmAssignment.message.deleteConfirm', { name: row.name }),
      t('crmAssignment.message.deleteTitle'),
      {
        type: 'warning',
      }
    );
    // 修复假删除：实际调用 deleteRecycleRule 移除规则
    await deleteRecycleRule(row.id);
    ElMessage.success(t('crmAssignment.message.deleteSuccess'));
    fetchRules();
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error;
      ElMessage.error(err.message || t('crmAssignment.message.deleteFailed'));
    }
  }
};

onMounted(() => {
  fetchRules();
  loadIfNot('users', fetchUsers, hasLoaded);
});
</script>

<style scoped>
.assignment-page {
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

.assignment-tabs {
  background: #fff;
  border-radius: 4px;
}

.toolbar {
  margin-bottom: 16px;
}

.filter-form {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}
</style>
