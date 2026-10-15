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
              >{{ $t('exportApprovals.pendingApproval') }}</ElButton
            >
            <ElButton type="success" @click="createVisible = true">{{
              $t('exportApprovals.createRequest')
            }}</ElButton>
            <ElButton text @click="openVerifyToken">{{
              $t('exportApprovals.verifyToken')
            }}</ElButton>
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
          <ElOption
            v-for="status in EXPORT_APPROVAL_STATUSES"
            :key="status"
            :label="$t(EXPORT_APPROVAL_STATUS_LABEL_KEYS[status])"
            :value="status"
          />
        </ElSelect>
        <ElSelect
          v-model="searchForm.resource_type"
          :placeholder="$t('exportApprovals.placeholderResource')"
          clearable
          style="width: 200px"
          @change="loadList"
        >
          <ElOption
            v-for="rt in EXPORT_APPROVAL_RESOURCE_TYPES"
            :key="rt"
            :label="$t(EXPORT_APPROVAL_RESOURCE_LABEL_KEYS[rt])"
            :value="rt"
          />
        </ElSelect>
      </div>

      <ElTable v-loading="loading" :data="list" border style="width: 100%">
        <ElTableColumn prop="id" label="ID" width="60" />
        <ElTableColumn :label="$t('exportApprovals.colResource')" width="120">
          <template #default="{ row }">{{
            row.resource_type ? $t(exportApprovalResourceLabelKey(row.resource_type)) : ''
          }}</template>
        </ElTableColumn>
        <ElTableColumn
          prop="applicant_username"
          :label="$t('exportApprovals.colApplicant')"
          width="120"
        />
        <ElTableColumn prop="status" :label="$t('exportApprovals.colStatus')" width="100">
          <template #default="{ row }">
            <ElTag :type="exportApprovalStatusTagType(row.status)">{{
              $t(exportApprovalStatusLabelKey(row.status))
            }}</ElTag>
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
        <ElTableColumn :label="$t('common.action')" width="320" fixed="right">
          <template #default="{ row }">
            <ElButton size="small" @click="openDetail(row)">{{ $t('common.detail') }}</ElButton>
            <!-- 集中下载：申请人本人 + 已批准 + 持有一次性令牌且未用尽 -->
            <ElButton
              v-if="canDownloadRow(row)"
              type="success"
              size="small"
              :loading="downloadingId === row.id"
              @click="handleDownload(row)"
              >{{ $t('exportApprovals.download') }}</ElButton
            >
            <ElButton
              v-if="row.status === 'pending' || row.status === 'pending_l2'"
              v-permission="EXPORT_APPROVAL_APPROVE_PERMISSION_KEY"
              type="primary"
              size="small"
              @click="handleApprove(row)"
              >{{ $t('exportApprovals.approve') }}</ElButton
            >
            <ElButton
              v-if="row.status === 'pending' || row.status === 'pending_l2'"
              v-permission="EXPORT_APPROVAL_REJECT_PERMISSION_KEY"
              type="danger"
              size="small"
              @click="handleReject(row)"
              >{{ $t('exportApprovals.reject') }}</ElButton
            >
            <ElButton
              v-if="row.status === 'pending'"
              v-permission="EXPORT_APPROVAL_CANCEL_PERMISSION_KEY"
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
    <ElDialog v-model="detailVisible" :title="$t('exportApprovals.detailTitle')" width="640">
      <ElDescriptions v-if="detailRow" :column="2" border>
        <ElDescriptionsItem label="ID">{{ detailRow.id }}</ElDescriptionsItem>
        <ElDescriptionsItem :label="$t('exportApprovals.colResource')">{{
          $t(exportApprovalResourceLabelKey(detailRow.resource_type))
        }}</ElDescriptionsItem>
        <ElDescriptionsItem :label="$t('exportApprovals.colApplicant')">{{
          detailRow.applicant_username
        }}</ElDescriptionsItem>
        <ElDescriptionsItem :label="$t('exportApprovals.colStatus')">
          <ElTag :type="exportApprovalStatusTagType(detailRow.status)">{{
            $t(exportApprovalStatusLabelKey(detailRow.status))
          }}</ElTag>
        </ElDescriptionsItem>
        <ElDescriptionsItem :label="$t('exportApprovals.colRows')">{{
          detailRow.estimated_rows ?? '-'
        }}</ElDescriptionsItem>
        <ElDescriptionsItem :label="$t('exportApprovals.fileFormatLabel')">{{
          detailRow.file_format
        }}</ElDescriptionsItem>
        <ElDescriptionsItem :label="$t('exportApprovals.paramsLabel')" :span="2">
          <pre class="detail-json">{{
            detailRow.export_params ? JSON.stringify(detailRow.export_params, null, 2) : '-'
          }}</pre>
        </ElDescriptionsItem>
      </ElDescriptions>
    </ElDialog>

    <!-- 发起审批（createApproval） -->
    <ElDialog v-model="createVisible" :title="$t('exportApprovals.createTitle')" width="520">
      <ElForm :model="createForm" label-width="110px">
        <ElFormItem :label="$t('exportApprovals.colResource')" required>
          <ElSelect v-model="createForm.resource_type" style="width: 100%">
            <ElOption
              v-for="rt in EXPORT_APPROVAL_RESOURCE_TYPES"
              :key="rt"
              :label="$t(EXPORT_APPROVAL_RESOURCE_LABEL_KEYS[rt])"
              :value="rt"
            />
          </ElSelect>
        </ElFormItem>
        <ElFormItem :label="$t('exportApprovals.paramsLabel')" required>
          <ElInput
            v-model="createForm.paramsText"
            type="textarea"
            :rows="4"
            :placeholder="$t('exportApprovals.paramsPlaceholder')"
          />
        </ElFormItem>
        <ElFormItem :label="$t('exportApprovals.estimatedRowsLabel')">
          <ElInputNumber v-model="createForm.estimated_rows" :min="0" />
        </ElFormItem>
        <ElFormItem :label="$t('exportApprovals.fileFormatLabel')">
          <!-- 后端 create 校验 file_format 仅接受 xlsx/pdf（export_approval_service） -->
          <ElSelect v-model="createForm.file_format" style="width: 100%">
            <ElOption label="xlsx" value="xlsx" />
            <ElOption label="pdf" value="pdf" />
          </ElSelect>
        </ElFormItem>
      </ElForm>
      <template #footer>
        <ElButton @click="createVisible = false">{{ $t('common.cancel') }}</ElButton>
        <ElButton type="primary" :loading="createSaving" @click="handleCreate">{{
          $t('common.submit')
        }}</ElButton>
      </template>
    </ElDialog>

    <!-- 下载令牌校验（verifyToken） -->
    <ElDialog v-model="tokenVisible" :title="$t('exportApprovals.verifyTitle')" width="440">
      <ElInput v-model="tokenText" :placeholder="$t('exportApprovals.verifyTokenPlaceholder')" />
      <template #footer>
        <ElButton @click="tokenVisible = false">{{ $t('common.cancel') }}</ElButton>
        <ElButton type="primary" :loading="tokenChecking" @click="handleVerifyToken">{{
          $t('exportApprovals.verifyAction')
        }}</ElButton>
      </template>
    </ElDialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue';
