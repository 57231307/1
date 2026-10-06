<!--
  PeriodListTab.vue - 会计期间 Tab
  来源：原 accountingPeriod/index.vue 主体内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="period-list-tab">
    <div class="page-header">
      <h2 class="page-title">{{ $t('accountingPeriod.title') }}</h2>
      <div>
        <el-button type="primary" @click="openDialog()">
          <el-icon><Plus /></el-icon>{{ $t('accountingPeriod.create') }}
        </el-button>
        <el-button @click="handleInitYear">
          <el-icon><Refresh /></el-icon>{{ $t('accountingPeriod.initYear') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form
        :inline="true"
        :model="queryForm"
        :aria-label="$t('accountingPeriod.filter.ariaLabel')"
      >
        <el-form-item :label="$t('accountingPeriod.filter.year')">
          <el-input-number v-model="queryForm.year" :min="2000" :max="2100" style="width: 140px" />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleSearch">{{
            $t('accountingPeriod.filter.query')
          }}</el-button>
          <el-button @click="handleReset">{{ $t('accountingPeriod.filter.reset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover">
      <el-table
        v-loading="loading"
        :data="filteredPeriods"
        stripe
        :aria-label="$t('accountingPeriod.table.ariaLabel')"
      >
        <el-table-column
          prop="period_name"
          :label="$t('accountingPeriod.table.name')"
          width="160"
        />
        <el-table-column
          prop="year"
          :label="$t('accountingPeriod.table.year')"
          width="80"
          align="center"
        />
        <el-table-column
          prop="period"
          :label="$t('accountingPeriod.table.month')"
          width="80"
          align="center"
        />
        <el-table-column
          prop="start_date"
          :label="$t('accountingPeriod.table.startDate')"
          width="160"
        >
          <template #default="{ row }">{{ formatDate(row.start_date) }}</template>
        </el-table-column>
        <el-table-column prop="end_date" :label="$t('accountingPeriod.table.endDate')" width="160">
          <template #default="{ row }">{{ formatDate(row.end_date) }}</template>
        </el-table-column>
        <el-table-column prop="status" :label="$t('accountingPeriod.table.status')" width="100">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="closed_at"
          :label="$t('accountingPeriod.table.closedAt')"
          width="180"
        >
          <template #default="{ row }">{{
            row.closed_at ? formatDate(row.closed_at) : '-'
          }}</template>
        </el-table-column>
        <el-table-column :label="$t('accountingPeriod.table.operation')" width="200" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="openDialog(row)">{{
              $t('accountingPeriod.table.edit')
            }}</el-button>
            <el-button
              v-if="row.status === 'OPEN'"
              type="warning"
              link
              size="small"
              @click="closePeriod(row)"
              >{{ $t('accountingPeriod.table.close') }}</el-button
            >
            <el-button
              v-if="row.status === 'CLOSED'"
              type="info"
              link
              size="small"
              @click="reopenPeriod(row)"
              >{{ $t('accountingPeriod.table.reopen') }}</el-button
            >
            <el-button
              v-if="row.status !== 'CLOSED'"
              type="danger"
              link
              size="small"
              @click="deletePeriod(row)"
              >{{ $t('accountingPeriod.table.delete') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog
      v-model="dialogVisible"
      :title="
        isEdit ? $t('accountingPeriod.dialog.editTitle') : $t('accountingPeriod.dialog.createTitle')
      "
      width="500px"
      :aria-label="
        isEdit
          ? $t('accountingPeriod.dialog.editAriaLabel')
          : $t('accountingPeriod.dialog.createAriaLabel')
      "
    >
      <el-form
        ref="formRef"
        :model="form"
        :rules="rules"
        label-width="100px"
        :aria-label="$t('accountingPeriod.dialog.ariaLabel')"
      >
        <template v-if="!isEdit">
          <el-form-item :label="$t('accountingPeriod.dialog.year')" prop="year">
            <el-input-number v-model="form.year" :min="2000" :max="2100" style="width: 100%" />
          </el-form-item>
          <el-form-item :label="$t('accountingPeriod.dialog.month')" prop="period">
            <el-input-number v-model="form.period" :min="1" :max="12" style="width: 100%" />
          </el-form-item>
          <!-- 起止日期由后端按 year/period 推导（CreateAccountingPeriodPayload 无日期字段），
               这里仅只读预览，不可编辑，避免"填了不生效"的假采集 -->
          <el-form-item :label="$t('accountingPeriod.dialog.startDate')">
            <el-input :value="previewRange.start_date" disabled />
          </el-form-item>
          <el-form-item :label="$t('accountingPeriod.dialog.endDate')">
            <el-input :value="previewRange.end_date" disabled />
          </el-form-item>
        </template>
        <template v-else>
          <!-- 更新端点仅支持改 period_name / status(OPEN|CLOSING)，年度/期间/日期后端不可改 -->
          <el-form-item :label="$t('accountingPeriod.dialog.name')" prop="period_name">
            <el-input v-model="form.period_name" maxlength="50" />
          </el-form-item>
        </template>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{
          $t('accountingPeriod.dialog.cancel')
        }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
          $t('accountingPeriod.dialog.confirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { isDialogDismissal } from '@/utils/monitor';
import { ref, reactive, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus';
import { Plus, Refresh } from '@element-plus/icons-vue';
import {
  getAccountingPeriodList,
  createAccountingPeriod,
  updateAccountingPeriod,
  deleteAccountingPeriod,
  closePeriod as closePeriodApi,
  reopenPeriod as reopenPeriodApi,
  type AccountingPeriodDetail,
} from '@/api/accounting-period';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

const loading = ref(false);
const submitLoading = ref(false);
const dialogVisible = ref(false);
const periodList = ref<AccountingPeriodDetail[]>([]);
const formRef = ref<FormInstance>();

const queryForm = reactive({
  year: new Date().getFullYear(),
});

const isEdit = computed(() => !!form.id);

const form = reactive<{ id?: number; year: number; period: number; period_name: string }>({
  id: undefined,
  year: new Date().getFullYear(),
  period: 1,
  period_name: '',
});

const rules = computed<FormRules>(() => ({
  year: [
    { required: true, message: t('accountingPeriod.validation.yearRequired'), trigger: 'blur' },
  ],
  period: [
    { required: true, message: t('accountingPeriod.validation.monthRequired'), trigger: 'blur' },
  ],
}));

// 状态词表写入方：services/accounting_period_service.rs + models/status/finance.rs
// （OPEN / CLOSING / CLOSED 全大写），比较点与之逐字符一致
const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    OPEN: t('accountingPeriod.status.open'),
    CLOSING: t('accountingPeriod.status.closing'),
    CLOSED: t('accountingPeriod.status.closed'),
  };
  return map[status] || status;
};

const getStatusType = (status: string) => {
  const map: Record<string, 'success' | 'warning' | 'info'> = {
    OPEN: 'success',
    CLOSING: 'warning',
    CLOSED: 'info',
  };
  return map[status] || 'info';
};

const formatDate = (value: string) => (value ? value.replace('T', ' ').slice(0, 10) : '-');

// 后端列表端点无 Query<T>（参数被 Axum 整体丢弃），年度筛选只能在已加载全量列表本地做
const filteredPeriods = computed(() => periodList.value.filter(p => p.year === queryForm.year));

const fetchPeriods = async () => {
  loading.value = true;
  try {
    const res = await getAccountingPeriodList();
    const d = (res as { data?: unknown }).data as
      | AccountingPeriodDetail[]
      | {
          items?: AccountingPeriodDetail[];
          data?: AccountingPeriodDetail[];
          list?: AccountingPeriodDetail[];
        };
    if (Array.isArray(d)) {
      periodList.value = d;
    } else {
      periodList.value = d?.items || d?.data || d?.list || [];
    }
  } catch (e) {
    const err = e as Error;
    ElMessage.error(err.message || t('accountingPeriod.message.fetchListFailed'));
  } finally {
    loading.value = false;
  }
};

const handleSearch = () => {
  fetchPeriods();
};

const handleReset = () => {
  queryForm.year = new Date().getFullYear();
  fetchPeriods();
};

const computeDateRange = (year: number, month: number) => {
  const start = new Date(year, month - 1, 1);
  const end = new Date(year, month, 0);
  return {
    start_date: start.toISOString().split('T')[0],
    end_date: end.toISOString().split('T')[0],
  };
};

// 创建对话框的起止日期预览（后端推导逻辑的本地镜像，仅展示，不随请求发送）
const previewRange = computed(() => computeDateRange(form.year, form.period));

const openDialog = (row?: AccountingPeriodDetail) => {
  formRef.value?.resetFields();
  if (row) {
    form.id = row.id;
    form.year = row.year;
    form.period = row.period;
    form.period_name = row.period_name;
  } else {
    const now = new Date();
    form.id = undefined;
    form.year = now.getFullYear();
    form.period = now.getMonth() + 1;
    form.period_name = '';
  }
  dialogVisible.value = true;
};

const handleSubmit = async () => {
  if (!formRef.value) return;
  await formRef.value.validate(async valid => {
    if (!valid) return;
    submitLoading.value = true;
    try {
      if (form.id) {
        // period_name 为 Option<String> 且 #[validate(length(min=1,max=50))]：
        // 空串会触发 422，未填写时省略该键（Some("") 判空 = 假更新）
        const trimmed = form.period_name.trim();
        await updateAccountingPeriod(form.id, {
          ...(trimmed ? { period_name: trimmed } : {}),
        });
        ElMessage.success(t('accountingPeriod.message.updateSuccess'));
      } else {
        // 后端 CreateAccountingPeriodPayload 仅 year + period(1-12)，日期由其推导
        await createAccountingPeriod({ year: form.year, period: form.period });
        ElMessage.success(t('accountingPeriod.message.createSuccess'));
      }
      dialogVisible.value = false;
      fetchPeriods();
    } catch (e) {
      const err = e as Error;
      ElMessage.error(err.message || t('accountingPeriod.message.operationFailed'));
    } finally {
      submitLoading.value = false;
    }
  });
};

const periodLabel = (row: AccountingPeriodDetail) =>
  row.period_name || `${row.year}-${String(row.period).padStart(2, '0')}`;

const closePeriod = async (row: AccountingPeriodDetail) => {
  try {
    await ElMessageBox.confirm(
      t('accountingPeriod.message.closeConfirm', { name: periodLabel(row) }),
      t('accountingPeriod.message.closeConfirmTitle'),
      { type: 'warning' }
    );
    await closePeriodApi(row.id);
    logger.info('月末结账操作成功', { periodId: row.id, periodName: periodLabel(row) });
    ElMessage.success(t('accountingPeriod.message.closedSuccess'));
    fetchPeriods();
  } catch (e) {
    if (!isDialogDismissal(e)) {
      const err = e as Error;
      logger.error('月末结账操作失败', { periodId: row.id, error: err.message });
      ElMessage.error(err.message || t('accountingPeriod.message.closeFailed'));
    }
  }
};

// 反结账：后端 ReopenPeriodRequest.reason 为必填非 Option String，
// 必须真实采集反结账原因（缺失即 422），不得塞默认值
const reopenPeriod = async (row: AccountingPeriodDetail) => {
  try {
    const { value } = await ElMessageBox.prompt(
      t('accountingPeriod.message.reopenReasonPrompt', { name: periodLabel(row) }),
      t('accountingPeriod.message.reopenConfirmTitle'),
      {
        type: 'warning',
        inputType: 'textarea',
        inputPlaceholder: t('accountingPeriod.message.reopenReasonPlaceholder'),
        inputValidator: (v: string) =>
          (v && v.trim()) || t('accountingPeriod.message.reopenReasonRequired'),
      }
    );
    await reopenPeriodApi(row.id, value.trim());
    ElMessage.success(t('accountingPeriod.message.reopenedSuccess'));
    fetchPeriods();
  } catch (e) {
    if (!isDialogDismissal(e)) {
      const err = e as Error;
      ElMessage.error(err.message || t('accountingPeriod.message.operationFailed'));
    }
  }
};

const deletePeriod = async (row: AccountingPeriodDetail) => {
  try {
    await ElMessageBox.confirm(
      t('accountingPeriod.message.deleteConfirm', { name: periodLabel(row) }),
      t('accountingPeriod.message.deleteConfirmTitle'),
      { type: 'warning' }
    );
    await deleteAccountingPeriod(row.id);
    ElMessage.success(t('accountingPeriod.message.deleteSuccess'));
    fetchPeriods();
  } catch (e) {
    if (!isDialogDismissal(e)) {
      const err = e as Error;
      ElMessage.error(err.message || t('accountingPeriod.message.deleteFailed'));
    }
  }
};

const handleInitYear = async () => {
  try {
    const year = queryForm.year;
    await ElMessageBox.confirm(
      t('accountingPeriod.message.initYearConfirm', { year }),
      t('accountingPeriod.message.initYearConfirmTitle'),
      { type: 'info' }
    );
    for (let month = 1; month <= 12; month++) {
      await createAccountingPeriod({ year, period: month });
    }
    ElMessage.success(t('accountingPeriod.message.initSuccess'));
    fetchPeriods();
  } catch (e) {
    if (!isDialogDismissal(e)) {
      const err = e as Error;
      logger.error(t('accountingPeriod.message.initFailed'), err);
      ElMessage.error(err.message || t('accountingPeriod.message.initFailed'));
    }
  }
};

onMounted(() => {
  fetchPeriods();
});
</script>
