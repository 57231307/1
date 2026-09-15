<template>
  <div class="page">
    <el-card shadow="never">
      <template #header>
        <div class="card-header">
          <span>产量工资</span>
          <el-button type="primary" @click="dialogVisible = true">新建工资单</el-button>
        </div>
      </template>
      <el-table v-loading="loading" :data="records" border>
        <el-table-column prop="id" label="ID" width="70" />
        <el-table-column label="状态" width="110">
          <template #default="{ row }">
            <el-tag :type="statusTag(row.status)">{{
              WAGE_RECORD_STATUS_LABEL[row.status] ?? row.status
            }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="period_start" label="期间起" width="120" />
        <el-table-column prop="period_end" label="期间止" width="120" />
        <el-table-column prop="workshop" label="车间" min-width="120" show-overflow-tooltip />
        <el-table-column prop="remarks" label="备注" min-width="140" show-overflow-tooltip />
        <el-table-column label="操作" width="330" fixed="right">
          <template #default="{ row }">
            <el-button
              v-if="row.status === 'draft'"
              size="small"
              type="primary"
              @click="onCalculate(row)"
              >核算</el-button
            >
            <el-button
              v-if="row.status === 'calculated'"
              size="small"
              type="success"
              @click="onConfirm(row)"
              >确认</el-button
            >
            <el-button
              v-if="row.status === 'confirmed'"
              size="small"
              type="success"
              plain
              @click="onPay(row)"
              >发放</el-button
            >
            <el-button
              v-if="row.status === 'draft' || row.status === 'calculated'"
              size="small"
              type="danger"
              plain
              @click="onCancel(row)"
              >取消</el-button
            >
            <el-button size="small" @click="onDetails(row)">明细</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="dialogVisible" title="新建工资单" width="480">
      <el-form :model="form" label-width="100px">
        <el-form-item label="期间起" required>
          <el-date-picker
            v-model="form.period_start"
            type="date"
            value-format="YYYY-MM-DD"
            class="w-full"
          />
        </el-form-item>
        <el-form-item label="期间止" required>
          <el-date-picker
            v-model="form.period_end"
            type="date"
            value-format="YYYY-MM-DD"
            class="w-full"
          />
        </el-form-item>
        <el-form-item label="车间">
          <el-input v-model="form.workshop" />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="form.remarks" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="onCreate">保存</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="detailsVisible" title="工资明细" width="720">
      <el-table :data="details" border max-height="420">
        <el-table-column
          v-for="col in detailCols"
          :key="col"
          :prop="col"
          :label="col.replace(/_/g, ' ')"
          min-width="120"
          show-overflow-tooltip
        />
      </el-table>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  calculateWageRecord,
  cancelWageRecord,
  confirmWageRecord,
  createWageRecord,
  getWageRecordDetails,
  getWageRecordList,
  payWageRecord,
  WAGE_RECORD_STATUS_LABEL,
  type WageRecord,
} from '@/api/wage';

const records = ref<WageRecord[]>([]);
const details = ref<Array<Record<string, unknown>>>([]);
const loading = ref(false);
const saving = ref(false);
const dialogVisible = ref(false);
const detailsVisible = ref(false);
const detailCols = ref<string[]>([]);

const form = reactive({ period_start: '', period_end: '', workshop: '', remarks: '' });

const unwrapList = <T,>(p: unknown): T[] =>
  Array.isArray(p) ? p : ((p as { items?: T[] })?.items ?? []);

const statusTag = (s: string) =>
  ({
    draft: 'info',
    calculated: 'warning',
    confirmed: 'primary',
    paid: 'success',
    cancelled: 'danger',
  })[s] ?? 'info';

async function load() {
  loading.value = true;
  try {
    records.value = unwrapList(await getWageRecordList());
  } finally {
    loading.value = false;
  }
}

async function onCreate() {
  if (!form.period_start || !form.period_end) {
    ElMessage.warning('请选择期间');
    return;
  }
  saving.value = true;
  try {
    await createWageRecord({
      period_start: form.period_start,
      period_end: form.period_end,
      workshop: form.workshop || undefined,
      remarks: form.remarks || undefined,
    });
    ElMessage.success('工资单已创建');
    dialogVisible.value = false;
    await load();
  } finally {
    saving.value = false;
  }
}

const act = async (row: WageRecord, fn: (id: number) => Promise<unknown>, msg: string) => {
  await ElMessageBox.confirm(`确认${msg}？`, '确认');
  await fn(row.id);
  ElMessage.success(msg + '成功');
  await load();
};

const onCalculate = (row: WageRecord) => act(row, id => calculateWageRecord(id), '核算');
const onConfirm = (row: WageRecord) => act(row, confirmWageRecord, '确认');
const onPay = (row: WageRecord) => act(row, payWageRecord, '发放');
const onCancel = (row: WageRecord) => act(row, cancelWageRecord, '取消');

async function onDetails(row: WageRecord) {
  const payload = await getWageRecordDetails(row.id);
  details.value = unwrapList(payload);
  detailCols.value = details.value.length ? Object.keys(details.value[0]).slice(0, 8) : [];
  detailsVisible.value = true;
}

onMounted(load);
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.w-full {
  width: 100%;
}
</style>
