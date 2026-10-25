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
        <ElTableColumn prop="status" :label="$t('exportApprovals.colStatus')" width="110">
          <template #default="{ row }">
            <!-- 与资源类型列同款空值门：真实记录 status 恒非空（后端 NOT NULL），
                 仅空表占位行会带无 status 的对象；缺 status 时不渲染标签，
                 避免把「空表渲染」误判成「词表外取值」触发 normalize 显式抛错。 -->
            <ElTag v-if="row.status" :type="exportApprovalStatusTagType(row.status)">{{
              $t(exportApprovalStatusLabelKey(row.status))
            }}</ElTag>
          </template>
        </ElTableColumn>
        <ElTableColumn prop="estimated_rows" :label="$t('exportApprovals.colRows')" width="100" />
        <ElTableColumn prop="created_at" :label="$t('exportApprovals.colCreatedAt')" width="180" />
        <ElTableColumn :label="$t('exportApprovals.colToken')" min-width="180">
          <template #default="{ row }">
            <!-- 一次性凭据形态：列表永不出现明文，只显示前缀指纹与状态标记 -->
            <template v-if="row.token_fingerprint">
              <span class="token-text">{{ row.token_fingerprint }}****</span>
              <span v-if="row.token_consumed" class="token-state">{{
                $t('exportApprovals.tokenConsumedMark')
              }}</span>
              <span v-else-if="row.token_viewed" class="token-state">{{
                $t('exportApprovals.tokenViewedMark')
              }}</span>
            </template>
            <span v-else>-</span>
          </template>
        </ElTableColumn>
        <ElTableColumn :label="$t('common.action')" width="380" fixed="right">
          <template #default="{ row }">
            <ElButton size="small" @click="openDetail(row)">{{ $t('common.detail') }}</ElButton>
            <!-- 申请人下载已迁各资源导出页就地进行（composables/useExportApproval），
                 本页提供一次性查看、重发与跳转到导出入口。 -->
            <template v-if="isMine(row)">
              <ElButton
                v-if="row.status === 'approved' && !row.token_consumed"
                size="small"
                @click="openTokenOnce(row)"
                >{{ $t('exportApprovals.viewTokenAction') }}</ElButton
              >
              <ElButton
                v-if="row.status === 'approved' && !row.token_consumed"
                size="small"
                @click="handleRotateToken(row)"
                >{{ $t('exportApprovals.rotateTokenAction') }}</ElButton
              >
              <ElButton
                v-if="row.status === 'approved' && !row.token_consumed"
                size="small"
                type="primary"
                @click="goToExport(row)"
                >{{ $t('exportApprovals.goToExport') }}</ElButton
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
            <!--
              审批办理已并入 BPM 审批链条：pending 行由流程定义分派的审批人在
              审批中心（或本页经后端代理端点）办理，办理动作驱动流程节点推进。
            -->
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
        <!-- 一次性凭据与重试额度：只回指纹与计数，明文不外显 -->
        <ElDescriptionsItem :label="$t('exportApprovals.colToken')">
          {{ detailRow.token_fingerprint ? detailRow.token_fingerprint + '****' : '-' }}
        </ElDescriptionsItem>
        <ElDescriptionsItem :label="$t('exportApprovals.retryUsageLabel')">
          {{ detailRow.retry_count }} / {{ detailRow.max_retries }}
        </ElDescriptionsItem>
        <ElDescriptionsItem :label="$t('exportApprovals.approvalProgressLabel')">
          <ElButton v-if="detailRow.bpm_instance_id" text type="primary" @click="goApprovalCenter">
            {{ $t('exportApprovals.viewInApprovalCenter') }}
          </ElButton>
          <span v-else>-</span>
        </ElDescriptionsItem>
        <ElDescriptionsItem :label="$t('exportApprovals.paramsLabel')" :span="2">
          <pre class="detail-json">{{
            detailRow.export_params ? JSON.stringify(detailRow.export_params, null, 2) : '-'
          }}</pre>
        </ElDescriptionsItem>
      </ElDescriptions>
    </ElDialog>

    <!-- 一次性查看令牌（token-once：明文仅此一次） -->
    <ElDialog v-model="tokenOnceVisible" :title="$t('exportApprovals.tokenOnceTitle')" width="520">
      <ElAlert
        :title="$t('exportApprovals.tokenOnceWarning')"
        type="warning"
        :closable="false"
        class="token-once-alert"
      />
      <p v-if="tokenOnceText" class="token-once-text">{{ tokenOnceText }}</p>
      <p v-else-if="tokenOnceHint">{{ tokenOnceHint }}</p>
      <template #footer>
        <ElButton type="primary" @click="tokenOnceVisible = false">{{
          $t('exportApprovals.tokenOnceClose')
        }}</ElButton>
      </template>
    </ElDialog>

    <!-- 发起审批（createApproval） -->
    <ElDialog v-model="createVisible" :title="$t('exportApprovals.createTitle')" width="520">
      <ElForm ref="createFormRef" :model="createForm" label-width="110px">
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
        <!-- 硬红线：禁步进数字输入框，数值录入用 el-input 加校验器 -->
        <ElFormItem
          :label="$t('exportApprovals.estimatedRowsLabel')"
          prop="estimatedRowsText"
          :rules="estimatedRowsRules"
        >
          <ElInput v-model="createForm.estimatedRowsText" clearable />
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
import { ref, reactive, onMounted } from 'vue';
import { useRouter } from 'vue-router';
import type { FormInstance, FormItemRule } from 'element-plus';
import { logger, logAuxLoadFailure } from '@/utils/logger';
import { ElMessage, ElMessageBox } from 'element-plus';
import { isDialogDismissal, rethrowNonDismissal } from '@/utils/monitor';
import { msg } from '@/utils/message';
import { useUserStore } from '@/store/user';
import {
  listApprovals,
  listPendingForMe,
  createApproval,
  getApprovalDetail,
  verifyToken,
  approveRequest,
  rejectRequest,
  cancelRequest,
  viewTokenOnce,
  rotateToken,
} from '@/api/export-approvals';
import type { ExportApprovalRequest, ListApprovalQuery } from '@/api/export-approvals';
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
  exportApprovalTargetRoute,
  BPM_APPROVAL_CENTER_ROUTE,
} from '@/constants/export-approval';
import { useI18n } from 'vue-i18n';