import { logger, logAuxLoadFailure } from '@/utils/logger';
import { ElMessage, ElMessageBox } from 'element-plus';
import { isDialogDismissal, rethrowNonDismissal } from '@/utils/monitor';
import { msg } from '@/utils/message';
import { downloadExportWithToken } from '@/utils/export';
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
import type { ExportDownloadTarget } from '@/constants/export-approval';
import {
  EXPORT_APPROVAL_STATUSES,
  EXPORT_APPROVAL_STATUS_LABEL_KEYS,
  EXPORT_APPROVAL_RESOURCE_TYPES,
  EXPORT_APPROVAL_RESOURCE_LABEL_KEYS,
  EXPORT_APPROVAL_APPROVE_PERMISSION_KEY,
  EXPORT_APPROVAL_REJECT_PERMISSION_KEY,
  EXPORT_APPROVAL_CANCEL_PERMISSION_KEY,
  exportApprovalResourceLabelKey,
  exportApprovalStatusLabelKey,
  exportApprovalStatusTagType,
  resolveExportDownloadTarget,
  pickExportQueryParams,
} from '@/constants/export-approval';
import { useUserStore } from '@/store/user';
import { useI18n } from 'vue-i18n';

const { t } = useI18n({ useScope: 'global' });

const userStore = useUserStore();
const myUserId = computed(() => userStore.userInfo?.id);

const list = ref<ExportApprovalRequest[]>([]);
const loading = ref(false);
const total = ref(0);

const searchForm = reactive<ListApprovalQuery>({
  page: 1,
  page_size: 20,
  status: '',
  resource_type: '',
});

const loadList = async () => {
  loading.value = true;
  try {
    const query = { ...searchForm };
    if (viewMode.value === 'pending_me') delete query.status;
    const res =
      viewMode.value === 'pending_me' ? await listPendingForMe(query) : await listApprovals(query);
    list.value = res.data.items;
    total.value = res.data.total;
  } catch (error) {
    // 后端失败信封文案由 request 拦截器外显，此处按辅助加载分级留痕
    logAuxLoadFailure(t('exportApprovals.loadFailed'), error);
  } finally {
    loading.value = false;
  }
};

// ===== 待我审批视图切换（listPendingForMe） =====
const viewMode = ref<'all' | 'pending_me'>('all');

const switchMode = (mode: 'all' | 'pending_me') => {
  viewMode.value = mode;
  pageResetAndLoad();
};

const pageResetAndLoad = () => {
  searchForm.page = 1;
  void loadList();
};

// ===== 详情回源（getApprovalDetail） =====
const detailVisible = ref(false);
const detailRow = ref<ExportApprovalRequest | null>(null);

