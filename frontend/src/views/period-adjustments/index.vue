<template>
  <div class="period-adjustments-page">
    <el-card shadow="never">
      <div class="page-header">
        <h2>期末调整</h2>
        <div class="header-actions">
          <el-select
            v-model="queryType"
            placeholder="全部类型"
            clearable
            style="width: 140px"
            @change="handleFilter"
          >
            <el-option v-for="(label, key) in typeTextMap" :key="key" :label="label" :value="key" />
          </el-select>
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
            v-model="queryPeriod"
            placeholder="期间，如 2026-01"
            clearable
            style="width: 180px"
            @keyup.enter="handleFilter"
            @clear="handleFilter"
          />
          <el-button type="primary" @click="handleCreate">新建调整</el-button>
        </div>
      </div>

      <el-table v-loading="loading" :data="adjustmentList" border>
        <el-table-column prop="adjustment_no" label="调整单号" min-width="170" />
        <el-table-column prop="adjustment_type" label="类型" width="90" align="center">
          <template #default="{ row }">
            {{ typeTextMap[row.adjustment_type as PeriodAdjustmentType] ?? row.adjustment_type }}
          </template>
        </el-table-column>
        <el-table-column prop="period" label="期间" width="100" align="center" />
        <el-table-column prop="description" label="调整说明" min-width="160" show-overflow-tooltip>
          <template #default="{ row }">{{ row.description || '-' }}</template>
        </el-table-column>
        <el-table-column label="借方科目" min-width="150" show-overflow-tooltip>
          <template #default="{ row }">
            {{ row.debit_subject_code }} {{ row.debit_subject_name }}
          </template>
        </el-table-column>
        <el-table-column label="贷方科目" min-width="150" show-overflow-tooltip>
          <template #default="{ row }">
            {{ row.credit_subject_code }} {{ row.credit_subject_name }}
          </template>
        </el-table-column>
        <el-table-column prop="amount" label="金额" width="120" align="right">
          <template #default="{ row }">{{ row.amount ?? '-' }}</template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="statusTagMap[row.status as PeriodAdjustmentStatus] ?? 'info'">
              {{ statusTextMap[row.status as PeriodAdjustmentStatus] ?? row.status }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <el-button size="small" link type="primary" @click="handleDetail(row)">详情</el-button>
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

    <el-dialog v-model="createVisible" title="新建期末调整" width="600px" @close="resetForm">
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="100px">
        <el-form-item label="调整类型" prop="adjustment_type">
          <el-select
            v-model="formData.adjustment_type"
            placeholder="请选择调整类型"
            style="width: 100%"
          >
            <el-option v-for="(label, key) in typeTextMap" :key="key" :label="label" :value="key" />
          </el-select>
        </el-form-item>
        <el-form-item label="调整期间" prop="period">
          <el-input v-model="formData.period" placeholder="如 2026-01" />
        </el-form-item>
        <el-form-item label="调整说明" prop="description">
          <el-input v-model="formData.description" type="textarea" :rows="2" placeholder="必填" />
        </el-form-item>
        <el-form-item label="借方科目" prop="debit_subject_code">
          <div class="subject-row">
            <el-input v-model="formData.debit_subject_code" placeholder="科目编码（必填）" />
            <el-input v-model="formData.debit_subject_name" placeholder="科目名称（必填）" />
          </div>
        </el-form-item>
        <el-form-item label="贷方科目" prop="credit_subject_code">
          <div class="subject-row">
            <el-input v-model="formData.credit_subject_code" placeholder="科目编码（必填）" />
            <el-input v-model="formData.credit_subject_name" placeholder="科目名称（必填）" />
          </div>
        </el-form-item>
        <el-form-item label="调整金额" prop="amount">
          <el-input-number
            v-model="formData.amount"
            :min="0"
            :precision="2"
            style="width: 100%"
            placeholder="必填"
          />
        </el-form-item>
        <el-form-item label="备注" prop="remarks">
          <el-input v-model="formData.remarks" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="createVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="submitCreate">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="detailVisible" title="期末调整详情" width="680px">
      <el-descriptions v-if="detailRow" :column="2" border>
        <el-descriptions-item label="调整单号">{{ detailRow.adjustment_no }}</el-descriptions-item>
        <el-descriptions-item label="类型">
          {{ typeTextMap[detailRow.adjustment_type] ?? detailRow.adjustment_type }}
        </el-descriptions-item>
        <el-descriptions-item label="期间">{{ detailRow.period }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag :type="statusTagMap[detailRow.status] ?? 'info'">
            {{ statusTextMap[detailRow.status] ?? detailRow.status }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="借方科目" :span="2">
          {{ detailRow.debit_subject_code }} {{ detailRow.debit_subject_name }}
        </el-descriptions-item>
        <el-descriptions-item label="贷方科目" :span="2">
          {{ detailRow.credit_subject_code }} {{ detailRow.credit_subject_name }}
        </el-descriptions-item>
        <el-descriptions-item label="金额">{{ detailRow.amount }}</el-descriptions-item>
        <el-descriptions-item label="调整凭证 ID">{{
          detailRow.voucher_id ?? '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="红字凭证 ID">
          {{ detailRow.reverse_voucher_id ?? '-' }}
        </el-descriptions-item>
        <el-descriptions-item label="来源单据">
          {{ detailRow.source_bill_no || detailRow.source_type || '-' }}
        </el-descriptions-item>
        <el-descriptions-item label="确认时间">{{
          detailRow.confirmed_at || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="冲销时间">{{
          detailRow.reversed_at || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="创建时间">{{ detailRow.created_at }}</el-descriptions-item>
        <el-descriptions-item label="调整说明" :span="2">{{
          detailRow.description
        }}</el-descriptions-item>
        <el-descriptions-item label="备注" :span="2">{{
          detailRow.remarks || '-'
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
import {
  getPeriodAdjustmentList,
  createPeriodAdjustment,
  confirmPeriodAdjustment,
  reversePeriodAdjustment,
  cancelPeriodAdjustment,
  type PeriodAdjustment,
  type PeriodAdjustmentStatus,
  type PeriodAdjustmentType,
} from '@/api/period-adjustment';

const typeTextMap: Record<PeriodAdjustmentType, string> = {
  estimate: '暂估',
  amortization: '摊销',
  provision: '预提',
  transfer: '转账',
};

const statusTextMap: Record<PeriodAdjustmentStatus, string> = {
  draft: '草稿',
  confirmed: '已确认',
  reversed: '已冲销',
  cancelled: '已取消',
};

const statusTagMap: Record<
  PeriodAdjustmentStatus,
  'info' | 'warning' | 'primary' | 'success' | 'danger'
> = {
  draft: 'info',
  confirmed: 'primary',
  reversed: 'warning',
  cancelled: 'danger',
};

interface StatusAction {
  key: 'confirm' | 'reverse' | 'cancel';
  label: string;
  danger?: boolean;
}

/** 按后端状态机生成下一步操作：draft → confirmed → reversed；draft → cancelled */
const nextActionMap: Record<PeriodAdjustmentStatus, StatusAction[]> = {
  draft: [
    { key: 'confirm', label: '确认' },
    { key: 'cancel', label: '取消', danger: true },
  ],
  confirmed: [{ key: 'reverse', label: '红字冲销' }],
  reversed: [],
  cancelled: [],
};

const actionRunners: Record<string, (id: number) => Promise<unknown>> = {
  confirm: (id: number) => confirmPeriodAdjustment(id),
  reverse: (id: number) => reversePeriodAdjustment(id),
  cancel: (id: number) => cancelPeriodAdjustment(id),
};

const loading = ref(false);
const submitLoading = ref(false);
const adjustmentList = ref<PeriodAdjustment[]>([]);
const total = ref(0);
const page = ref(1);
const pageSize = ref(20);
const queryType = ref('');
const queryStatus = ref('');
const queryPeriod = ref('');
const createVisible = ref(false);
const detailVisible = ref(false);
const detailRow = ref<PeriodAdjustment | null>(null);
const formRef = ref<FormInstance>();

const formData = reactive<{
  adjustment_type: PeriodAdjustmentType | undefined;
  period: string;
  description: string;
  debit_subject_code: string;
  debit_subject_name: string;
  credit_subject_code: string;
  credit_subject_name: string;
  amount: number | undefined;
  remarks: string;
}>({
  adjustment_type: undefined,
  period: '',
  description: '',
  debit_subject_code: '',
  debit_subject_name: '',
  credit_subject_code: '',
  credit_subject_name: '',
  amount: undefined,
  remarks: '',
});

const formRules: FormRules = {
  adjustment_type: [{ required: true, message: '请选择调整类型', trigger: 'change' }],
  period: [{ required: true, message: '请输入调整期间', trigger: 'blur' }],
  description: [{ required: true, message: '请输入调整说明', trigger: 'blur' }],
  debit_subject_code: [{ required: true, message: '请输入借方科目编码', trigger: 'blur' }],
  debit_subject_name: [{ required: true, message: '请输入借方科目名称', trigger: 'blur' }],
  credit_subject_code: [{ required: true, message: '请输入贷方科目编码', trigger: 'blur' }],
  credit_subject_name: [{ required: true, message: '请输入贷方科目名称', trigger: 'blur' }],
  amount: [{ required: true, message: '请输入调整金额', trigger: 'blur' }],
};

/** 响应解包防御：兼容数组 / { items } 分页包装，避免 el-table "r is not iterable" */
const unwrapList = (payload: unknown): PeriodAdjustment[] => {
  if (Array.isArray(payload)) return payload;
  const paged = payload as { items?: PeriodAdjustment[] } | null;
  return paged?.items ?? [];
};

const loadList = async () => {
  loading.value = true;
  try {
    const res = await getPeriodAdjustmentList({
      page: page.value,
      page_size: pageSize.value,
      adjustment_type: queryType.value || undefined,
      status: queryStatus.value || undefined,
      period: queryPeriod.value || undefined,
    });
    adjustmentList.value = unwrapList(res.data);
    total.value =
      res.data && !Array.isArray(res.data) ? (res.data.total ?? 0) : adjustmentList.value.length;
  } catch {
    ElMessage.error('加载期末调整列表失败');
  } finally {
    loading.value = false;
  }
};

const handleFilter = () => {
  page.value = 1;
  loadList();
};

const getNextActions = (row: PeriodAdjustment): StatusAction[] =>
  nextActionMap[row.status as PeriodAdjustmentStatus] ?? [];

const runAction = async (action: StatusAction, row: PeriodAdjustment) => {
  try {
    await ElMessageBox.confirm(
      `确认对调整单 ${row.adjustment_no} 执行「${action.label}」吗？`,
      '操作确认',
      {
        confirmButtonText: '确认',
        cancelButtonText: '取消',
        type: 'warning',
      }
    );
    await actionRunners[action.key](row.id);
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
  formData.adjustment_type = undefined;
  formData.period = '';
  formData.description = '';
  formData.debit_subject_code = '';
  formData.debit_subject_name = '';
  formData.credit_subject_code = '';
  formData.credit_subject_name = '';
  formData.amount = undefined;
  formData.remarks = '';
  formRef.value?.resetFields();
};

const handleDetail = (row: PeriodAdjustment) => {
  detailRow.value = row;
  detailVisible.value = true;
};

const submitCreate = async () => {
  if (!formRef.value) return;
  await formRef.value.validate(async valid => {
    if (!valid) return;
    if (!formData.adjustment_type || formData.amount === undefined) return;
    submitLoading.value = true;
    try {
      await createPeriodAdjustment({
        adjustment_type: formData.adjustment_type,
        period: formData.period,
        description: formData.description,
        debit_subject_code: formData.debit_subject_code,
        debit_subject_name: formData.debit_subject_name,
        credit_subject_code: formData.credit_subject_code,
        credit_subject_name: formData.credit_subject_name,
        amount: formData.amount,
        remarks: formData.remarks || undefined,
      });
      ElMessage.success('期末调整创建成功');
      createVisible.value = false;
      await loadList();
    } catch {
      ElMessage.error('期末调整创建失败');
    } finally {
      submitLoading.value = false;
    }
  });
};

onMounted(() => {
  loadList();
});
</script>

<style scoped>
.period-adjustments-page {
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

.subject-row {
  display: flex;
  gap: 8px;
  width: 100%;
}
</style>
