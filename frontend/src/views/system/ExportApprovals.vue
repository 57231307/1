<template>
  <div class="app-container">
    <ElCard>
      <template #header>
        <div class="card-header">
          <span>{{ $t('exportApprovals.title') }}</span>
          <ElButton text @click="loadList">{{ $t('common.refresh') }}</ElButton>
        </div>
      </template>

      <div class="filter-bar">
        <ElSelect
          v-model="searchForm.status"
          :placeholder="$t('exportApprovals.placeholderStatus')"
          clearable
          style="width: 160px"
          @change="loadList"
        >
          <ElOption label="Pending" value="pending" />
          <ElOption label="Approved" value="approved" />
          <ElOption label="Rejected" value="rejected" />
          <ElOption label="Expired" value="expired" />
          <ElOption label="Cancelled" value="cancelled" />
        </ElSelect>
        <ElSelect
          v-model="searchForm.resource_type"
          :placeholder="$t('exportApprovals.placeholderResource')"
          clearable
          style="width: 200px"
          @change="loadList"
        >
          <ElOption label="Customer" value="customer" />
          <ElOption label="Product" value="product" />
          <ElOption label="Supplier" value="supplier" />
          <ElOption label="Dye Recipe" value="dye_recipe" />
          <ElOption label="Price List" value="price_list" />
          <ElOption label="Finance Report" value="finance_report" />
          <ElOption label="Audit Log" value="audit_log" />
        </ElSelect>
      </div>

      <ElTable v-loading="loading" :data="list" border style="width: 100%">
        <ElTableColumn prop="id" label="ID" width="60" />
        <ElTableColumn
          prop="resource_type"
          :label="$t('exportApprovals.colResource')"
          width="120"
        />
        <ElTableColumn
          prop="applicant_username"
          :label="$t('exportApprovals.colApplicant')"
          width="120"
        />
        <ElTableColumn prop="status" :label="$t('exportApprovals.colStatus')" width="100">
          <template #default="{ row }">
            <ElTag :type="statusTagType(row.status)">{{ row.status }}</ElTag>
          </template>
        </ElTableColumn>
        <ElTableColumn prop="estimated_rows" :label="$t('exportApprovals.colRows')" width="100" />
        <ElTableColumn prop="created_at" :label="$t('exportApprovals.colCreatedAt')" width="180" />
        <ElTableColumn :label="$t('exportApprovals.colToken')" min-width="200">
          <template #default="{ row }">
            <template v-if="row.download_token && row.status === 'approved'">
              <span class="token-text">{{ row.download_token.substring(0, 8) }}...</span>
              <ElButton text size="small" @click="copyToken(row.download_token)">{{
                $t('common.copy')
              }}</ElButton>
            </template>
            <span v-else>-</span>
          </template>
        </ElTableColumn>
        <ElTableColumn :label="$t('common.action')" width="200" fixed="right">
          <template #default="{ row }">
            <ElButton
              v-if="row.status === 'pending' || row.status === 'pending_l2'"
              type="primary"
              size="small"
              @click="handleApprove(row)"
              >{{ $t('exportApprovals.approve') }}</ElButton
            >
            <ElButton
              v-if="row.status === 'pending' || row.status === 'pending_l2'"
              type="danger"
              size="small"
              @click="handleReject(row)"
              >{{ $t('exportApprovals.reject') }}</ElButton
            >
            <ElButton
              v-if="row.status === 'pending'"
              text
              size="small"
              @click="handleCancel(row)"
              >{{ $t('exportApprovals.cancel') }}</ElButton
            >
          </template>
        </ElTableColumn>
      </ElTable>

      <div class="pagination-bar">
        <ElPagination
          v-model:current-page="searchForm.page"
          v-model:page-size="searchForm.page_size"
          :total="total"
          layout="total, prev, pager, next"
          @current-change="loadList"
        />
      </div>
    </ElCard>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  listApprovals,
  approveRequest,
  rejectRequest,
  cancelRequest,
} from '@/api/export-approvals';
import type { ExportApprovalRequest, ListApprovalQuery } from '@/api/export-approvals';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const list = ref<ExportApprovalRequest[]>([]);
const loading = ref(false);
const total = ref(0);

const searchForm = reactive<ListApprovalQuery>({
  page: 1,
  page_size: 20,
  status: '',
  resource_type: '',
});

const statusTagType = (status: string) => {
  const map: Record<string, string> = {
    pending: 'warning',
    pending_l2: 'warning',
    approved: 'success',
    rejected: 'danger',
    expired: 'info',
    cancelled: 'info',
  };
  return map[status] || 'info';
};

const loadList = async () => {
  loading.value = true;
  try {
    const res = await listApprovals(searchForm);
    list.value = res.data?.items || [];
    total.value = res.data?.total || 0;
  } catch {
    ElMessage.error(t('exportApprovals.loadFailed'));
  } finally {
    loading.value = false;
  }
};

const handleApprove = async (row: ExportApprovalRequest) => {
  try {
    await ElMessageBox.confirm(t('exportApprovals.confirmApprove'), t('common.confirm'), {
      type: 'warning',
    });
    await approveRequest(row.id);
    ElMessage.success(t('exportApprovals.approved'));
    loadList();
  } catch {
    // user cancelled or error
  }
};

const handleReject = async (row: ExportApprovalRequest) => {
  try {
    const { value } = await ElMessageBox.prompt(
      t('exportApprovals.inputRejectReason'),
      t('exportApprovals.reject'),
      { type: 'warning' }
    );
    await rejectRequest(row.id, value);
    ElMessage.success(t('exportApprovals.rejected'));
    loadList();
  } catch {
    // user cancelled or error
  }
};

const handleCancel = async (row: ExportApprovalRequest) => {
  try {
    await ElMessageBox.confirm(t('exportApprovals.confirmCancel'), t('common.confirm'), {
      type: 'warning',
    });
    await cancelRequest(row.id);
    ElMessage.success(t('exportApprovals.cancelled'));
    loadList();
  } catch {
    // user cancelled or error
  }
};

const copyToken = async (token: string) => {
  try {
    await navigator.clipboard.writeText(token);
    ElMessage.success(t('common.copied'));
  } catch {
    ElMessage.warning(t('exportApprovals.copyFailed'));
  }
};

onMounted(loadList);
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.filter-bar {
  margin-bottom: 16px;
  display: flex;
  gap: 12px;
}
.token-text {
  font-family: monospace;
  font-size: 12px;
}
.pagination-bar {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
</style>