const { t } = useI18n({ useScope: 'global' });
const router = useRouter();
const userStore = useUserStore();

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

// ===== 归属判定（查看/重发/跳转仅申请人本人可见，后端同样强制） =====
const isMine = (row: ExportApprovalRequest) => row.applicant_user_id === userStore.userInfo?.id;

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

const goApprovalCenter = () => {
  router.push(BPM_APPROVAL_CENTER_ROUTE);
};

// ===== 一次性查看令牌（token-once：服务端置位后明文永不再出） =====
const tokenOnceVisible = ref(false);
const tokenOnceText = ref('');
const tokenOnceHint = ref('');

const openTokenOnce = async (row: ExportApprovalRequest) => {
  try {
    const res = await viewTokenOnce(row.id);
    tokenOnceVisible.value = true;
    if (res.data.available && res.data.token) {
      tokenOnceText.value = res.data.token;
      tokenOnceHint.value = '';
      logger.info(t('exportApprovals.logTokenViewed', { id: row.id }));
    } else {
      tokenOnceText.value = '';
      tokenOnceHint.value = t('exportApprovals.tokenOnceUnavailable', {
        fingerprint: res.data.fingerprint ?? '-',
      });
    }
    // 明文已置位，刷新列表同步 viewed 标记
    void loadList();
  } catch (error) {
    // 失败信封（状态门/身份门）由拦截器外显
    logger.error(t('exportApprovals.logTokenOnceFailed', { id: row.id }), error);
  }
};

