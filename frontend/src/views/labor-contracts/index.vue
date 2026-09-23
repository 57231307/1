<template>
  <div class="page">
    <el-card shadow="never">
      <template #header>
        <div class="card-header">
          <span>劳动合同管理</span>
          <span class="toolbar">
            <el-button plain @click="onScanWarnings">扫描到期预警</el-button>
            <el-button type="primary" @click="openCreate">新建合同</el-button>
          </span>
        </div>
      </template>
      <pre v-if="warnings" class="result-box mb">{{ warnings }}</pre>
      <el-table v-loading="loading" :data="contracts" border>
        <el-table-column prop="id" label="ID" width="70" />
        <el-table-column prop="contract_no" label="合同号" width="150" />
        <el-table-column prop="worker_id" label="员工ID" width="90" />
        <el-table-column prop="contract_type" label="类型" width="110" />
        <el-table-column label="状态" width="110">
          <template #default="{ row }">
            <el-tag :type="statusTag(row.status)">{{ statusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="start_date" label="起" width="110" />
        <el-table-column prop="end_date" label="止" width="110" />
        <el-table-column prop="regular_salary" label="转正薪资" width="110" />
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <el-button size="small" @click="onEdit(row)">编辑/续签</el-button>
            <el-button
              v-if="row.status !== 'terminated'"
              size="small"
              type="danger"
              plain
              @click="onTerminate(row)"
              >解除</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog
      v-model="dialogVisible"
      :title="editingId ? '编辑/续签合同' : '新建合同'"
      width="560"
    >
      <el-form :model="form" label-width="110px">
        <el-form-item label="员工ID" required
          ><el-input-number
            v-model="form.worker_id"
            :min="1"
            :disabled="!!editingId"
            class="w-full"
        /></el-form-item>
        <el-form-item label="合同号" required
          ><el-input v-model="form.contract_no" readonly
        /></el-form-item>
        <el-form-item label="类型" required>
          <el-select v-model="form.contract_type" class="w-full">
            <el-option
              v-for="t in ['fixed_term', 'open_term', 'probation', 'intern']"
              :key="t"
              :label="t"
              :value="t"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="起始日期" required
          ><el-date-picker
            v-model="form.start_date"
            type="date"
            value-format="YYYY-MM-DD"
            class="w-full"
        /></el-form-item>
        <el-form-item label="终止日期"
          ><el-date-picker
            v-model="form.end_date"
            type="date"
            value-format="YYYY-MM-DD"
            class="w-full"
        /></el-form-item>
        <el-form-item label="试用止期"
          ><el-date-picker
            v-model="form.probation_end_date"
            type="date"
            value-format="YYYY-MM-DD"
            class="w-full"
        /></el-form-item>
        <el-form-item label="试用期薪资" required
          ><el-input-number v-model="form.probation_salary" :min="0" :precision="2" class="w-full"
        /></el-form-item>
        <el-form-item label="转正薪资" required
          ><el-input-number v-model="form.regular_salary" :min="0" :precision="2" class="w-full"
        /></el-form-item>
        <el-form-item label="岗位"><el-input v-model="form.position" /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="onSave">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import { generateUniqueDocNo } from '@/utils/document-no';
import {
  createLaborContract,
  getLaborContractList,
  scanContractExpiryWarnings,
  terminateLaborContract,
  updateLaborContract,
  type LaborContract,
} from '@/api/labor-contract';

const contracts = ref<LaborContract[]>([]);
const warnings = ref('');
const loading = ref(false);
const saving = ref(false);
const dialogVisible = ref(false);
const editingId = ref<number | null>(null);

/** 打开新建合同：自动预生成合同号（查重唯一后只读展示，防手动输入重复） */
const openCreate = async () => {
  form.contract_no = await generateUniqueDocNo('LC', 'labor_contract');
  dialogVisible.value = true;
};

const form = reactive({
  worker_id: undefined as number | undefined,
  contract_no: '',
  contract_type: 'fixed_term',
  start_date: '',
  end_date: '',
  probation_end_date: '',
  probation_salary: 0,
  regular_salary: 0,
  position: '',
});

const unwrapList = <T,>(p: unknown): T[] => (p as { data: { list: T[] } }).data.list;

const statusLabel = (s: string) =>
  ({ active: '生效中', expired: '已到期', terminated: '已解除', pending: '待生效' })[s] ?? s;
const statusTag = (s: string) =>
  ({ active: 'success', expired: 'warning', terminated: 'danger', pending: 'info' })[s] ?? 'info';

async function load() {
  loading.value = true;
  try {
    contracts.value = unwrapList(await getLaborContractList());
  } finally {
    loading.value = false;
  }
}

async function onScanWarnings() {
  warnings.value = JSON.stringify(await scanContractExpiryWarnings(), null, 2);
}

async function onSave() {
  if (!form.worker_id || !form.contract_no || !form.start_date) {
    ElMessage.warning('请填写员工/合同号/起始日期');
    return;
  }
  saving.value = true;
  try {
    const payload = {
      worker_id: form.worker_id,
      contract_no: form.contract_no,
      contract_type: form.contract_type,
      start_date: form.start_date,
      end_date: form.end_date || undefined,
      probation_end_date: form.probation_end_date || undefined,
      probation_salary: form.probation_salary,
      regular_salary: form.regular_salary,
      position: form.position || undefined,
    };
    if (editingId.value) {
      await updateLaborContract(editingId.value, payload);
      ElMessage.success('已更新');
    } else {
      await createLaborContract(payload);
      ElMessage.success('合同已创建');
    }
    dialogVisible.value = false;
    editingId.value = null;
    await load();
  } finally {
    saving.value = false;
  }
}

function onEdit(row: LaborContract) {
  editingId.value = row.id;
  Object.assign(form, {
    worker_id: row.worker_id as number,
    contract_no: row.contract_no as string,
    contract_type: (row.contract_type as string) ?? 'fixed_term',
    start_date: (row.start_date as string) ?? '',
    end_date: (row.end_date as string) ?? '',
    probation_end_date: (row.probation_end_date as string) ?? '',
    probation_salary: Number(row.probation_salary ?? 0),
    regular_salary: Number(row.regular_salary ?? 0),
    position: (row.position as string) ?? '',
  });
  dialogVisible.value = true;
}

async function onTerminate(row: LaborContract) {
  const { value: date } = await ElMessageBox.prompt('解除日期（YYYY-MM-DD）', '解除合同');
  const { value: reason } = await ElMessageBox.prompt('解除原因', '解除合同');
  if (!date || !reason) {
    ElMessage.warning('请填写解除日期与原因');
    return;
  }
  await terminateLaborContract(row.id, { termination_date: date, termination_reason: reason });
  ElMessage.success('已解除');
  await load();
}

onMounted(load);
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.toolbar {
  display: flex;
  gap: 8px;
}
.w-full {
  width: 100%;
}
.mb {
  margin-bottom: 12px;
}
.result-box {
  background: var(--el-fill-color-light);
  border-radius: 4px;
  padding: 12px;
  font-size: 12px;
  max-height: 220px;
  overflow: auto;
  white-space: pre-wrap;
}
</style>
