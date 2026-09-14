<template>
  <div class="page">
    <el-card shadow="never">
      <template #header>
        <div class="card-header">
          <span>坏账管理</span>
          <el-button type="primary" @click="provisionVisible = true">运行计提</el-button>
        </div>
      </template>
      <el-table v-loading="loading" :data="debts" border>
        <el-table-column prop="id" label="ID" width="70" />
        <el-table-column label="状态" width="110">
          <template #default="{ row }">
            <el-tag :type="row.status === 'confirmed' ? 'success' : row.status === 'reversed' ? 'info' : 'warning'">
              {{ BAD_DEBT_STATUS_LABEL[row.status] ?? row.status }}
            </el-tag>
          </template>
        </el-table-column>
        <template v-for="col in debtExtraCols" :key="col.prop">
          <el-table-column :prop="col.prop" :label="col.label" min-width="140" show-overflow-tooltip />
        </template>
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <el-button v-if="row.status === 'pending'" size="small" type="primary" @click="onConfirm(row)">确认</el-button>
            <el-button v-if="row.status === 'confirmed'" size="small" type="warning" @click="onReverse(row)">冲销</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-card shadow="never" class="mt">
      <template #header><span>催收任务</span></template>
      <el-table v-loading="loadingTasks" :data="tasks" border>
        <el-table-column prop="id" label="ID" width="70" />
        <el-table-column label="状态" width="110">
          <template #default="{ row }">
            <el-tag :type="row.status === 'cancelled' ? 'info' : row.status === 'completed' ? 'success' : 'primary'">
              {{ COLLECTION_TASK_STATUS_LABEL[row.status] ?? row.status }}
            </el-tag>
          </template>
        </el-table-column>
        <template v-for="col in taskExtraCols" :key="col.prop">
          <el-table-column :prop="col.prop" :label="col.label" min-width="140" show-overflow-tooltip />
        </template>
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <el-button v-if="row.status !== 'cancelled' && row.status !== 'completed'" size="small" @click="onReassign(row)">转派</el-button>
            <el-button v-if="row.status !== 'cancelled'" size="small" type="danger" plain @click="onCancelTask(row)">取消</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="provisionVisible" title="运行坏账计提" width="460">
      <el-form :model="provisionForm" label-width="120px">
        <el-form-item label="账期" required>
          <el-input v-model="provisionForm.period" placeholder="如 2026-08" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="provisionVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="onProvision">执行</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  cancelCollectionTask,
  confirmBadDebt,
  getBadDebtList,
  getCollectionTaskList,
  reassignCollectionTask,
  reverseBadDebt,
  runProvision,
  BAD_DEBT_STATUS_LABEL,
  COLLECTION_TASK_STATUS_LABEL,
  type BadDebt,
  type CollectionTask,
} from '@/api/bad-debt';

const debts = ref<BadDebt[]>([]);
const tasks = ref<CollectionTask[]>([]);
const loading = ref(false);
const loadingTasks = ref(false);
const saving = ref(false);
const provisionVisible = ref(false);
const provisionForm = reactive({ period: '' });

const unwrapList = <T,>(p: unknown): T[] =>
  Array.isArray(p) ? p : ((p as { items?: T[] })?.items ?? []);

const debtExtraCols = computed(() => extraCols(debts.value));
const taskExtraCols = computed(() => extraCols(tasks.value));

function extraCols(rows: Array<Record<string, unknown>>) {
  const skip = new Set(['id', 'status', 'created_at', 'updated_at']);
  const cols: Array<{ prop: string; label: string }> = [];
  const sample = rows[0] ?? {};
  for (const key of Object.keys(sample)) {
    if (skip.has(key) || cols.length >= 5) continue;
    if (typeof sample[key] === 'object' && sample[key] !== null) continue;
    cols.push({ prop: key, label: key.replace(/_/g, ' ') });
  }
  return cols;
}

async function load() {
  loading.value = true;
  loadingTasks.value = true;
  try {
    const [d, t] = await Promise.all([getBadDebtList(), getCollectionTaskList()]);
    debts.value = unwrapList<BadDebt>(d);
    tasks.value = unwrapList<CollectionTask>(t);
  } finally {
    loading.value = false;
    loadingTasks.value = false;
  }
}

async function onProvision() {
  if (!provisionForm.period) {
    ElMessage.warning('请填写账期');
    return;
  }
  saving.value = true;
  try {
    await runProvision({ period: provisionForm.period });
    ElMessage.success('计提完成');
    provisionVisible.value = false;
    await load();
  } finally {
    saving.value = false;
  }
}

async function onConfirm(row: BadDebt) {
  await ElMessageBox.confirm('确认该坏账计提？', '确认');
  await confirmBadDebt(row.id);
  ElMessage.success('已确认');
  await load();
}

async function onReverse(row: BadDebt) {
  await ElMessageBox.confirm('确认冲销该坏账？', '冲销确认');
  await reverseBadDebt(row.id);
  ElMessage.success('已冲销');
  await load();
}

async function onReassign(row: CollectionTask) {
  const { value } = await ElMessageBox.prompt('请输入新负责人用户 ID', '转派任务');
  await reassignCollectionTask(row.id, { assignee_id: Number(value) });
  ElMessage.success('已转派');
  await load();
}

async function onCancelTask(row: CollectionTask) {
  await ElMessageBox.confirm('确认取消该催收任务？', '取消确认');
  await cancelCollectionTask(row.id);
  ElMessage.success('已取消');
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
.mt {
  margin-top: 16px;
}
</style>