const openDetail = async (row: ExportApprovalRequest) => {
  detailRow.value = row;
  detailVisible.value = true;
  try {
    const res = await getApprovalDetail(row.id);
    detailRow.value = res.data;
  } catch (error) {
    logAuxLoadFailure(t('exportApprovals.detailFailed'), error);
  }
};

// ===== 发起审批（createApproval） =====
const createVisible = ref(false);
const createSaving = ref(false);
const createForm = reactive({
  resource_type: 'customer',
  paramsText: '{}',
  estimated_rows: undefined as number | undefined,
  file_format: 'xlsx',
});

const handleCreate = async () => {
  let exportParams: unknown;
  try {
    exportParams = JSON.parse(createForm.paramsText);
  } catch {
    ElMessage.warning(t('exportApprovals.jsonInvalid'));
    return;
  }
  if (typeof exportParams !== 'object' || exportParams === null || Array.isArray(exportParams)) {
    ElMessage.warning(t('exportApprovals.jsonMustBeObject'));
    return;
  }
  createSaving.value = true;
  try {
    await createApproval({
      resource_type: createForm.resource_type,
      export_params: exportParams as Record<string, unknown>,
      estimated_rows: createForm.estimated_rows,
      file_format: createForm.file_format,
    });
    msg.success('exportApprovalSubmittedGuide');
    createVisible.value = false;
    viewMode.value = 'all';
    pageResetAndLoad();
  } catch (error) {
    // 失败文案由 request 拦截器按后端失败信封外显，此处留显式日志
    logger.error(t('exportApprovals.logRequestFailed'), error);
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
    ElMessage.warning(t('exportApprovals.tokenRequired'));
    return;
  }
  tokenChecking.value = true;
  try {
    const res = await verifyToken(tokenText.value.trim());
    ElMessageBox.alert(JSON.stringify(res.data, null, 2), t('exportApprovals.verifyResultTitle'));
  } catch (error) {
    // 令牌无效/过期等失败文案由 request 拦截器按后端失败信封外显
    logger.error(t('exportApprovals.logVerifyFailed'), error);
  } finally {
    tokenChecking.value = false;
  }
};

// ===== 申请人集中下载（凭一次性令牌调注册表解析出的资源导出端点） =====
const downloadingId = ref<number | null>(null);

// 令牌一次性：max_downloads 默认 1，download_count 达上限后不可再用；
// 下载端点校验令牌归属与资源类型由后端权威判定，前端只按出参字段控制按钮可达性，
// 不自行比对时间（避免客户端时钟偏差误判）
const canDownloadRow = (row: ExportApprovalRequest): boolean =>
  row.applicant_user_id === myUserId.value &&
  row.status === 'approved' &&
  row.download_token !== null &&
  row.download_count < row.max_downloads;

const handleDownload = async (row: ExportApprovalRequest) => {
  const token = row.download_token;
  // 模板已按 canDownloadRow 门控，此处收窄类型并防御直达调用
  if (!token) return;
  downloadingId.value = row.id;
  try {
    // 端点解析失败（如 finance_report 申请缺 report_type）单独外显，不进入下载分支
    let target: ExportDownloadTarget;
    try {
      target = resolveExportDownloadTarget(row.resource_type, row.export_params);
    } catch (error) {
      logger.error(t('exportApprovals.logEndpointFailed'), error);
      ElMessage.error(
        error instanceof Error && error.message
          ? error.message
          : t('exportApprovals.logEndpointFailed')
      );
      return;
    }
    await downloadExportWithToken({
      apiPath: target.apiPath,
      params: pickExportQueryParams(row.resource_type, row.export_params),
      token,
      filenamePrefix: target.filenamePrefix,
    });
    logger.info(t('exportApprovals.logDownloaded', { id: row.id }));
  } catch (error) {
    // 失败原因已由 downloadExportWithToken 按后端信封外显，此处留痕
    logger.error(t('exportApprovals.logDownloadFailed', { id: row.id }), error);
  } finally {
    downloadingId.value = null;
    // 令牌用后即消费（download_count 变化），无论成败都回源刷新真实状态
    await loadList();
  }
};

const handleApprove = async (row: ExportApprovalRequest) => {
  try {
    await ElMessageBox.confirm(t('exportApprovals.confirmApprove'), t('common.confirm'), {
      type: 'warning',
    });
    await approveRequest(row.id);
    msg.approveOk();
    loadList();
  } catch (error: unknown) {
    if (isDialogDismissal(error)) return; // 仅用户取消/关闭才静默中断
    // 走到这里说明 confirm 之后的业务请求或渲染抛错，不是「用户取消」，必须外显并上报
    rethrowNonDismissal('exportApprovals.handleApprove', error);
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
  } catch (error: unknown) {
    if (isDialogDismissal(error)) return;
    rethrowNonDismissal('exportApprovals.handleReject', error);
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
  } catch (error: unknown) {
    if (isDialogDismissal(error)) return;
    rethrowNonDismissal('exportApprovals.handleCancel', error);
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
