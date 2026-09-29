<!--
  BudgetPlanTab.vue - 预算方案头列表（一级）
  消费后端 /budgets/plans（budget_plans）：列表 + 新建 + 审批/驳回 + 进入明细行。
  方案与明细的关联通过 budget_items.plan_id（见 BudgetItemTab.vue）。
-->
<template>
  <div class="budget-plan-tab">
    <div class="page-header">
      <h2 class="page-title">{{ $t('budget.plan.title') }}</h2>
      <div>
        <el-button v-permission="PERMISSIONS.BUDGET_CREATE" type="primary" @click="openDialog()">
          <el-icon><Plus /></el-icon>{{ $t('budget.plan.create') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover">
      <el-table
        v-loading="loading"
        :data="plans"
        stripe
        row-key="id"
        :aria-label="$t('budget.plan.table.ariaLabel')"
      >
        <el-table-column prop="plan_no" :label="$t('budget.plan.table.planNo')" min-width="140" />
        <el-table-column
          prop="plan_name"
          :label="$t('budget.plan.table.planName')"
          min-width="180"
          show-overflow-tooltip
        />
        <el-table-column
          prop="budget_year"
          :label="$t('budget.plan.table.budgetYear')"
          width="110"
          align="center"
        />
        <el-table-column
          prop="budget_type"
          :label="$t('budget.plan.table.budgetType')"
          width="130"
        />
        <el-table-column :label="$t('budget.plan.table.departmentId')" width="110" align="center">
          <template #default="{ row }">{{ row.department_id ?? '-' }}</template>
        </el-table-column>
        <el-table-column :label="$t('budget.plan.table.totalAmount')" width="150" align="right">
          <template #default="{ row }">¥{{ amountOf(row.total_amount) }}</template>
        </el-table-column>
        <el-table-column :label="$t('budget.plan.table.status')" width="110" align="center">
          <template #default="{ row }">
            <el-tag :type="planTagType(row.status)">{{ planStatusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="$t('budget.plan.table.operation')" width="240" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="emit('select', row)">
              {{ $t('budget.plan.viewItems') }}
            </el-button>
            <el-button
              v-if="row.status === 'draft'"
              v-permission="PERMISSIONS.BUDGET_APPROVE"
              type="success"
              link
              size="small"
              @click="handleApprove(row)"
            >
              {{ $t('budget.plan.table.approve') }}
            </el-button>
            <el-button
              v-if="row.status === 'draft'"
              v-permission="PERMISSIONS.BUDGET_APPROVE"
              type="warning"
              link
              size="small"
              @click="handleReject(row)"
            >
              {{ $t('budget.plan.table.reject') }}
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog
      v-model="dialogVisible"
      :title="$t('budget.plan.dialog.createTitle')"
      width="520px"
      :aria-label="$t('budget.plan.dialog.formAriaLabel')"
    >
      <el-form
        ref="formRef"
        :model="form"
        :rules="rules"
        label-width="110px"
        :aria-label="$t('budget.plan.dialog.formAriaLabel')"
      >
        <el-form-item :label="$t('budget.plan.dialog.planNo')" prop="plan_no">
          <el-input
            v-model="form.plan_no"
            :placeholder="$t('budget.plan.dialog.planNoPlaceholder')"
          />
        </el-form-item>
        <el-form-item :label="$t('budget.plan.dialog.planName')" prop="plan_name">
          <el-input v-model="form.plan_name" />
        </el-form-item>
        <el-form-item :label="$t('budget.plan.dialog.budgetYear')" prop="budget_year">
          <el-input-number
            v-model="form.budget_year"
            :min="2000"
            :max="2100"
            :precision="0"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="$t('budget.plan.dialog.budgetType')" prop="budget_type">
          <el-input
            v-model="form.budget_type"
            :placeholder="$t('budget.plan.dialog.budgetTypePlaceholder')"
          />
        </el-form-item>
        <el-form-item :label="$t('budget.plan.dialog.departmentId')" prop="department_id">
          <el-input-number
            v-model="form.department_id"
            :min="1"
            :precision="0"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="$t('budget.plan.dialog.totalAmount')" prop="total_amount">
          <el-input-number
            v-model="form.total_amount"
            :min="0"
            :precision="2"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="$t('budget.plan.dialog.remark')" prop="remark">
          <el-input v-model="form.remark" type="textarea" :rows="3" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ $t('budget.plan.dialog.cancel') }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">
          {{ $t('budget.plan.dialog.confirm') }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import {
  getBudgetPlanList,
  createBudgetPlan,
  approveBudgetPlan,
  rejectBudgetPlan,
  type BudgetPlan,
} from '@/api/budget';
import { PERMISSIONS } from '@/constants/permissions';
import { isDialogDismissal } from '@/utils/monitor';

const { t } = useI18n({ useScope: 'global' });
const emit = defineEmits<{ (e: 'select', plan: BudgetPlan): void }>();

const loading = ref(false);
const plans = ref<BudgetPlan[]>([]);

const amountOf = (value: string | number | null | undefined): string =>
  Number(value ?? 0).toFixed(2);

const PLAN_STATUS_TYPE: Record<string, 'info' | 'warning' | 'success' | 'danger' | 'primary'> = {
  draft: 'info',
  approved: 'success',
  rejected: 'danger',
  active: 'primary',
  closed: 'info',
};

const planTagType = (status: string | null) => (status && PLAN_STATUS_TYPE[status]) || 'info';
const planStatusLabel = (status: string | null) =>
  status ? t(`budget.plan.status.${status}`) : '-';

const loadPlans = async () => {
  loading.value = true;
  try {
    const res = await getBudgetPlanList({ page: 1, page_size: 100 });
    plans.value = Array.isArray(res.data) ? res.data : [];
  } catch (error) {
    ElMessage.error((error as Error)?.message || t('budget.plan.message.loadFailed'));
  } finally {
    loading.value = false;
  }
};
loadPlans();

// ===== 新建方案 =====
const dialogVisible = ref(false);
const submitLoading = ref(false);
const formRef = ref<FormInstance>();

const emptyForm = () => ({
  plan_no: '',
  plan_name: '',
  budget_year: new Date().getFullYear() as number,
  budget_type: '',
  department_id: undefined as number | undefined,
  total_amount: 0,
  remark: '',
});

const form = reactive(emptyForm());

const rules: FormRules = {
  plan_name: [
    { required: true, message: t('budget.plan.validation.planNameRequired'), trigger: 'blur' },
  ],
  budget_year: [
    { required: true, message: t('budget.plan.validation.budgetYearRequired'), trigger: 'blur' },
  ],
  department_id: [
    { required: true, message: t('budget.plan.validation.departmentRequired'), trigger: 'blur' },
  ],
};

const openDialog = () => {
  formRef.value?.resetFields();
  Object.assign(form, emptyForm());
  dialogVisible.value = true;
};

const handleSubmit = async () => {
  if (!formRef.value) return;
  const valid = await formRef.value.validate().catch(() => false);
  if (!valid) return;
  submitLoading.value = true;
  try {
    await createBudgetPlan({
      plan_no: form.plan_no || undefined,
      plan_name: form.plan_name,
      budget_year: form.budget_year,
      budget_type: form.budget_type || undefined,
      department_id: form.department_id as number,
      total_amount: form.total_amount,
      remark: form.remark || undefined,
    });
    ElMessage.success(t('budget.plan.message.created'));
    dialogVisible.value = false;
    await loadPlans();
  } catch (error) {
    ElMessage.error((error as Error)?.message || t('budget.message.operationFailed'));
  } finally {
    submitLoading.value = false;
  }
};

// ===== 方案审批 / 驳回 =====
const handleApprove = async (row: BudgetPlan) => {
  try {
    const { value } = await ElMessageBox.prompt(
      t('budget.plan.message.approveConfirm', { name: row.plan_name }),
      t('budget.plan.message.comment'),
      {
        confirmButtonText: t('budget.plan.dialog.confirm'),
        cancelButtonText: t('budget.plan.dialog.cancel'),
        inputPlaceholder: t('budget.plan.message.commentPlaceholder'),
      }
    );
    await approveBudgetPlan(row.id, value || undefined);
    ElMessage.success(t('budget.auditSuccess'));
    await loadPlans();
  } catch (error) {
    if (!isDialogDismissal(error)) {
      ElMessage.error((error as Error)?.message || t('budget.message.auditFailed'));
    }
  }
};

const handleReject = async (row: BudgetPlan) => {
  try {
    const { value } = await ElMessageBox.prompt(
      t('budget.plan.message.rejectConfirm', { name: row.plan_name }),
      t('budget.plan.message.comment'),
      {
        confirmButtonText: t('budget.plan.dialog.confirm'),
        cancelButtonText: t('budget.plan.dialog.cancel'),
        inputPlaceholder: t('budget.plan.message.commentPlaceholder'),
      }
    );
    await rejectBudgetPlan(row.id, value || undefined);
    ElMessage.success(t('budget.auditSuccess'));
    await loadPlans();
  } catch (error) {
    if (!isDialogDismissal(error)) {
      ElMessage.error((error as Error)?.message || t('budget.message.auditFailed'));
    }
  }
};
</script>

<style scoped>
.budget-plan-tab {
  min-height: 100%;
}
</style>
