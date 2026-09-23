<template>
  <div class="page">
    <el-tabs v-model="activeTab">
      <el-tab-pane label="工资单" name="records">
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
                <el-button
                  v-if="row.status === 'draft'"
                  size="small"
                  type="warning"
                  plain
                  @click="onEditRecord(row)"
                  >编辑</el-button
                >
                <el-button
                  v-if="row.status === 'draft'"
                  size="small"
                  type="danger"
                  @click="onDeleteRecord(row)"
                  >删除</el-button
                >
                <el-button size="small" @click="onDetails(row)">明细</el-button>
                <el-button size="small" type="success" plain @click="onExportDetails(row)">
                  导出
                </el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="工资费率" name="rates">
        <div class="card-header" style="margin-bottom: 12px">
          <el-button type="primary" @click="rateDialogVisible = true">新建费率</el-button>
          <el-button plain @click="effectiveVisible = true">生效费率查询</el-button>
        </div>
        <el-table v-loading="rateLoading" :data="wageRates" border>
          <el-table-column prop="id" label="ID" width="70" />
          <el-table-column prop="route_id" label="工序路线 ID" width="110" />
          <el-table-column prop="process_name" label="工序" min-width="130" />
          <el-table-column prop="wage_rate" label="费率" width="110" align="right">
            <template #default="{ row }">{{
              Number(row.wage_rate ?? 0).toLocaleString()
            }}</template>
          </el-table-column>
          <el-table-column prop="effective_date" label="生效日期" width="120" />
          <el-table-column prop="status" label="状态" width="100">
            <template #default="{ row }">
              <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
                {{ row.status }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column label="操作" width="220" fixed="right">
            <template #default="{ row }">
              <el-button size="small" @click="onEditRate(row)">编辑</el-button>
              <el-button
                v-if="row.status !== 'active'"
                size="small"
                type="success"
                @click="onActivateRate(row)"
                >激活</el-button
              >
              <el-button
                v-if="row.status === 'active'"
                size="small"
                type="warning"
                @click="onDisableRate(row)"
                >停用</el-button
              >
              <el-button size="small" type="danger" plain @click="onDeleteRate(row)"
                >删除</el-button
              >
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>
    </el-tabs>

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
      <div class="toolbar" style="margin-bottom: 8px">
        <el-input-number v-model="workerId" :min="1" placeholder="工人ID" style="width: 140px" />
        <el-button type="primary" plain :loading="workerLoading" @click="onWorkerDetails">
          按工人查询工资历史
        </el-button>
      </div>
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

    <!-- 编辑工资单（updateWageRecord：draft 态修改车间/备注） -->
    <el-dialog v-model="recordEditVisible" title="编辑工资单" width="480">
      <el-form :model="recordEditForm" label-width="90px">
        <el-form-item label="车间">
          <el-input v-model="recordEditForm.workshop" />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="recordEditForm.remarks" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="recordEditVisible = false">取消</el-button>
        <el-button type="primary" :loading="recordSaving" @click="onSaveRecord">保存</el-button>
      </template>
    </el-dialog>

    <!-- 生效费率查询（getEffectiveWageRate） -->
    <el-dialog v-model="effectiveVisible" title="生效费率查询" width="440">
      <div class="toolbar" style="margin-bottom: 8px">
        <el-input-number v-model="effectiveForm.route_id" :min="1" placeholder="工序路线ID" />
        <el-input v-model="effectiveForm.date" placeholder="日期 2026-01-01" style="width: 160px" />
        <el-button type="primary" :loading="effectiveLoading" @click="onQueryEffective">
          查询
        </el-button>
      </div>
      <pre v-if="effectiveResult" class="effective-result">{{ effectiveResult }}</pre>
    </el-dialog>

    <!-- 工资费率新建/编辑（createWageRate/updateWageRate） -->
    <el-dialog v-model="rateDialogVisible" title="工资费率" width="440">
      <el-form :model="rateForm" label-width="90px">
        <el-form-item label="工序路线" required>
          <el-input-number v-model="rateForm.route_id" :min="1" class="w-full" />
        </el-form-item>
        <el-form-item label="工序名称" required>
          <el-input v-model="rateForm.process_name" />
        </el-form-item>
        <el-form-item label="费率">
          <el-input-number v-model="rateForm.wage_rate" :min="0" :precision="2" class="w-full" />
        </el-form-item>
        <el-form-item label="生效日期">
          <el-date-picker
            v-model="rateForm.effective_date"
            type="date"
            value-format="YYYY-MM-DD"
            class="w-full"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="rateDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="rateSaving" @click="onSaveRate">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import type { ApiResponse } from '@/types/api';
import {
  calculateWageRecord,
  cancelWageRecord,
  confirmWageRecord,
  createWageRecord,
  getWageRecordDetails,
  getWageRecordList,
  payWageRecord,
  getWageRateList,
  createWageRate,
  updateWageRate,
  deleteWageRate,
  activateWageRate,
  disableWageRate,
  updateWageRecord,
  deleteWageRecord,
  exportWageRecordDetails,
  getWageDetailsByWorker,
  getEffectiveWageRate,
  WAGE_RECORD_STATUS_LABEL,
  type WageRecord,
  type CreateWageRatePayload,
} from '@/api/wage';

const records = ref<WageRecord[]>([]);
const details = ref<Array<Record<string, unknown>>>([]);
const loading = ref(false);
const saving = ref(false);
const dialogVisible = ref(false);
const detailsVisible = ref(false);
const detailCols = ref<string[]>([]);

const form = reactive({ period_start: '', period_end: '', workshop: '', remarks: '' });

const unwrapList = <T,>(p: unknown): T[] => (p as { data: { items: T[] } }).data.items;

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

// ===== 编辑/删除工资单（draft 态） =====
const recordEditVisible = ref(false);
const recordSaving = ref(false);
const recordEditForm = reactive({ id: 0, workshop: '', remarks: '' });

function onEditRecord(row: WageRecord) {
  recordEditForm.id = row.id;
  recordEditForm.workshop = (row as Record<string, unknown>).workshop as string;
  recordEditForm.remarks = (row as Record<string, unknown>).remarks as string;
  recordEditVisible.value = true;
}

async function onSaveRecord() {
  recordSaving.value = true;
  try {
    await updateWageRecord(recordEditForm.id, {
      workshop: recordEditForm.workshop || undefined,
      remarks: recordEditForm.remarks || undefined,
    });
    ElMessage.success('工资单已更新');
    recordEditVisible.value = false;
    await load();
  } catch (e) {
    ElMessage.error((e as Error).message || '更新失败');
  } finally {
    recordSaving.value = false;
  }
}

async function onDeleteRecord(row: WageRecord) {
  try {
    await ElMessageBox.confirm(`确认删除工资单 #${row.id}？`, '删除确认', { type: 'warning' });
  } catch {
    return;
  }
  try {
    await deleteWageRecord(row.id);
    ElMessage.success('删除成功');
    await load();
  } catch (e) {
    ElMessage.error((e as Error).message || '删除失败');
  }
}

// ===== 导出工资明细（xlsx blob 下载） =====
async function onExportDetails(row: WageRecord) {
  try {
    const blob = await exportWageRecordDetails(row.id);
    const url = URL.createObjectURL(blob as unknown as Blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `wage_details_${row.id}.xlsx`;
    a.click();
    URL.revokeObjectURL(url);
    ElMessage.success('明细已导出');
  } catch (e) {
    ElMessage.error((e as Error).message || '导出失败');
  }
}

// ===== 按工人查询工资历史（getWageDetailsByWorker） =====
const workerId = ref(1);
const workerLoading = ref(false);

async function onWorkerDetails() {
  workerLoading.value = true;
  try {
    const payload = await getWageDetailsByWorker(workerId.value);
    details.value = unwrapList(payload);
    detailCols.value = details.value.length ? Object.keys(details.value[0]).slice(0, 8) : [];
    ElMessage.success(details.value.length ? '' : '暂无该工人工资历史');
  } catch (e) {
    ElMessage.error((e as Error).message || '查询失败');
  } finally {
    workerLoading.value = false;
  }
}

// ===== 生效费率查询（getEffectiveWageRate） =====
const effectiveVisible = ref(false);
const effectiveLoading = ref(false);
const effectiveResult = ref('');
const effectiveForm = reactive({ route_id: 1, date: '' });

async function onQueryEffective() {
  if (!effectiveForm.date) {
    ElMessage.warning('请输入查询日期');
    return;
  }
  effectiveLoading.value = true;
  try {
    const res = (await getEffectiveWageRate(
      effectiveForm.route_id,
      effectiveForm.date
    )) as ApiResponse<unknown>;
    effectiveResult.value = JSON.stringify(res.data ?? res, null, 2);
  } catch (e) {
    ElMessage.error((e as Error).message || '查询失败');
  } finally {
    effectiveLoading.value = false;
  }
}

onMounted(load);

// ==================== 工资费率管理（wage-rates） ====================
const activeTab = ref('records');
const wageRates = ref<Array<Record<string, unknown>>>([]);
const rateLoading = ref(false);
const rateDialogVisible = ref(false);
const rateSaving = ref(false);
const editingRateId = ref<number | null>(null);
const rateForm = reactive({
  route_id: undefined as number | undefined,
  process_name: '',
  wage_rate: 0,
  effective_date: new Date().toISOString().split('T')[0],
  status: 'active',
});

const loadRates = async () => {
  rateLoading.value = true;
  try {
    const res = (await getWageRateList()) as ApiResponse<unknown>;
    const d = res.data as unknown;
    wageRates.value = Array.isArray(d)
      ? (d as Array<Record<string, unknown>>)
      : ((d as { items?: Array<Record<string, unknown>> })?.items ?? []);
  } catch (e) {
    ElMessage.error((e as Error).message || '加载费率失败');
  } finally {
    rateLoading.value = false;
  }
};

const onSaveRate = async () => {
  if (!rateForm.route_id || !rateForm.process_name) {
    ElMessage.warning('工序路线与工序名称必填');
    return;
  }
  rateSaving.value = true;
  try {
    if (editingRateId.value) {
      await updateWageRate(editingRateId.value, rateForm);
    } else {
      await createWageRate(rateForm as unknown as CreateWageRatePayload);
    }
    ElMessage.success('保存成功');
    rateDialogVisible.value = false;
    loadRates();
  } catch (e) {
    ElMessage.error((e as Error).message || '保存失败');
  } finally {
    rateSaving.value = false;
  }
};

const onEditRate = (row: Record<string, unknown>) => {
  editingRateId.value = Number(row.id);
  rateForm.route_id = Number(row.route_id ?? 0);
  rateForm.process_name = String(row.process_name || '');
  rateForm.wage_rate = Number(row.wage_rate ?? 0);
  rateForm.effective_date = String(row.effective_date || new Date().toISOString().split('T')[0]);
  rateForm.status = String(row.status || 'active');
  rateDialogVisible.value = true;
};

const onActivateRate = async (row: Record<string, unknown>) => {
  try {
    await activateWageRate(Number(row.id));
    ElMessage.success('已激活');
    loadRates();
  } catch (e) {
    ElMessage.error((e as Error).message || '激活失败');
  }
};

const onDisableRate = async (row: Record<string, unknown>) => {
  try {
    await disableWageRate(Number(row.id));
    ElMessage.success('已停用');
    loadRates();
  } catch (e) {
    ElMessage.error((e as Error).message || '停用失败');
  }
};

const onDeleteRate = async (row: Record<string, unknown>) => {
  try {
    await ElMessageBox.confirm(`确认删除费率 #${row.id}？`, '删除确认', { type: 'warning' });
  } catch {
    return;
  }
  try {
    await deleteWageRate(Number(row.id));
    ElMessage.success('删除成功');
    loadRates();
  } catch (e) {
    ElMessage.error((e as Error).message || '删除失败');
  }
};
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
.effective-result {
  background: var(--el-fill-color-light);
  border-radius: 4px;
  padding: 12px;
  font-size: 12px;
  max-height: 300px;
  overflow: auto;
  white-space: pre-wrap;
  margin: 0;
}
.toolbar {
  display: flex;
  gap: 8px;
  align-items: center;
}
</style>
