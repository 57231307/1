<template>
  <div class="periods-page">
    <el-card shadow="never">
      <div class="page-header">
        <h2>会计期间</h2>
        <div class="header-actions">
          <el-button @click="handleInit">初始化当前期间</el-button>
          <el-button @click="handleYearEnd">年度结账</el-button>
          <el-button type="primary" @click="handleCreate">新建期间</el-button>
        </div>
      </div>

      <el-table v-loading="loading" :data="periodList" border>
        <el-table-column prop="period_name" label="期间名称" min-width="140">
          <template #default="{ row }">{{ row.period_name || `-${row.period}` }}</template>
        </el-table-column>
        <el-table-column prop="year" label="年度" width="90" align="center" />
        <el-table-column prop="period" label="期间" width="80" align="center" />
        <el-table-column prop="start_date" label="开始日期" min-width="160">
          <template #default="{ row }">{{ formatDate(row.start_date) }}</template>
        </el-table-column>
        <el-table-column prop="end_date" label="结束日期" min-width="160">
          <template #default="{ row }">{{ formatDate(row.end_date) }}</template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="statusTagMap[row.status as PeriodStatus] ?? 'info'">
              {{ statusTextMap[row.status as PeriodStatus] ?? row.status }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="closed_at" label="关账时间" min-width="160">
          <template #default="{ row }">{{
            row.closed_at ? formatDate(row.closed_at) : '-'
          }}</template>
        </el-table-column>
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <el-button size="small" link type="primary" @click="handleDetail(row)">详情</el-button>
            <el-button
              v-if="row.status === 'OPEN'"
              size="small"
              link
              type="danger"
              @click="handleToggle(row, 'close')"
            >
              关账停用
            </el-button>
            <el-button
              v-if="row.status === 'CLOSED'"
              size="small"
              link
              type="success"
              @click="handleToggle(row, 'reopen')"
            >
              重开启用
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="createVisible" title="新建会计期间" width="440px" @close="resetForm">
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="100px">
        <el-form-item label="年度" prop="year">
          <el-input-number
            v-model="formData.year"
            :min="2000"
            :max="2100"
            :precision="0"
            style="width: 100%"
            placeholder="必填"
          />
        </el-form-item>
        <el-form-item label="期间" prop="period">
          <el-input-number
            v-model="formData.period"
            :min="1"
            :max="12"
            :precision="0"
            style="width: 100%"
            placeholder="1-12"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="createVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="submitCreate">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="detailVisible" title="会计期间详情" width="560px">
      <el-descriptions v-if="detailRow" :column="2" border>
        <el-descriptions-item label="期间名称">{{ detailRow.period_name }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag :type="statusTagMap[detailRow.status as PeriodStatus] ?? 'info'">
            {{ statusTextMap[detailRow.status as PeriodStatus] ?? detailRow.status }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="年度">{{ detailRow.year }}</el-descriptions-item>
        <el-descriptions-item label="期间">{{ detailRow.period }}</el-descriptions-item>
        <el-descriptions-item label="开始日期">{{
          formatDate(detailRow.start_date)
        }}</el-descriptions-item>
        <el-descriptions-item label="结束日期">{{
          formatDate(detailRow.end_date)
        }}</el-descriptions-item>
        <el-descriptions-item label="关账时间">
          {{ detailRow.closed_at ? formatDate(detailRow.closed_at) : '-' }}
        </el-descriptions-item>
        <el-descriptions-item label="关账人">{{ detailRow.closed_by ?? '-' }}</el-descriptions-item>
        <el-descriptions-item label="创建时间" :span="2">
          {{ formatDate(detailRow.created_at) }}
        </el-descriptions-item>
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
  getAccountingPeriodList,
  getAccountingPeriod,
  createPeriodByYearPeriod,
  closePeriod,
  reopenPeriod,
  initPeriod,
  yearEndClosing,
  type AccountingPeriodDetail,
} from '@/api/accounting-period';

type PeriodStatus = 'OPEN' | 'CLOSED';

const statusTextMap: Record<PeriodStatus, string> = {
  OPEN: '开放',
  CLOSED: '已关账',
};

const statusTagMap: Record<PeriodStatus, 'info' | 'success' | 'warning'> = {
  OPEN: 'success',
  CLOSED: 'info',
};

const loading = ref(false);
const submitLoading = ref(false);
const periodList = ref<AccountingPeriodDetail[]>([]);
const createVisible = ref(false);
const detailVisible = ref(false);
const detailRow = ref<AccountingPeriodDetail | null>(null);
const formRef = ref<FormInstance>();

const formData = reactive<{
  year: number | undefined;
  period: number | undefined;
}>({
  year: undefined,
  period: undefined,
});

const formRules: FormRules = {
  year: [{ required: true, message: '请输入年度', trigger: 'blur' }],
  period: [{ required: true, message: '请输入期间（1-12）', trigger: 'blur' }],
};

const formatDate = (value: string | null): string => {
  if (!value) return '-';
  return value.replace('T', ' ').slice(0, 19);
};

/** 响应解包防御：兼容数组 / { items } 分页包装，避免 el-table "r is not iterable" */
const unwrapList = (payload: unknown): AccountingPeriodDetail[] => {
  if (Array.isArray(payload)) return payload;
  const paged = payload as { items?: AccountingPeriodDetail[] } | null;
  return paged?.items ?? [];
};

const loadList = async () => {
  loading.value = true;
  try {
    const res = await getAccountingPeriodList();
    periodList.value = unwrapList(res.data);
  } catch {
    ElMessage.error('加载会计期间列表失败');
  } finally {
    loading.value = false;
  }
};

/** 启停：OPEN → 关账停用（close）；CLOSED → 重开启用（reopen） */
const handleToggle = async (row: AccountingPeriodDetail, mode: 'close' | 'reopen') => {
  const label = mode === 'close' ? '关账停用' : '重开启用';
  try {
    await ElMessageBox.confirm(
      `确认对期间 ${row.period_name || `${row.year}-${row.period}`} 执行「${label}」吗？`,
      '操作确认',
      { confirmButtonText: '确认', cancelButtonText: '取消', type: 'warning' }
    );
    if (mode === 'close') {
      await closePeriod(row.id);
    } else {
      await reopenPeriod(row.id);
    }
    ElMessage.success(`操作成功：${label}`);
    await loadList();
  } catch (error) {
    if (error === 'cancel' || (error as { message?: string })?.message === 'cancel') return;
    ElMessage.error(`操作失败：${label}`);
  }
};

const handleInit = async () => {
  try {
    await ElMessageBox.confirm(
      '将初始化当前财务期间（不存在时创建当年当月期间），确认执行吗？',
      '初始化确认',
      {
        confirmButtonText: '确认',
        cancelButtonText: '取消',
        type: 'warning',
      }
    );
    await initPeriod();
    ElMessage.success('初始化成功');
    await loadList();
  } catch (error) {
    if (error === 'cancel' || (error as { message?: string })?.message === 'cancel') return;
    ElMessage.error('初始化失败');
  }
};

const handleYearEnd = async () => {
  try {
    const { value } = await ElMessageBox.prompt(
      '年度结账要求该年度 12 个期间全部已关账，将结转损益并创建下一年 1 月期间。请输入结账年度。',
      '年度结账',
      {
        confirmButtonText: '确认结账',
        cancelButtonText: '取消',
        inputPattern: /^\d{4}$/,
        inputErrorMessage: '请输入 4 位数字年度',
        inputPlaceholder: '如 2026',
      }
    );
    const res = await yearEndClosing(Number(value));
    const result = res.data;
    ElMessage.success(
      result
        ? `年度结账成功：${result.year} → ${result.next_year}，结转科目 ${result.transferred_subjects} 个`
        : '年度结账成功'
    );
    await loadList();
  } catch (error) {
    if (error === 'cancel' || (error as { message?: string })?.message === 'cancel') return;
    ElMessage.error('年度结账失败');
  }
};

const handleCreate = () => {
  createVisible.value = true;
};

const resetForm = () => {
  formData.year = undefined;
  formData.period = undefined;
  formRef.value?.resetFields();
};

/** 详情：按行内数据展示，并回源 GET /finance/accounting-periods/{id} 保证最新 */
const handleDetail = async (row: AccountingPeriodDetail) => {
  detailRow.value = row;
  detailVisible.value = true;
  try {
    const res = await getAccountingPeriod(row.id);
    if (res.data) detailRow.value = res.data as AccountingPeriodDetail;
  } catch {
    ElMessage.error('加载会计期间详情失败');
  }
};

const submitCreate = async () => {
  if (!formRef.value) return;
  await formRef.value.validate(async valid => {
    if (!valid) return;
    if (!formData.year || !formData.period) return;
    submitLoading.value = true;
    try {
      await createPeriodByYearPeriod({ year: formData.year, period: formData.period });
      ElMessage.success('会计期间创建成功');
      createVisible.value = false;
      await loadList();
    } catch {
      ElMessage.error('会计期间创建失败');
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
.periods-page {
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
