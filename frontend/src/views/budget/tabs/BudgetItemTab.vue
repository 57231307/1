<!--
  BudgetItemTab.vue - 预算明细行列表（二级，隶属某个预算方案）
  ----------------------------------------------------------------
  - 取数：GET /budgets?plan_id=<当前方案>（后端 budget_items 列表，支持 plan_id 过滤，不含 periods）
  - 新建/编辑：plan_id 由所属方案带入（表单只读、必填），随 POST/PUT 提交；
    POST /budgets 创建、PUT /budgets/{id} 更新（update 入参不含 item_code/plan_id）
  - 期间分解：编辑态随详情 GET /budgets/{id} 载入 periods，支持按 月/季/年度 多行金额录入；
    实时显示 Σ期间，与 planned_amount 不一致时前端提示（后端 400 为最终裁决，前端不做覆盖/兜底）
-->
<template>
  <div class="budget-item-tab">
    <div class="page-header">
      <div class="header-left">
        <el-button link @click="emit('back')">
          <el-icon><ArrowLeft /></el-icon>{{ $t('budget.plan.back') }}
        </el-button>
        <h2 class="page-title">{{ $t('budget.item.currentPlan', { name: plan.plan_name }) }}</h2>
      </div>
      <div>
        <el-button v-permission="PERMISSIONS.BUDGET_CREATE" type="primary" @click="openDialog()">
          <el-icon><Plus /></el-icon>{{ $t('budget.item.create') }}
        </el-button>
        <el-button v-permission="PERMISSIONS.BUDGET_READ" @click="handleExport">
          <el-icon><Download /></el-icon>{{ $t('budget.export') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :aria-label="$t('budget.filter.ariaLabel')">
        <el-form-item :label="$t('budget.filter.status')">
          <el-select
            v-model="queryStatus"
            clearable
            :placeholder="$t('budget.filter.statusPlaceholder')"
          >
            <el-option :label="$t('budget.item.status.draft')" value="draft" />
            <el-option :label="$t('budget.item.status.pending')" value="pending" />
            <el-option :label="$t('budget.item.status.approved')" value="approved" />
            <el-option :label="$t('budget.item.status.rejected')" value="rejected" />
            <el-option :label="$t('budget.item.status.active')" value="active" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleSearch">{{
            $t('budget.filter.query')
          }}</el-button>
          <el-button @click="handleReset">{{ $t('budget.filter.reset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover">
      <el-table
        v-loading="loading"
        :data="items"
        stripe
        :aria-label="$t('budget.item.table.ariaLabel')"
      >
        <el-table-column prop="item_code" :label="$t('budget.item.table.itemCode')" min-width="140">
          <template #default="{ row }">{{ row.item_code || '-' }}</template>
        </el-table-column>
        <el-table-column
          prop="item_name"
          :label="$t('budget.item.table.itemName')"
          min-width="160"
          show-overflow-tooltip
        />
        <el-table-column prop="item_type" :label="$t('budget.item.table.itemType')" width="120">
          <template #default="{ row }">{{ row.item_type || '-' }}</template>
        </el-table-column>
        <el-table-column :label="$t('budget.item.table.plannedAmount')" width="150" align="right">
          <template #default="{ row }">¥{{ amountOf(row.planned_amount) }}</template>
        </el-table-column>
        <el-table-column :label="$t('budget.item.table.status')" width="110" align="center">
          <template #default="{ row }">
            <el-tag :type="itemTagType(row.status)">{{ itemStatusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="remark"
          :label="$t('budget.item.table.remark')"
          min-width="140"
          show-overflow-tooltip
        />
        <el-table-column :label="$t('budget.item.table.operation')" width="240" fixed="right">
          <template #default="{ row }">
            <el-button
              v-permission="PERMISSIONS.BUDGET_UPDATE"
              type="primary"
              link
              size="small"
              @click="openDialog(row)"
            >
              {{ $t('budget.item.table.edit') }}
            </el-button>
            <el-button
              v-permission="PERMISSIONS.BUDGET_UPDATE"
              type="warning"
              link
              size="small"
              plain
              @click="openAdjust(row)"
            >
              {{ $t('budget.item.table.adjust') }}
            </el-button>
            <el-button
              v-permission="PERMISSIONS.BUDGET_DELETE"
              type="danger"
              link
              size="small"
              @click="handleDelete(row)"
            >
              {{ $t('budget.item.table.delete') }}
            </el-button>
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
          :aria-label="$t('budget.table.paginationAriaLabel')"
        />
      </div>
    </el-card>

    <!-- 明细编辑（含期间分解子表） -->
    <el-dialog
      v-model="dialogVisible"
      :title="form.id ? $t('budget.item.dialog.editTitle') : $t('budget.item.dialog.createTitle')"
      width="720px"
      :aria-label="$t('budget.item.dialog.ariaLabel')"
    >
      <el-form
        ref="formRef"
        :model="form"
        :rules="rules"
        label-width="110px"
        :aria-label="$t('budget.item.dialog.formAriaLabel')"
      >
        <el-form-item :label="$t('budget.item.dialog.planId')">
          <el-input :model-value="plan.plan_name" disabled />
        </el-form-item>
        <el-form-item :label="$t('budget.item.dialog.itemName')" prop="item_name">
          <el-input v-model="form.item_name" />
        </el-form-item>
        <el-form-item :label="$t('budget.item.dialog.itemCode')" prop="item_code">
          <el-input
            v-model="form.item_code"
            :disabled="!!form.id"
            :placeholder="$t('budget.item.dialog.itemCodePlaceholder')"
          />
        </el-form-item>
        <el-form-item :label="$t('budget.item.dialog.itemType')" prop="item_type">
          <el-input v-model="form.item_type" />
        </el-form-item>
        <el-form-item :label="$t('budget.item.dialog.plannedAmount')" prop="planned_amount">
          <el-input-number
            v-model="form.planned_amount"
            :min="0"
            :precision="2"
            style="width: 100%"
          />
        </el-form-item>

        <el-form-item :label="$t('budget.item.period.section')" label-width="110px">
          <div class="period-block">
            <el-table :data="form.periods" size="small" border>
              <el-table-column :label="$t('budget.item.period.type')" width="120">
                <template #default="{ row }">
                  <el-select v-model="row.type" style="width: 100%">
                    <el-option :label="$t('budget.item.period.month')" value="month" />
                    <el-option :label="$t('budget.item.period.quarter')" value="quarter" />
                    <el-option :label="$t('budget.item.period.yearly')" value="year" />
                  </el-select>
                </template>
              </el-table-column>
              <el-table-column :label="$t('budget.item.period.year')" width="120">
                <template #default="{ row }">
                  <el-select v-model="row.year" style="width: 100%">
                    <el-option v-for="y in yearOptions" :key="y" :label="y" :value="y" />
                  </el-select>
                </template>
              </el-table-column>
              <el-table-column :label="$t('budget.item.period.monthValue')" width="120">
                <template #default="{ row }">
                  <el-select v-if="row.type === 'month'" v-model="row.month" style="width: 100%">
                    <el-option v-for="m in 12" :key="m" :label="m" :value="m" />
                  </el-select>
                  <span v-else-if="row.type === 'quarter'">-</span>
                  <span v-else>-</span>
                </template>
              </el-table-column>
              <el-table-column :label="$t('budget.item.period.quarterValue')" width="120">
                <template #default="{ row }">
                  <el-select
                    v-if="row.type === 'quarter'"
                    v-model="row.quarter"
                    style="width: 100%"
                  >
                    <el-option v-for="q in 4" :key="q" :label="`Q${q}`" :value="q" />
                  </el-select>
                  <span v-else>-</span>
                </template>
              </el-table-column>
              <el-table-column :label="$t('budget.item.period.amount')" width="150">
                <template #default="{ row }">
                  <el-input-number
                    v-model="row.planned_amount"
                    :min="0"
                    :precision="2"
                    :controls="false"
                    style="width: 100%"
                  />
                </template>
              </el-table-column>
              <el-table-column :label="$t('budget.item.period.actual')" width="120" align="right">
                <template #default="{ row }">{{
                  row.actual_amount != null ? amountOf(row.actual_amount) : '-'
                }}</template>
              </el-table-column>
              <el-table-column :label="$t('budget.item.period.remove')" width="70" align="center">
                <template #default="{ $index }">
                  <el-button type="danger" link size="small" @click="removePeriod($index)">
                    {{ $t('budget.item.period.remove') }}
                  </el-button>
                </template>
              </el-table-column>
            </el-table>
            <div class="period-toolbar">
              <el-button type="primary" link @click="addPeriod">
                <el-icon><Plus /></el-icon>{{ $t('budget.item.period.add') }}
              </el-button>
              <span class="period-total">
                {{ $t('budget.item.period.total') }}: ¥{{ amountOf(periodTotal) }}
              </span>
            </div>
            <p v-if="!form.periods.length" class="period-empty">
              {{ $t('budget.item.period.empty') }}
            </p>
            <el-alert v-else-if="periodMismatch" type="warning" :closable="false" show-icon>
              {{
                $t('budget.item.period.mismatch', {
                  sum: amountOf(periodTotal),
                  planned: amountOf(form.planned_amount ?? 0),
                })
              }}
            </el-alert>
          </div>
        </el-form-item>

        <el-form-item :label="$t('budget.item.dialog.remark')" prop="remark">
          <el-input v-model="form.remark" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ $t('budget.item.dialog.cancel') }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">
          {{ $t('budget.item.dialog.confirm') }}
        </el-button>
      </template>
    </el-dialog>

    <!-- 预算调整 -->
    <el-dialog v-model="adjustVisible" :title="$t('budget.item.table.adjust')" width="460px">
      <el-form :model="adjustForm" label-width="100px">
        <el-form-item :label="$t('budget.item.table.plannedAmount')">
          <el-input-number v-model="adjustForm.adjust_amount" :precision="2" style="width: 100%" />
        </el-form-item>
        <el-form-item :label="$t('budget.item.dialog.remark')">
          <el-input v-model="adjustForm.reason" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="adjustVisible = false">{{ $t('budget.item.dialog.cancel') }}</el-button>
        <el-button type="primary" :loading="adjustSaving" @click="handleAdjust">
          {{ $t('budget.item.dialog.confirm') }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus';
import { Plus, Download, ArrowLeft } from '@element-plus/icons-vue';
import {
  getBudgetDetail,
  createBudgetItem,
  updateBudgetItem,
  adjustBudget,
  deleteBudget,
  type BudgetItem,
  type BudgetPlan,
  type BudgetItemPeriodInput,
} from '@/api/budget';
import { PERMISSIONS } from '@/constants/permissions';
import { isDialogDismissal } from '@/utils/monitor';
import { exportFromBackend } from '@/utils/export';
import { useTableApi } from '@/composables/useTableApi';

const props = defineProps<{ plan: BudgetPlan }>();
const emit = defineEmits<{ (e: 'back'): void }>();

const { t } = useI18n({ useScope: 'global' });

const amountOf = (value: string | number | null | undefined): string =>
  Number(value ?? 0).toFixed(2);

const ITEM_STATUS_TYPE: Record<string, 'info' | 'warning' | 'success' | 'danger' | 'primary'> = {
  draft: 'info',
  pending: 'warning',
  approved: 'success',
  rejected: 'danger',
  active: 'primary',
};
const itemTagType = (status: string) => ITEM_STATUS_TYPE[status] || 'info';
const itemStatusLabel = (status: string) => t(`budget.item.status.${status}`);

// ===== 列表（按 plan_id 过滤）=====
const queryStatus = ref('');

const {
  data: items,
  total,
  loading,
  page,
  pageSize,
  setQueryParam,
  refresh: refreshItems,
} = useTableApi<BudgetItem>({
  url: '/budgets',
  defaultParams: { plan_id: props.plan.id },
  defaultPageSize: 20,
  onError: (err: unknown) => {
    ElMessage.error((err as Error)?.message || t('budget.item.message.loadFailed'));
  },
});

const handleSearch = () => {
  setQueryParam('status', queryStatus.value);
  setQueryParam('plan_id', props.plan.id);
  page.value = 1;
  refreshItems();
};
const handleReset = () => {
  queryStatus.value = '';
  handleSearch();
};

// ===== 期间子表 =====
type PeriodRow = {
  type: 'month' | 'quarter' | 'year';
  year: number;
  month: number;
  quarter: number;
  planned_amount: number;
  actual_amount: string | number | null;
};

const anchorYear = computed(() => props.plan.budget_year || new Date().getFullYear());
const yearOptions = computed(() => {
  const a = anchorYear.value;
  return [a - 1, a, a + 1, a + 2];
});

/** 将期间行合约为后端 period token：月 2026-01 / 季 2026-Q1 / 年 2026-FY */
const composePeriod = (row: PeriodRow): string => {
  if (row.type === 'month') return `${row.year}-${String(row.month).padStart(2, '0')}`;
  if (row.type === 'quarter') return `${row.year}-Q${row.quarter}`;
  return `${row.year}-FY`;
};

/** 解析后端 period token 回期间行（用于编辑态回显） */
const parsePeriod = (
  token: string,
  planned: string | number,
  actual: string | number
): PeriodRow => {
  const base: PeriodRow = {
    type: 'year',
    year: anchorYear.value,
    month: 1,
    quarter: 1,
    planned_amount: Number(planned ?? 0),
    actual_amount: actual ?? null,
  };
  const month = /^(\d{4})-(\d{2})$/.exec(token);
  if (month) return { ...base, type: 'month', year: Number(month[1]), month: Number(month[2]) };
  const quarter = /^(\d{4})-Q([1-4])$/.exec(token);
  if (quarter)
    return { ...base, type: 'quarter', year: Number(quarter[1]), quarter: Number(quarter[2]) };
  const yearly = /^(\d{4})-FY$/.exec(token);
  if (yearly) return { ...base, type: 'year', year: Number(yearly[1]) };
  return { ...base, type: 'year', year: Number(token.slice(0, 4)) || anchorYear.value };
};

const newPeriodRow = (): PeriodRow => ({
  type: 'month',
  year: anchorYear.value,
  month: 1,
  quarter: 1,
  planned_amount: 0,
  actual_amount: null,
});

// ===== 明细编辑表单 =====
const dialogVisible = ref(false);
const submitLoading = ref(false);
const formRef = ref<FormInstance>();

const form = reactive<{
  id?: number;
  plan_id: number;
  item_name: string;
  item_code: string;
  item_type: string;
  planned_amount: number | undefined;
  remark: string;
  periods: PeriodRow[];
}>({
  id: undefined,
  plan_id: props.plan.id,
  item_name: '',
  item_code: '',
  item_type: '',
  planned_amount: 0,
  remark: '',
  periods: [],
});

const rules: FormRules = {
  item_name: [
    { required: true, message: t('budget.item.validation.itemNameRequired'), trigger: 'blur' },
  ],
  planned_amount: [
    {
      required: true,
      message: t('budget.item.validation.plannedAmountRequired'),
      trigger: 'blur',
    },
  ],
};

const periodTotal = computed(() =>
  form.periods.reduce((sum, row) => sum + Number(row.planned_amount || 0), 0)
);
const periodMismatch = computed(
  () =>
    form.periods.length > 0 &&
    Math.abs(periodTotal.value - Number(form.planned_amount || 0)) > 0.005
);

const addPeriod = () => {
  form.periods.push(newPeriodRow());
};
const removePeriod = (index: number) => {
  form.periods.splice(index, 1);
};

const openDialog = async (row?: BudgetItem) => {
  formRef.value?.resetFields();
  form.periods = [];
  if (row) {
    form.id = row.id;
    form.plan_id = row.plan_id;
    form.item_name = row.item_name;
    form.item_code = row.item_code || '';
    form.item_type = row.item_type || '';
    form.planned_amount = Number(row.planned_amount);
    form.remark = row.remark || '';
    try {
      const res = await getBudgetDetail(row.id);
      const periods = res.data?.periods ?? [];
      form.periods = periods.map(p => parsePeriod(p.period, p.planned_amount, p.actual_amount));
    } catch (error) {
      ElMessage.error((error as Error)?.message || t('budget.message.detailFailed'));
    }
  } else {
    form.id = undefined;
    form.plan_id = props.plan.id;
    form.item_name = '';
    form.item_code = '';
    form.item_type = '';
    form.planned_amount = 0;
    form.remark = '';
  }
  dialogVisible.value = true;
};

const buildPeriodInputs = (): BudgetItemPeriodInput[] | undefined => {
  if (!form.periods.length) return undefined;
  return form.periods.map(row => ({
    period: composePeriod(row),
    planned_amount: Number(row.planned_amount || 0),
  }));
};

const handleSubmit = async () => {
  if (!formRef.value) return;
  const valid = await formRef.value.validate().catch(() => false);
  if (!valid) return;
  submitLoading.value = true;
  const periods = buildPeriodInputs();
  try {
    if (form.id) {
      await updateBudgetItem(form.id, {
        item_name: form.item_name,
        item_type: form.item_type || undefined,
        planned_amount: Number(form.planned_amount),
        periods,
        remark: form.remark || undefined,
      });
      ElMessage.success(t('budget.item.message.updated'));
    } else {
      await createBudgetItem({
        item_name: form.item_name,
        item_code: form.item_code || undefined,
        item_type: form.item_type || undefined,
        plan_id: form.plan_id,
        budget_year: anchorYear.value,
        planned_amount: Number(form.planned_amount),
        periods,
        remark: form.remark || undefined,
      });
      ElMessage.success(t('budget.item.message.created'));
    }
    dialogVisible.value = false;
    await refreshItems();
  } catch (error) {
    ElMessage.error((error as Error)?.message || t('budget.message.operationFailed'));
  } finally {
    submitLoading.value = false;
  }
};

const handleDelete = async (row: BudgetItem) => {
  try {
    await ElMessageBox.confirm(
      t('budget.item.message.deleteConfirm', { name: row.item_name }),
      t('message.deleteConfirmTitle'),
      { type: 'warning' }
    );
    await deleteBudget(row.id);
    ElMessage.success(t('message.deleteSuccess'));
    await refreshItems();
  } catch (error) {
    if (!isDialogDismissal(error)) {
      ElMessage.error((error as Error)?.message || t('budget.message.deleteFailed'));
    }
  }
};

// ===== 预算调整 =====
const adjustVisible = ref(false);
const adjustSaving = ref(false);
const adjustForm = reactive({ item_id: 0, adjust_amount: 0, reason: '' });

const openAdjust = (row: BudgetItem) => {
  adjustForm.item_id = row.id;
  adjustForm.adjust_amount = 0;
  adjustForm.reason = '';
  adjustVisible.value = true;
};

const handleAdjust = async () => {
  if (!adjustForm.item_id || adjustForm.adjust_amount === 0) return;
  adjustSaving.value = true;
  try {
    await adjustBudget({
      item_id: adjustForm.item_id,
      adjust_amount: adjustForm.adjust_amount,
      reason: adjustForm.reason || undefined,
    });
    ElMessage.success(t('budget.item.message.updated'));
    adjustVisible.value = false;
    await refreshItems();
  } catch (error) {
    ElMessage.error((error as Error)?.message || t('budget.message.operationFailed'));
  } finally {
    adjustSaving.value = false;
  }
};

// ===== 导出（按当前方案过滤）=====
const handleExport = async () => {
  await exportFromBackend('/budgets/export', { plan_id: props.plan.id }, 'budget_items_export');
  ElMessage.success(t('budget.message.listExported'));
};
</script>

<style scoped>
.budget-item-tab {
  min-height: 100%;
}
.header-left {
  display: flex;
  align-items: center;
  gap: 8px;
}
.period-block {
  width: 100%;
}
.period-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 8px;
}
.period-total {
  font-weight: 600;
}
.period-empty {
  margin: 8px 0 0;
  color: #909399;
  font-size: 12px;
}
</style>
