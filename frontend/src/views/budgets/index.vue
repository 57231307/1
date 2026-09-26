<template>
  <div class="budgets-page">
    <el-card shadow="never">
      <div class="page-header">
        <h2>预算审批</h2>
        <div class="header-actions">
          <el-select
            v-model="queryStatus"
            placeholder="全部状态"
            clearable
            style="width: 140px"
            @change="handleFilter"
          >
            <el-option
              v-for="(label, key) in statusTextMap"
              :key="key"
              :label="label"
              :value="key"
            />
          </el-select>
          <el-input
            v-model="queryItemType"
            placeholder="预算类型"
            clearable
            style="width: 140px"
            @keyup.enter="handleFilter"
            @clear="handleFilter"
          />
          <el-button type="primary" @click="handleCreate">新建预算</el-button>
        </div>
      </div>

      <el-table v-loading="loading" :data="budgetList" border row-key="id">
        <el-table-column type="expand">
          <template #default="{ row }">
            <div class="approval-records">
              <div class="approval-records-header">
                <span>审批记录（版本历史）</span>
                <el-button size="small" link type="primary" @click="loadVersions(row)">
                  刷新
                </el-button>
              </div>
              <el-table
                v-loading="versionLoadingMap[row.id]"
                :data="versionMap[row.id] ?? []"
                size="small"
                border
              >
                <el-table-column prop="version_no" label="版本号" min-width="120" />
                <el-table-column prop="version_name" label="版本名称" min-width="140" />
                <el-table-column prop="total_amount" label="总金额" width="130" align="right" />
                <el-table-column prop="status" label="状态" width="100" align="center">
                  <template #default="{ row: version }">
                    <el-tag :type="versionStatusTagMap[version.status] ?? 'info'">
                      {{ versionStatusTextMap[version.status] ?? version.status }}
                    </el-tag>
                  </template>
                </el-table-column>
                <el-table-column
                  prop="change_reason"
                  label="变更原因"
                  min-width="140"
                  show-overflow-tooltip
                >
                  <template #default="{ row: version }">{{
                    version.change_reason || '-'
                  }}</template>
                </el-table-column>
                <el-table-column prop="approved_by" label="审批人" width="90" align="center">
                  <template #default="{ row: version }">{{ version.approved_by ?? '-' }}</template>
                </el-table-column>
                <el-table-column prop="approved_at" label="审批时间" min-width="160">
                  <template #default="{ row: version }">{{ version.approved_at || '-' }}</template>
                </el-table-column>
              </el-table>
            </div>
          </template>
        </el-table-column>
        <el-table-column prop="item_code" label="预算编码" min-width="140">
          <template #default="{ row }">{{ row.item_code || '-' }}</template>
        </el-table-column>
        <el-table-column prop="item_name" label="预算名称" min-width="150" show-overflow-tooltip />
        <el-table-column prop="item_type" label="类型" width="110" align="center">
          <template #default="{ row }">{{ row.item_type || '-' }}</template>
        </el-table-column>
        <el-table-column prop="budget_year" label="年度" width="90" align="center">
          <template #default="{ row }">{{ row.budget_year ?? '-' }}</template>
        </el-table-column>
        <el-table-column prop="planned_amount" label="计划金额" width="130" align="right">
          <template #default="{ row }">{{ row.planned_amount ?? '-' }}</template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="statusTagMap[row.status as BudgetItemStatus] ?? 'info'">
              {{ statusTextMap[row.status as BudgetItemStatus] ?? row.status }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="remark" label="备注" min-width="130" show-overflow-tooltip>
          <template #default="{ row }">{{ row.remark || '-' }}</template>
        </el-table-column>
        <el-table-column label="操作" width="240" fixed="right">
          <template #default="{ row }">
            <el-button
              v-for="action in getNextActions(row)"
              :key="action.key"
              size="small"
              link
              :type="action.danger ? 'danger' : 'primary'"
              @click="runAction(action, row)"
            >
              {{ action.label }}
            </el-button>
            <el-button
              v-if="canDelete(row)"
              size="small"
              link
              type="danger"
              @click="handleDelete(row)"
            >
              删除
            </el-button>
          </template>
        </el-table-column>
      </el-table>

      <el-pagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :total="total"
        :page-sizes="[10, 20, 50]"
        layout="total, sizes, prev, pager, next"
        style="margin-top: 16px; justify-content: flex-end"
        @current-change="loadList"
        @size-change="handleFilter"
      />
    </el-card>

    <el-dialog v-model="createVisible" title="新建预算" width="560px" @close="resetForm">
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="100px">
        <el-form-item :label="t('budgets.form.planId')" prop="plan_id">
          <el-select
            v-model="formData.plan_id"
            :placeholder="t('budgets.form.planIdPlaceholder')"
            filterable
            style="width: 100%"
          >
            <el-option
              v-for="plan in planList"
              :key="plan.id"
              :label="`${plan.plan_no} - ${plan.plan_name}`"
              :value="plan.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="预算名称" prop="item_name">
          <el-input v-model="formData.item_name" placeholder="必填" />
        </el-form-item>
        <el-form-item label="预算编码" prop="item_code">
          <el-input v-model="formData.item_code" placeholder="选填，留空自动生成" />
        </el-form-item>
        <el-form-item label="预算类型" prop="item_type">
          <el-input v-model="formData.item_type" placeholder="选填" />
        </el-form-item>
        <el-form-item label="预算年度" prop="budget_year">
          <el-input-number
            v-model="formData.budget_year"
            :min="2000"
            :max="2100"
            :precision="0"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="计划金额" prop="planned_amount">
          <el-input-number
            v-model="formData.planned_amount"
            :min="0"
            :precision="2"
            style="width: 100%"
            placeholder="必填"
          />
        </el-form-item>
        <el-form-item label="备注" prop="remark">
          <el-input v-model="formData.remark" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="createVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="submitCreate">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="detailVisible" title="预算详情" width="640px">
      <el-descriptions v-if="detailRow" :column="2" border>
        <el-descriptions-item label="预算编码">{{
          detailRow.item_code || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="预算名称">{{ detailRow.item_name }}</el-descriptions-item>
        <el-descriptions-item label="类型">{{ detailRow.item_type || '-' }}</el-descriptions-item>
        <el-descriptions-item label="年度">{{ detailRow.budget_year ?? '-' }}</el-descriptions-item>
        <el-descriptions-item label="计划金额">{{ detailRow.planned_amount }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag :type="statusTagMap[detailRow.status] ?? 'info'">
            {{ statusTextMap[detailRow.status] ?? detailRow.status }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="层级">{{ detailRow.level }}</el-descriptions-item>
        <el-descriptions-item label="关联会计科目 ID">
          {{ detailRow.account_subject_id ?? '-' }}
        </el-descriptions-item>
        <el-descriptions-item label="创建时间">{{ detailRow.created_at }}</el-descriptions-item>
        <el-descriptions-item label="更新时间">{{ detailRow.updated_at }}</el-descriptions-item>
        <el-descriptions-item label="备注" :span="2">{{
          detailRow.remark || '-'
        }}</el-descriptions-item>
      </el-descriptions>
      <template #footer>
        <el-button @click="detailVisible = false">关闭</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import { useI18n } from 'vue-i18n';
import {
  getBudgetItemList,
  getBudgetDetail,
  createBudgetItem,
  submitBudget,
  approveBudgetRecord,
  rejectBudget,
  deleteBudget,
  getBudgetVersions,
  getBudgetPlanList,
  type BudgetItem,
  type BudgetItemStatus,
  type BudgetVersion,
  type BudgetPlan,
} from '@/api/budget';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

const statusTextMap: Record<BudgetItemStatus, string> = {
  draft: '草稿',
  pending: '待审批',
  approved: '已批准',
  rejected: '已驳回',
  active: '执行中',
};

const statusTagMap: Record<
  BudgetItemStatus,
  'info' | 'warning' | 'primary' | 'success' | 'danger'
> = {
  draft: 'info',
  pending: 'warning',
  approved: 'success',
  rejected: 'danger',
  active: 'primary',
};

const versionStatusTextMap: Record<string, string> = {
  draft: '草稿',
  approved: '已批准',
};

const versionStatusTagMap: Record<string, 'info' | 'success'> = {
  draft: 'info',
  approved: 'success',
};

interface StatusAction {
  key: 'submit' | 'approve' | 'reject' | 'detail';
  label: string;
  danger?: boolean;
}

/** 审批链：draft 提交审批 → pending 审批/驳回 → approved / rejected */
const nextActionMap: Record<BudgetItemStatus, StatusAction[]> = {
  draft: [
    { key: 'detail', label: '详情' },
    { key: 'submit', label: '提交审批' },
  ],
  pending: [
    { key: 'detail', label: '详情' },
    { key: 'approve', label: '审批' },
    { key: 'reject', label: '驳回', danger: true },
  ],
  approved: [{ key: 'detail', label: '详情' }],
  rejected: [
    { key: 'detail', label: '详情' },
    { key: 'approve', label: '审批' },
  ],
  active: [{ key: 'detail', label: '详情' }],
};

const loading = ref(false);
const submitLoading = ref(false);
const budgetList = ref<BudgetItem[]>([]);
const total = ref(0);
const page = ref(1);
const pageSize = ref(20);
const queryStatus = ref('');
const queryItemType = ref('');
const createVisible = ref(false);
const detailVisible = ref(false);
const detailRow = ref<BudgetItem | null>(null);
const formRef = ref<FormInstance>();

/** 预算方案列表（创建预算明细时必选所属方案） */
const planList = ref<BudgetPlan[]>([]);

/** 审批记录子表数据：按预算 id 缓存版本审批历史 */
const versionMap = ref<Record<number, BudgetVersion[]>>({});
const versionLoadingMap = ref<Record<number, boolean>>({});

const formData = reactive<{
  item_name: string;
  item_code: string;
  item_type: string;
  plan_id: number | undefined;
  budget_year: number | undefined;
  planned_amount: number | undefined;
  remark: string;
}>({
  item_name: '',
  item_code: '',
  item_type: '',
  plan_id: undefined,
  budget_year: undefined,
  planned_amount: undefined,
  remark: '',
});

const formRules: FormRules = {
  item_name: [{ required: true, message: '请输入预算名称', trigger: 'blur' }],
  planned_amount: [{ required: true, message: '请输入计划金额', trigger: 'blur' }],
  plan_id: [{ required: true, message: '请选择所属预算方案', trigger: 'change' }],
};

const loadVersions = async (row: BudgetItem) => {
  versionLoadingMap.value = { ...versionLoadingMap.value, [row.id]: true };
  try {
    const res = await getBudgetVersions(row.id);
    const versions = Array.isArray(res.data)
      ? res.data
      : ((res.data as { items?: BudgetVersion[] } | null)?.items ?? []);
    versionMap.value = { ...versionMap.value, [row.id]: versions };
  } catch {
    versionMap.value = { ...versionMap.value, [row.id]: [] };
    ElMessage.error('加载审批记录失败');
  } finally {
    versionLoadingMap.value = { ...versionLoadingMap.value, [row.id]: false };
  }
};

/** 响应解包防御：兼容数组 / { items } 分页包装，避免 el-table "r is not iterable" */
const unwrapList = (payload: unknown): BudgetItem[] => {
  if (Array.isArray(payload)) return payload;
  const paged = payload as { items?: BudgetItem[] } | null;
  return paged?.items ?? [];
};

const loadList = async () => {
  loading.value = true;
  try {
    const res = await getBudgetItemList({
      page: page.value,
      page_size: pageSize.value,
      status: queryStatus.value || undefined,
      item_type: queryItemType.value || undefined,
    });
    budgetList.value = unwrapList(res.data);
    total.value =
      res.data && !Array.isArray(res.data) ? (res.data.total ?? 0) : budgetList.value.length;
  } catch {
    ElMessage.error('加载预算列表失败');
  } finally {
    loading.value = false;
  }
};

const handleFilter = () => {
  page.value = 1;
  loadList();
};

const getNextActions = (row: BudgetItem): StatusAction[] =>
  nextActionMap[row.status as BudgetItemStatus] ?? [];

const runAction = async (action: StatusAction, row: BudgetItem) => {
  try {
    if (action.key === 'detail') {
      const res = await getBudgetDetail(row.id);
      detailRow.value = res.data ?? row;
      detailVisible.value = true;
      return;
    }
    if (action.key === 'approve') {
      const { value } = await ElMessageBox.prompt(
        `确认审批通过预算「${row.item_name}」吗？可填写审批意见。`,
        '审批确认',
        {
          confirmButtonText: '通过',
          cancelButtonText: '取消',
          inputPlaceholder: '审批意见（选填）',
        }
      );
      await approveBudgetRecord(row.id, value || undefined);
    } else if (action.key === 'reject') {
      const { value } = await ElMessageBox.prompt(
        `确认驳回预算「${row.item_name}」吗？请填写驳回原因。`,
        '驳回确认',
        {
          confirmButtonText: '确认驳回',
          cancelButtonText: '取消',
          inputPlaceholder: '驳回原因（选填）',
        }
      );
      await rejectBudget(row.id, value || undefined);
    } else {
      await ElMessageBox.confirm(`确认将预算「${row.item_name}」提交审批吗？`, '提交确认', {
        confirmButtonText: '确认',
        cancelButtonText: '取消',
        type: 'warning',
      });
      await submitBudget(row.id);
    }
    ElMessage.success(`操作成功：${action.label}`);
    await loadList();
  } catch (error) {
    if (error === 'cancel' || (error as { message?: string })?.message === 'cancel') return;
    ElMessage.error(`操作失败：${action.label}`);
  }
};

const handleCreate = () => {
  createVisible.value = true;
};

const resetForm = () => {
  formData.item_name = '';
  formData.item_code = '';
  formData.item_type = '';
  formData.plan_id = undefined;
  formData.budget_year = undefined;
  formData.planned_amount = undefined;
  formData.remark = '';
  formRef.value?.resetFields();
};

const submitCreate = async () => {
  if (!formRef.value) return;
  await formRef.value.validate(async valid => {
    if (!valid) return;
    if (!formData.planned_amount) return;
    if (!formData.plan_id) return;
    submitLoading.value = true;
    try {
      await createBudgetItem({
        item_name: formData.item_name,
        item_code: formData.item_code || undefined,
        item_type: formData.item_type || undefined,
        plan_id: formData.plan_id,
        budget_year: formData.budget_year,
        planned_amount: formData.planned_amount,
        remark: formData.remark || undefined,
      });
      ElMessage.success('预算创建成功');
      createVisible.value = false;
      await loadList();
    } catch {
      ElMessage.error('预算创建失败');
    } finally {
      submitLoading.value = false;
    }
  });
};

const handleDelete = async (row: BudgetItem) => {
  try {
    await ElMessageBox.confirm(`确认删除预算「${row.item_name}」吗？`, '删除确认', {
      confirmButtonText: '确认删除',
      cancelButtonText: '取消',
      type: 'warning',
    });
    await deleteBudget(row.id);
    ElMessage.success('删除成功');
    await loadList();
  } catch (error) {
    if (error === 'cancel' || (error as { message?: string })?.message === 'cancel') return;
    ElMessage.error('删除失败');
  }
};

const canDelete = (row: BudgetItem): boolean => row.status === 'draft';

/** 加载预算方案列表（创建明细时必选所属方案） */
const fetchPlanList = async () => {
  try {
    const res = await getBudgetPlanList({ page: 1, page_size: 200 });
    planList.value = Array.isArray(res.data) ? res.data : [];
  } catch (error) {
    logger.error('加载预算方案列表失败', error);
  }
};

onMounted(() => {
  loadList();
  fetchPlanList();
});
</script>

<style scoped>
.budgets-page {
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

.approval-records {
  padding: 8px 24px 8px 48px;
}

.approval-records-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
  font-weight: 600;
}
</style>
