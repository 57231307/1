<template>
  <div class="bulk-color-page">
    <el-card shadow="never">
      <div class="page-header">
        <h2>{{ t('bulkColorApproval.index.pageTitle') }}</h2>
        <div class="header-actions">
          <el-button type="primary" @click="handleCreate">{{
            t('bulkColorApproval.index.buttonCreate')
          }}</el-button>
          <el-button plain :loading="statsLoading" @click="openStatistics">{{
            t('bulkColorApproval.index.buttonStatistics')
          }}</el-button>
        </div>
      </div>

      <el-table v-loading="loading" :data="list" border>
        <el-table-column
          :label="t('bulkColorApproval.index.colApprovalNo')"
          prop="approval_no"
          min-width="160"
        />
        <el-table-column
          :label="t('bulkColorApproval.index.colOrderNo')"
          prop="order_no"
          min-width="140"
        >
          <template #default="{ row }">{{ row.order_no || '-' }}</template>
        </el-table-column>
        <el-table-column
          :label="t('bulkColorApproval.index.colColorNo')"
          prop="color_no"
          width="120"
        >
          <template #default="{ row }">{{ row.color_no || '-' }}</template>
        </el-table-column>
        <el-table-column
          :label="t('bulkColorApproval.index.colDyeLotNo')"
          prop="dye_lot_no"
          width="130"
        >
          <template #default="{ row }">{{ row.dye_lot_no || '-' }}</template>
        </el-table-column>
        <el-table-column prop="status" :label="t('common.status')" width="120" align="center">
          <template #default="{ row }">
            <el-tag :type="statusTag(row.status)">{{ statusText(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="applicant_name"
          :label="t('bulkColorApproval.index.colApplicant')"
          width="100"
        />
        <el-table-column :label="t('common.operation')" width="360" fixed="right">
          <template #default="{ row }">
            <el-button size="small" link type="primary" @click="openDetail(row)">
              {{ t('common.detail') }}
            </el-button>
            <el-button size="small" link type="primary" @click="handleCut(row)">
              {{ t('bulkColorApproval.index.buttonCut') }}
            </el-button>
            <el-button size="small" link type="primary" @click="handleSend(row)">
              {{ t('bulkColorApproval.index.buttonSend') }}
            </el-button>
            <el-button size="small" link type="success" @click="handleApprove(row)">
              {{ t('bulkColorApproval.index.buttonApprove') }}
            </el-button>
            <el-button size="small" link type="warning" @click="handleRework(row)">
              {{ t('bulkColorApproval.index.buttonRework') }}
            </el-button>
            <el-button size="small" link type="danger" @click="handleReject(row)">
              {{ t('bulkColorApproval.index.buttonReject') }}
            </el-button>
            <el-button
              v-if="row.status === 'approved'"
              size="small"
              link
              type="warning"
              @click="handleDowngrade(row)"
            >
              {{ t('bulkColorApproval.index.buttonDowngrade') }}
            </el-button>
            <el-button
              v-if="['approved', 'pending', 'sampled'].includes(row.status)"
              size="small"
              link
              type="danger"
              @click="handleScrap(row)"
            >
              {{ t('bulkColorApproval.index.buttonScrap') }}
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog
      v-model="dialogVisible"
      :title="t('bulkColorApproval.index.titleCreate')"
      width="520px"
    >
      <el-form :model="form" label-width="90px">
        <el-form-item :label="t('bulkColorApproval.index.labelSalesOrder')" required>
          <el-input-number
            v-model="form.sales_order_id"
            :min="1"
            :precision="0"
            style="width: 100%"
            :placeholder="t('bulkColorApproval.index.placeholderSalesOrderId')"
          />
        </el-form-item>
        <el-form-item :label="t('bulkColorApproval.index.labelDyeBatch')" required>
          <el-input-number
            v-model="form.dye_batch_id"
            :min="1"
            :precision="0"
            style="width: 100%"
            :placeholder="t('bulkColorApproval.index.placeholderDyeBatchId')"
          />
        </el-form-item>
        <el-form-item :label="t('bulkColorApproval.index.labelCustomer')" required>
          <el-input-number
            v-model="form.customer_id"
            :min="1"
            :precision="0"
            style="width: 100%"
            :placeholder="t('bulkColorApproval.index.placeholderCustomerId')"
          />
        </el-form-item>
        <el-form-item :label="t('bulkColorApproval.index.colColorNo')">
          <el-input
            v-model="form.color_no"
            :placeholder="t('bulkColorApproval.index.placeholderOptional')"
          />
        </el-form-item>
        <el-form-item :label="t('bulkColorApproval.index.colDyeLotNo')">
          <el-input
            v-model="form.dye_lot_no"
            :placeholder="t('bulkColorApproval.index.placeholderOptional')"
          />
        </el-form-item>
        <el-form-item :label="t('bulkColorApproval.index.labelBatchNo')">
          <el-input
            v-model="form.batch_no"
            :placeholder="t('bulkColorApproval.index.placeholderOptional')"
          />
        </el-form-item>
        <el-form-item :label="t('bulkColorApproval.index.labelRemarks')">
          <el-input v-model="form.remark" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="submitting" @click="submitCreate">{{
          t('bulkColorApproval.index.buttonConfirm')
        }}</el-button>
      </template>
    </el-dialog>

    <!-- 批色详情（getBulkColorApproval 回源 + getBulkColorApprovalHistory 历史） -->
    <el-dialog
      v-model="detailVisible"
      :title="t('bulkColorApproval.index.titleDetail')"
      width="720"
    >
      <el-descriptions v-if="detailRow" :column="2" border>
        <el-descriptions-item label="ID">{{ detailRow.id }}</el-descriptions-item>
        <el-descriptions-item :label="t('bulkColorApproval.index.colOrderNo')">{{
          detailRow.order_no
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('bulkColorApproval.index.colColorNo')">{{
          detailRow.color_no
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('bulkColorApproval.index.colDyeLotNo')">{{
          detailRow.dye_lot_no
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('bulkColorApproval.index.colApplicant')">{{
          detailRow.applicant_name || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('common.status')">{{
          statusText(String(detailRow.status ?? ''))
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('bulkColorApproval.index.labelRemarks')" :span="2">{{
          detailRow.notes || '-'
        }}</el-descriptions-item>
      </el-descriptions>

      <h4 class="section-title">{{ t('bulkColorApproval.index.sectionHistory') }}</h4>
      <el-table v-loading="historyLoading" :data="historyRows" border size="small" max-height="240">
        <el-table-column prop="id" label="ID" width="70" />
        <el-table-column
          prop="from_status"
          :label="t('bulkColorApproval.index.colFromStatus')"
          width="130"
        >
          <template #default="{ row }">{{ row.from_status || '-' }}</template>
        </el-table-column>
        <el-table-column
          prop="to_status"
          :label="t('bulkColorApproval.index.colToStatus')"
          width="130"
        >
          <template #default="{ row }">{{ row.to_status || '-' }}</template>
        </el-table-column>
        <el-table-column
          prop="operator_name"
          :label="t('bulkColorApproval.index.colOperator')"
          width="110"
        >
          <template #default="{ row }">{{ row.operator_name || '-' }}</template>
        </el-table-column>
        <el-table-column
          prop="reason"
          :label="t('bulkColorApproval.index.colReason')"
          min-width="140"
          show-overflow-tooltip
        >
          <template #default="{ row }">{{ row.reason || '-' }}</template>
        </el-table-column>
        <el-table-column
          prop="created_at"
          :label="t('bulkColorApproval.index.colCreatedAt')"
          min-width="150"
        >
          <template #default="{ row }">{{ row.created_at || '-' }}</template>
        </el-table-column>
      </el-table>
      <template #footer>
        <el-button @click="detailVisible = false">{{ t('common.close') }}</el-button>
      </template>
    </el-dialog>

    <!-- 统计对话框（getBulkColorApprovalStatistics） -->
    <el-dialog
      v-model="statsVisible"
      :title="t('bulkColorApproval.index.titleStatistics')"
      width="520"
    >
      <pre v-if="statsResult" class="stats-json">{{ statsResult }}</pre>
      <template #footer>
        <el-button @click="statsVisible = false">{{ t('common.close') }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue';
import { logger } from '@/utils/logger';
import { ElMessage, ElMessageBox } from 'element-plus';
import { useI18n } from 'vue-i18n';
import {
  getBulkColorApprovalList,
  getBulkColorApproval,
  getBulkColorApprovalHistory,
  getBulkColorApprovalStatistics,
  downgradeBulkColor,
  scrapBulkColor,
  createBulkColorApproval,
  cutBulkColorSample,
  sendBulkColorToCustomer,
  approveBulkColor,
  rejectBulkColor,
  reworkBulkColor,
  type BulkColorApproval,
  type BulkColorApprovalStatus,
} from '@/api/bulk-color-approval';

const { t } = useI18n({ useScope: 'global' });
const loading = ref(false);
const submitting = ref(false);
const dialogVisible = ref(false);
const list = ref<BulkColorApproval[]>([]);

const form = reactive<{
  sales_order_id: number | undefined;
  dye_batch_id: number | undefined;
  customer_id: number | undefined;
  color_no: string;
  dye_lot_no: string;
  batch_no: string;
  remark: string;
}>({
  sales_order_id: undefined,
  dye_batch_id: undefined,
  customer_id: undefined,
  color_no: '',
  dye_lot_no: '',
  batch_no: '',
  remark: '',
});

const statusText = (s: string) =>
  ({
    draft: t('bulkColorApproval.index.statusDraft'),
    pending: t('bulkColorApproval.index.statusPending'),
    cut: t('bulkColorApproval.index.statusCut'),
    sent_to_customer: t('bulkColorApproval.index.statusSentToCustomer'),
    customer_approved: t('bulkColorApproval.index.statusCustomerApproved'),
    customer_rejected: t('bulkColorApproval.index.statusCustomerRejected'),
    approved: t('bulkColorApproval.index.statusApproved'),
    rejected: t('bulkColorApproval.index.statusRejected'),
    rework: t('bulkColorApproval.index.statusRework'),
    downgraded: t('bulkColorApproval.index.statusDowngraded'),
    scrapped: t('bulkColorApproval.index.statusScrapped'),
    cancelled: t('bulkColorApproval.index.statusCancelled'),
  })[s] ?? s;
const statusTag = (s: BulkColorApprovalStatus | string) =>
  (
    ({
      approved: 'success',
      customer_approved: 'success',
      rejected: 'danger',
      customer_rejected: 'danger',
      scrapped: 'danger',
      rework: 'warning',
      downgraded: 'warning',
    }) as Record<string, string>
  )[s] ?? 'info';

const unwrap = (payload: unknown): BulkColorApproval[] => {
  const p = payload as unknown;
  return Array.isArray(p) ? p : ((p as { items?: BulkColorApproval[] })?.items ?? []);
};

const loadList = async () => {
  loading.value = true;
  try {
    const res = await getBulkColorApprovalList({ page: 1, page_size: 50 });
    list.value = unwrap(res.data);
  } catch (e) {
    ElMessage.error(
      t('bulkColorApproval.index.messageFetchListFailed', { msg: (e as Error).message })
    );
  } finally {
    loading.value = false;
  }
};

const handleCreate = () => {
  Object.assign(form, {
    sales_order_id: undefined,
    dye_batch_id: undefined,
    customer_id: undefined,
    color_no: '',
    dye_lot_no: '',
    batch_no: '',
    remark: '',
  });
  dialogVisible.value = true;
};

const submitCreate = async () => {
  if (!form.sales_order_id || !form.dye_batch_id || !form.customer_id) {
    ElMessage.warning(t('bulkColorApproval.index.messageRequiredFields'));
    return;
  }
  submitting.value = true;
  try {
    await createBulkColorApproval({
      sales_order_id: form.sales_order_id,
      dye_batch_id: form.dye_batch_id,
      customer_id: form.customer_id,
      color_no: form.color_no || undefined,
      dye_lot_no: form.dye_lot_no || undefined,
      batch_no: form.batch_no || undefined,
      remark: form.remark || undefined,
    });
    ElMessage.success(t('bulkColorApproval.index.messageCreateSuccess'));
    dialogVisible.value = false;
    await loadList();
  } catch (e) {
    ElMessage.error(
      t('bulkColorApproval.index.messageCreateFailed', { msg: (e as Error).message })
    );
  } finally {
    submitting.value = false;
  }
};

const runAction = async (_row: BulkColorApproval, label: string, fn: () => Promise<unknown>) => {
  try {
    await fn();
    ElMessage.success(t('bulkColorApproval.index.messageActionSuccess', { action: label }));
    await loadList();
  } catch (e) {
    ElMessage.error(
      t('bulkColorApproval.index.messageActionFailed', {
        action: label,
        msg: (e as Error).message,
      })
    );
  }
};

const handleCut = (row: BulkColorApproval) =>
  runAction(row, t('bulkColorApproval.index.buttonCut'), () =>
    cutBulkColorSample(row.id, { sample_length_m: 1 })
  );
const handleSend = (row: BulkColorApproval) =>
  runAction(row, t('bulkColorApproval.index.buttonSend'), () => sendBulkColorToCustomer(row.id));
const handleApprove = (row: BulkColorApproval) =>
  runAction(row, t('bulkColorApproval.index.buttonApprove'), () =>
    approveBulkColor(row.id, { feedback: '客户确认批色通过' })
  );
const handleRework = (row: BulkColorApproval) =>
  runAction(row, t('bulkColorApproval.index.buttonRework'), () =>
    reworkBulkColor(row.id, { reject_reason: '批色回修' })
  );
const handleReject = (row: BulkColorApproval) =>
  runAction(row, t('bulkColorApproval.index.buttonReject'), async () => {
    const { value } = await ElMessageBox.prompt(
      t('bulkColorApproval.index.messageRejectPrompt'),
      t('bulkColorApproval.index.titleReject'),
      { inputValue: '' }
    );
    await rejectBulkColor(row.id, { reject_reason: value || '客户拒绝' });
  });

// ===== 降级（downgradeBulkColor：仅 approved 态） =====
const handleDowngrade = (row: BulkColorApproval) =>
  runAction(row, t('bulkColorApproval.index.buttonDowngrade'), async () => {
    const { value } = await ElMessageBox.prompt(
      t('bulkColorApproval.index.messageDowngradePrompt'),
      t('bulkColorApproval.index.titleDowngrade'),
      {
        inputValidator: v =>
          !!v.trim() || t('bulkColorApproval.index.messageDowngradeReasonRequired'),
      }
    );
    await downgradeBulkColor(row.id, { reject_reason: value });
  });

// ===== 报废（scrapBulkColor：approved/pending/sampled 态） =====
const handleScrap = (row: BulkColorApproval) =>
  runAction(row, t('bulkColorApproval.index.buttonScrap'), async () => {
    const { value } = await ElMessageBox.prompt(
      t('bulkColorApproval.index.messageScrapPrompt'),
      t('bulkColorApproval.index.titleScrap'),
      { inputValidator: v => !!v.trim() || t('bulkColorApproval.index.messageScrapReasonRequired') }
    );
    await scrapBulkColor(row.id, { reject_reason: value });
  });

// ===== 详情回源 + 流转历史 =====
const detailVisible = ref(false);
const detailRow = ref<Record<string, unknown> | null>(null);
const historyRows = ref<Array<Record<string, unknown>>>([]);
const historyLoading = ref(false);

const openDetail = async (row: BulkColorApproval) => {
  detailRow.value = row as unknown as Record<string, unknown>;
  detailVisible.value = true;
  historyLoading.value = true;
  try {
    const res = await getBulkColorApproval(row.id);
    if (res.data) detailRow.value = res.data as unknown as Record<string, unknown>;
  } catch (error) {
    logger.error(t('message.loadBulkColorDetailFailed'), error);
  }
  try {
    const res = await getBulkColorApprovalHistory(row.id);
    const d = res.data as unknown;
    historyRows.value = Array.isArray(d)
      ? d
      : ((d as { items?: Array<Record<string, unknown>> })?.items ?? []);
  } catch (error) {
    logger.error(t('message.loadBulkColorHistoryFailed'), error);
    historyRows.value = [];
  } finally {
    historyLoading.value = false;
  }
};

// ===== 统计 =====
const statsVisible = ref(false);
const statsLoading = ref(false);
const statsResult = ref('');

const openStatistics = async () => {
  statsVisible.value = true;
  statsLoading.value = true;
  try {
    const res = await getBulkColorApprovalStatistics();
    statsResult.value = JSON.stringify(res.data ?? res, null, 2);
  } catch (e) {
    ElMessage.error((e as Error).message || t('bulkColorApproval.index.messageFetchStatsFailed'));
  } finally {
    statsLoading.value = false;
  }
};

onMounted(() => {
  loadList();
});
</script>

<style scoped>
.section-title {
  margin: 14px 0 8px;
  font-size: 14px;
}
.stats-json {
  background: var(--el-fill-color-light);
  border-radius: 4px;
  padding: 12px;
  font-size: 12px;
  max-height: 320px;
  overflow: auto;
  white-space: pre-wrap;
  margin: 0;
}
.bulk-color-page {
  padding: 20px;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}
.page-header h2 {
  margin: 0;
  font-size: 18px;
}
.header-actions {
  display: flex;
  gap: 12px;
  align-items: center;
}
</style>
