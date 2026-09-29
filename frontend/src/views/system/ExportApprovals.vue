<template>
  <div class="app-container">
    <ElCard>
      <template #header>
        <div class="card-header">
          <span>{{ $t('exportApprovals.title') }}</span>
          <div>
            <ElButton
              :type="viewMode === 'all' ? 'primary' : 'default'"
              @click="switchMode('all')"
              >{{ $t('common.all') }}</ElButton
            >
            <ElButton
              :type="viewMode === 'pending_me' ? 'primary' : 'default'"
              @click="switchMode('pending_me')"
              >待我审批</ElButton
            >
            <ElButton type="success" @click="createVisible = true">发起审批</ElButton>
            <ElButton text @click="openVerifyToken">校验令牌</ElButton>
            <ElButton text @click="loadList">{{ $t('common.refresh') }}</ElButton>
          </div>
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
        <ElTableColumn :label="$t('common.action')" width="260" fixed="right">
          <template #default="{ row }">
            <ElButton size="small" @click="openDetail(row)">详情</ElButton>
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

    <!-- 审批详情对话框（getApprovalDetail 回源） -->
    <ElDialog v-model="detailVisible" title="审批详情" width="640">
      <ElDescriptions v-if="detailRow" :column="2" border>
        <ElDescriptionsItem label="ID">{{ detailRow.id }}</ElDescriptionsItem>
        <ElDescriptionsItem label="资源类型">{{ detailRow.resource_type }}</ElDescriptionsItem>
        <ElDescriptionsItem label="申请人">{{
          detailRow.applicant_username || '-'
        }}</ElDescriptionsItem>
        <ElDescriptionsItem label="状态">
          <ElTag :type="statusTagType(String(detailRow.status))">{{ detailRow.status }}</ElTag>
        </ElDescriptionsItem>
        <ElDescriptionsItem label="预计行数">{{
          detailRow.estimated_rows ?? '-'
        }}</ElDescriptionsItem>
        <ElDescriptionsItem label="文件格式">{{ detailRow.file_format || '-' }}</ElDescriptionsItem>
        <ElDescriptionsItem label="导出参数" :span="2">
          <pre class="detail-json">{{ JSON.stringify(detailRow.export_params, null, 2) }}</pre>
        </ElDescriptionsItem>
      </ElDescriptions>
    </ElDialog>

    <!-- 发起审批（createApproval） -->
    <ElDialog v-model="createVisible" title="发起审批" width="520">
      <ElForm :model="createForm" label-width="110px">
        <ElFormItem label="资源类型" required>
          <ElSelect v-model="createForm.resource_type" style="width: 100%">
            <ElOption label="客户数据" value="customer" />
            <ElOption label="产品数据" value="product" />
            <ElOption label="供应商数据" value="supplier" />
            <ElOption label="染色配方" value="dye_recipe" />
            <ElOption label="价目表" value="price_list" />
            <ElOption label="财务报表" value="finance_report" />
            <ElOption label="审计日志" value="audit_log" />
          </ElSelect>
        </ElFormItem>
        <ElFormItem label="导出参数" required>
          <ElInput
            v-model="createForm.paramsText"
            type="textarea"
            :rows="4"
            placeholder='JSON，如 {"date_range":["2026-01-01","2026-12-31"]}'
          />
        </ElFormItem>
        <ElFormItem label="预计行数">
          <ElInputNumber v-model="createForm.estimated_rows" :min="0" />
        </ElFormItem>
        <ElFormItem label="文件格式">
          <ElSelect v-model="createForm.file_format" style="width: 100%">
            <ElOption label="Excel" value="excel" />
            <ElOption label="PDF" value="pdf" />
          </ElSelect>
        </ElFormItem>
      </ElForm>
      <template #footer>
        <ElButton @click="createVisible = false">取消</ElButton>
        <ElButton type="primary" :loading="createSaving" @click="handleCreate">提交</ElButton>
      </template>
    </ElDialog>

    <!-- 下载令牌校验（verifyToken） -->
    <ElDialog v-model="tokenVisible" title="下载令牌校验" width="440">
      <ElInput v-model="tokenText" placeholder="粘贴下载令牌" />
      <template #footer>
        <ElButton @click="tokenVisible = false">取消</ElButton>
        <ElButton type="primary" :loading="tokenChecking" @click="handleVerifyToken">校验</ElButton>
      </template>
    </ElDialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue';