// ===== 重发令牌（旧令牌即刻作废；额度与硬有效期不刷新） =====
const handleRotateToken = async (row: ExportApprovalRequest) => {
  try {
    await ElMessageBox.confirm(
      t('exportApprovals.confirmRotateToken'),
      t('exportApprovals.rotateTokenAction'),
      { type: 'warning' }
    );
  } catch (error: unknown) {
    if (isDialogDismissal(error)) return;
    rethrowNonDismissal('exportApprovals.handleRotateToken.confirm', error);
    return;
  }
  try {
    const res = await rotateToken(row.id);
    // 重发成功即一次性展示新明文，不再经复制按钮扩散
    tokenOnceVisible.value = true;
    tokenOnceText.value = res.data.token;
    tokenOnceHint.value = '';
    msg.success('exportApprovalRotatedGuide');
    logger.info(t('exportApprovals.logRotateOk', { id: row.id }));
    void loadList();
  } catch (error) {
    // 重发被拒（用尽/过期/已消费）的区分文案由后端失败信封外显
    logger.error(t('exportApprovals.logRotateFailed', { id: row.id }), error);
  }
};

// ===== 跳转到对应资源的导出入口（审批通过后的正规路径） =====
const goToExport = (row: ExportApprovalRequest) => {
  try {
    router.push(exportApprovalTargetRoute(row.resource_type));
  } catch (error) {
    // 词表外资源类型 normalize 抛错，留痕外显
    logger.error(t('exportApprovals.logRouteFailed', { id: row.id }), error);
    ElMessage.error(error instanceof Error ? error.message : String(error));
  }
};

// ===== 发起审批（createApproval） =====
const createVisible = ref(false);
const createSaving = ref(false);
const createFormRef = ref<FormInstance>();
const createForm = reactive({
  resource_type: 'customer',
  paramsText: '{}',
  estimatedRowsText: '',
  file_format: 'xlsx',
});

/** 预计行数校验：可空（空按 0 行低风险），非空必须为非负整数字符串（范围靠校验器不靠按钮） */
const estimatedRowsRules: FormItemRule[] = [
  {
    validator: (_rule, value, callback) => {
      const text = String(value ?? '').trim();
      if (text === '') {
        callback();
        return;
      }
      if (!/^\d+$/.test(text)) {
        callback(new Error(t('exportApprovals.estimatedRowsInvalid')));
        return;
      }
      callback();
    },
    trigger: 'blur',
  },
];

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
  const valid = await createFormRef.value?.validate().catch(() => false);
  if (valid === false) return;
  const rowsText = createForm.estimatedRowsText.trim();
  createSaving.value = true;
  try {
    await createApproval({
      resource_type: createForm.resource_type,
      export_params: exportParams as Record<string, unknown>,
      estimated_rows: rowsText === '' ? undefined : Number(rowsText),
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
    // 令牌无效/过期/已消费/窗口未到/用尽的区分文案由 request 拦截器按后端失败信封外显
    logger.error(t('exportApprovals.logVerifyFailed'), error);
  } finally {
    tokenChecking.value = false;
  }
};

// 申请人下载已迁各资源导出页就地（composables/useExportApproval.downloadApprovedExport），
// 本页提供审批职能（批准/驳回/撤销 + 待办列表）、一次性查看与重发、跳转导出入口。
// 令牌明文不提供复制按钮（一次性凭据形态，复制扩散正是本改造要消灭的形态）。

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
.token-state {
  margin-left: 6px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
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
.token-once-alert {
  margin-bottom: 12px;
}
.token-once-text {
  font-family: monospace;
  font-size: 14px;
  background: var(--el-fill-color-light);
  border-radius: 4px;
  padding: 12px;
  word-break: break-all;
  user-select: none;
}
</style>
