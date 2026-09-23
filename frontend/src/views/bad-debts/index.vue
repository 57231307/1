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
            <el-tag
              :type="
                row.status === 'confirmed'
                  ? 'success'
                  : row.status === 'reversed'
                    ? 'info'
                    : 'warning'
              "
            >
              {{ BAD_DEBT_STATUS_LABEL[row.status] ?? row.status }}
            </el-tag>
          </template>
        </el-table-column>
        <template v-for="col in debtExtraCols" :key="col.prop">
          <el-table-column
            :prop="col.prop"
            :label="col.label"
            min-width="140"
            show-overflow-tooltip
          />
        </template>
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <el-button
              v-if="row.status === 'pending'"
              size="small"
              type="primary"
              @click="onConfirm(row)"
              >确认</el-button
            >
            <el-button
              v-if="row.status === 'confirmed'"
              size="small"
              type="warning"
              @click="onReverse(row)"
              >冲销</el-button
            >
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
            <el-tag
              :type="
                row.status === 'cancelled'
                  ? 'info'
                  : row.status === 'completed'
                    ? 'success'
                    : 'primary'
              "
            >
              {{ COLLECTION_TASK_STATUS_LABEL[row.status] ?? row.status }}
            </el-tag>
          </template>
        </el-table-column>
        <template v-for="col in taskExtraCols" :key="col.prop">
          <el-table-column
            :prop="col.prop"
            :label="col.label"
            min-width="140"
            show-overflow-tooltip
          />
        </template>
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <el-button
              v-if="row.status !== 'cancelled' && row.status !== 'completed'"
              size="small"
              @click="onReassign(row)"
              >转派</el-button
            >
            <el-button
              v-if="row.status !== 'cancelled'"
              size="small"
              type="danger"
              plain
              @click="onCancelTask(row)"
              >取消</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="provisionVisible" title="运行坏账计提" width="460">
      <el-form :model="provisionForm" label-width="120px">
        <el-form-item label="计提年度" required>
          <el-input-number
            v-model="provisionForm.period_year"
            :min="2000"
            :max="2100"
            class="w-full"
          />
        </el-form-item>
        <el-form-item label="计提月份" required>
          <el-input-number v-model="provisionForm.period_month" :min="1" :max="12" class="w-full" />
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
import { useI18n } from 'vue-i18n';
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
const provisionForm = reactive({
  period_year: new Date().getFullYear(),
  period_month: new Date().getMonth() + 1,
});

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
  saving.value = true;
  try {
    await runProvision({
      period_year: provisionForm.period_year,
      period_month: provisionForm.period_month,
    });
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

const { t } = useI18n({ useScope: 'global' });

async function onReassign(row: CollectionTask) {
  // 不加 inputPattern 时留空会以 Number('') === 0 提交，把催收任务派给不存在的用户 ID=0，
  // 后端按 i32 正常收下 ⇒ 派单"看起来成功了"，任务却落进无人处理的账户。
  const { value } = await ElMessageBox.prompt(
    t('badDebts.reassign.prompt'),
    t('badDebts.reassign.title'),
    {
      inputPattern: /^\d+$/,
      inputErrorMessage: t('badDebts.reassign.userIdInvalid'),
    }
  );
  await reassignCollectionTask(row.id, { assigned_to: Number(value) });
  ElMessage.success(t('badDebts.reassign.success'));
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