import { logger } from '@/utils/logger';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  listApprovals,
  listPendingForMe,
  createApproval,
  getApprovalDetail,
  verifyToken,
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
    const query = { ...searchForm };
    if (viewMode.value === 'pending_me') delete query.status;
    const res =
      viewMode.value === 'pending_me' ? await listPendingForMe(query) : await listApprovals(query);
    list.value = res.data?.items || [];
    total.value = res.data?.total || 0;
  } catch {
    ElMessage.error(t('exportApprovals.loadFailed'));
  } finally {
    loading.value = false;
  }
};

// ===== 待我审批视图切换（listPendingForMe） =====
const viewMode = ref<'all' | 'pending_me'>('all');

const switchMode = (mode: 'all' | 'pending_me') => {
  viewMode.value = mode;
  page_reset_and_load();
};

const page_reset_and_load = () => {
  searchForm.page = 1;
  void loadList();
};

// ===== 详情回源（getApprovalDetail） =====
const detailVisible = ref(false);
const detailRow = ref<Record<string, unknown> | null>(null);

const openDetail = async (row: ExportApprovalRequest) => {
  detailRow.value = row as unknown as Record<string, unknown>;
  detailVisible.value = true;
  try {
    const res = await getApprovalDetail(row.id);
    if (res.data) {
      detailRow.value = res.data as unknown as Record<string, unknown>;
      detailVisible.value = true;
    }
  } catch (error) {
    logger.error(t('exportApprovals.detailFailed'), error);
  }
};

// ===== 发起审批（createApproval） =====
const createVisible = ref(false);
const createSaving = ref(false);
const createForm = reactive({
  resource_type: 'customer',
  paramsText: '{}',
  estimated_rows: undefined as number | undefined,
  file_format: 'excel',
});

const handleCreate = async () => {
  let exportParams: unknown;
  try {
    exportParams = JSON.parse(createForm.paramsText || '{}');
  } catch {
    ElMessage.warning('导出参数 JSON 格式有误');
    return;
  }
  if (typeof exportParams !== 'object' || exportParams === null || Array.isArray(exportParams)) {
    ElMessage.warning('导出参数需为 JSON 对象');
    return;
  }
  createSaving.value = true;
  try {
    await createApproval({
      resource_type: createForm.resource_type,
      export_params: exportParams as Record<string, unknown>,
      estimated_rows: createForm.estimated_rows ?? undefined,
      file_format: createForm.file_format || undefined,
    });
    ElMessage.success('审批申请已提交');
    createVisible.value = false;
    viewMode.value = 'all';
    page_reset_and_load();
  } catch (e) {
    ElMessage.error(e instanceof Error ? e.message : String(e));
  } finally {
    createSaving.value = false;
  }
};

// ===== 下载令牌校验（verifyToken） =====
const tokenVisible = ref(false);
const tokenChecking = ref(false);
const tokenText = ref('');

const openVerifyToken = () => {
  tokenVisible.value = true;
};

const handleVerifyToken = async () => {
  if (!tokenText.value.trim()) {
    ElMessage.warning('请粘贴令牌');
    return;
  }
  tokenChecking.value = true;
  try {
    const res = await verifyToken(tokenText.value.trim());
    ElMessageBox.alert(JSON.stringify(res.data ?? res, null, 2), '校验结果');
  } catch (e) {
    ElMessage.error(e instanceof Error ? e.message : String(e));
  } finally {
    tokenChecking.value = false;
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
.detail-json {
  background: var(--el-fill-color-light);
  border-radius: 4px;
  padding: 8px;
  font-size: 12px;
  max-height: 200px;
  overflow: auto;
  white-space: pre-wrap;
  margin: 0;
}
</style>
