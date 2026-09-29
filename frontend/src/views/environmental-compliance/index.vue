<template>
  <div class="env-compliance-page">
    <el-card shadow="never">
      <el-tabs v-model="activeTab">
        <el-tab-pane label="排污许可证" name="permits">
          <div class="toolbar mb">
            <el-button type="primary" @click="openCreatePermit">新建许可证</el-button>
            <el-button plain @click="loadPermits">刷新</el-button>
          </div>
          <el-table v-loading="permitLoading" :data="permits" border>
            <el-table-column prop="id" label="ID" width="70" />
            <el-table-column prop="permit_no" label="许可证编号" min-width="150" />
            <el-table-column prop="permit_type" label="类型" width="110" />
            <el-table-column prop="permit_category" label="类别" width="110">
              <template #default="{ row }">{{ row.permit_category || '-' }}</template>
            </el-table-column>
            <el-table-column prop="issue_date" label="发证日期" width="110" />
            <el-table-column prop="expiry_date" label="到期日期" width="110" />
            <el-table-column prop="issuing_authority" label="发证机关" min-width="130" />
            <el-table-column prop="permitted_capacity" label="许可量" width="100">
              <template #default="{ row }">{{ row.permitted_capacity ?? '-' }}</template>
            </el-table-column>
            <el-table-column prop="status" label="状态" width="100">
              <template #default="{ row }">
                <el-tag :type="row.status === 'active' ? 'success' : 'info'">{{
                  row.status
                }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="110" fixed="right">
              <template #default="{ row }">
                <el-button
                  v-if="row.status === 'active'"
                  link
                  type="danger"
                  size="small"
                  @click="handleRevoke(row)"
                  >撤销</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>

        <el-tab-pane label="污染物监测" name="monitoring">
          <div class="toolbar mb">
            <el-button type="primary" @click="openCreateRecord">新建监测记录</el-button>
            <el-button plain @click="loadRecords">刷新</el-button>
            <el-button type="warning" plain :loading="scanLoading" @click="handleScanAlerts">
              扫描超标警报
            </el-button>
          </div>
          <el-table v-loading="recordLoading" :data="records" border>
            <el-table-column prop="id" label="ID" width="70" />
            <el-table-column prop="monitoring_type" label="监测类型" width="110" />
            <el-table-column prop="monitoring_point" label="监测点" min-width="120" />
            <el-table-column prop="pollutant_name" label="污染物" min-width="120" />
            <el-table-column prop="measured_value" label="实测值" width="100" />
            <el-table-column prop="unit" label="单位" width="80" />
            <el-table-column prop="limit_value" label="限值" width="100" />
            <el-table-column label="超标" width="100" align="center">
              <template #default="{ row }">
                <el-tag v-if="row.is_exceeding" type="danger">超标</el-tag>
                <el-tag v-else type="success">正常</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="monitoring_time" label="监测时间" min-width="160" />
          </el-table>
        </el-tab-pane>
      </el-tabs>
    </el-card>

    <el-dialog v-model="permitDialogVisible" title="新建排污许可证" width="560">
      <el-form :model="permitForm" label-width="110px">
        <el-form-item label="许可证编号" required>
          <el-input v-model="permitForm.permit_no" />
        </el-form-item>
        <el-form-item label="类型" required>
          <el-select v-model="permitForm.permit_type" class="w-full">
            <el-option label="废水" value="wastewater" />
            <el-option label="废气" value="exhaust_gas" />
            <el-option label="噪声" value="noise" />
            <el-option label="固废" value="solid_waste" />
          </el-select>
        </el-form-item>
        <el-form-item label="类别">
          <el-input v-model="permitForm.permit_category" placeholder="如：重点管理 / 简化管理" />
        </el-form-item>
        <el-form-item label="发证日期" required>
          <el-input v-model="permitForm.issue_date" placeholder="2026-01-01" />
        </el-form-item>
        <el-form-item label="到期日期" required>
          <el-input v-model="permitForm.expiry_date" placeholder="2030-12-31" />
        </el-form-item>
        <el-form-item label="发证机关" required>
          <el-input v-model="permitForm.issuing_authority" />
        </el-form-item>
        <el-form-item label="许可量">
          <el-input-number v-model="permitForm.permitted_capacity" :min="0" :precision="2" />
        </el-form-item>
        <el-form-item label="许可量单位">
          <el-input v-model="permitForm.capacity_unit" placeholder="如：t/a" />
        </el-form-item>
        <el-form-item label="备注"
          ><el-input v-model="permitForm.remarks" type="textarea" :rows="2"
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="permitDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="permitSaving" @click="handleSavePermit">保存</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="recordDialogVisible" title="新建监测记录" width="560">
      <el-form :model="recordForm" label-width="110px">
        <el-form-item label="监测类型" required>
          <el-select v-model="recordForm.monitoring_type" class="w-full">
            <el-option label="废水" value="wastewater" />
            <el-option label="废气" value="exhaust_gas" />
            <el-option label="噪声" value="noise" />
            <el-option label="土壤" value="soil" />
          </el-select>
        </el-form-item>
        <el-form-item label="监测点" required>
          <el-input v-model="recordForm.monitoring_point" />
        </el-form-item>
        <el-form-item label="污染物" required>
          <el-input v-model="recordForm.pollutant_name" />
        </el-form-item>
        <el-form-item label="实测值" required>
          <el-input-number v-model="recordForm.measured_value" :min="0" :precision="3" />
        </el-form-item>
        <el-form-item label="单位" required>
          <el-input v-model="recordForm.unit" placeholder="如 mg/L" />
        </el-form-item>
        <el-form-item label="限值" required>
          <el-input-number v-model="recordForm.limit_value" :min="0" :precision="3" />
        </el-form-item>
        <el-form-item label="监测时间" required>
          <el-input v-model="recordForm.monitoring_time" placeholder="2026-01-01T08:00:00Z" />
        </el-form-item>
        <el-form-item label="监测方法">
          <el-input v-model="recordForm.monitoring_method" />
        </el-form-item>
        <el-form-item label="设备 ID">
          <el-input-number v-model="recordForm.equipment_id" :min="1" />
        </el-form-item>
        <el-form-item label="备注"
          ><el-input v-model="recordForm.remarks" type="textarea" :rows="2"
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="recordDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="recordSaving" @click="handleSaveRecord">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, onMounted } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  getPollutionPermitList,
  createPollutionPermit,
  revokePollutionPermit,
  getPollutantMonitoringRecordList,
  createPollutantMonitoringRecord,
  scanExceedanceAlerts,
  type PollutantMonitoringRecord,
} from '@/api/compliance';

const activeTab = ref('permits');

// ===== 排污许可证 =====
const permits = ref<Array<Record<string, unknown>>>([]);
const permitLoading = ref(false);
const permitDialogVisible = ref(false);
const permitSaving = ref(false);
const permitForm = reactive({
  permit_no: '',
  permit_type: 'wastewater',
  permit_category: '',
  issue_date: '',
  expiry_date: '',
  issuing_authority: '',
  permitted_capacity: undefined as number | undefined,
  capacity_unit: '',
  remarks: '',
});

async function loadPermits() {
  permitLoading.value = true;
  try {
    const res = await getPollutionPermitList({ page: 1, page_size: 50 });
    permits.value = (res.data?.items ?? []) as Array<Record<string, unknown>>;
  } finally {
    permitLoading.value = false;
  }
}

const openCreatePermit = () => {
  Object.assign(permitForm, {
    permit_no: '',
    permit_type: 'wastewater',
    permit_category: '',
    issue_date: '',
    expiry_date: '',
    issuing_authority: '',
    permitted_capacity: undefined,
    capacity_unit: '',
    remarks: '',
  });
  permitDialogVisible.value = true;
};

const handleSavePermit = async () => {
  if (
    !permitForm.permit_no ||
    !permitForm.issue_date ||
    !permitForm.expiry_date ||
    !permitForm.issuing_authority
  ) {
    ElMessage.warning('请填写编号/发证日期/到期日期/发证机关');
    return;
  }
  permitSaving.value = true;
  try {
    await createPollutionPermit({
      permit_no: permitForm.permit_no,
      permit_type: permitForm.permit_type,
      permit_category: permitForm.permit_category || undefined,
      issue_date: permitForm.issue_date,
      expiry_date: permitForm.expiry_date,
      issuing_authority: permitForm.issuing_authority,
      permitted_capacity: permitForm.permitted_capacity ?? undefined,
      capacity_unit: permitForm.capacity_unit || undefined,
      remarks: permitForm.remarks || undefined,
    });
    ElMessage.success('许可证已创建');
    permitDialogVisible.value = false;
    await loadPermits();
  } finally {
    permitSaving.value = false;
  }
};

const handleRevoke = async (row: Record<string, unknown>) => {
  try {
    await ElMessageBox.confirm(`确认撤销许可证 ${row.permit_no}？`, '确认', { type: 'warning' });
  } catch {
    return;
  }
  try {
    await revokePollutionPermit(row.id as number);
    ElMessage.success('已撤销');
    await loadPermits();
  } catch (e) {
    ElMessage.error((e as Error).message || '撤销失败');
  }
};

// ===== 污染物监测 =====
const records = ref<PollutantMonitoringRecord[]>([]);
const recordLoading = ref(false);
const recordDialogVisible = ref(false);
const recordSaving = ref(false);
const scanLoading = ref(false);
const recordForm = reactive({
  monitoring_type: 'wastewater',
  monitoring_point: '',
  pollutant_name: '',
  measured_value: 0,
  unit: 'mg/L',
  limit_value: 0,
  monitoring_time: '',
  monitoring_method: '',
  equipment_id: undefined as number | undefined,
  remarks: '',
});

async function loadRecords() {
  recordLoading.value = true;
  try {
    const res = await getPollutantMonitoringRecordList({ page: 1, page_size: 50 });
    records.value = res.data?.items ?? [];
  } finally {
    recordLoading.value = false;
  }
}

const openCreateRecord = () => {
  Object.assign(recordForm, {
    monitoring_type: 'wastewater',
    monitoring_point: '',
    pollutant_name: '',
    measured_value: 0,
    unit: 'mg/L',
    limit_value: 0,
    monitoring_time: '',
    monitoring_method: '',
    equipment_id: undefined,
    remarks: '',
  });
  recordDialogVisible.value = true;
};

const handleSaveRecord = async () => {
  if (!recordForm.monitoring_point || !recordForm.pollutant_name || !recordForm.monitoring_time) {
    ElMessage.warning('请填写监测点/污染物/监测时间');
    return;
  }
  recordSaving.value = true;
  try {
    await createPollutantMonitoringRecord({
      monitoring_type: recordForm.monitoring_type,
      monitoring_point: recordForm.monitoring_point,
      pollutant_name: recordForm.pollutant_name,
      measured_value: recordForm.measured_value,
      unit: recordForm.unit,
      limit_value: recordForm.limit_value,
      monitoring_time: recordForm.monitoring_time,
      monitoring_method: recordForm.monitoring_method || undefined,
      equipment_id: recordForm.equipment_id ?? undefined,
      remarks: recordForm.remarks || undefined,
    });
    ElMessage.success('监测记录已创建');
    recordDialogVisible.value = false;
    await loadRecords();
  } finally {
    recordSaving.value = false;
  }
};

const handleScanAlerts = async () => {
  scanLoading.value = true;
  try {
    const res = await scanExceedanceAlerts();
    const d = res.data as { total?: number; alerts?: unknown[] } | undefined;
    const total = d?.total ?? (Array.isArray(d) ? d.length : 0);
    ElMessage.warning(total > 0 ? `发现 ${total} 条超标警报，详见后端返回` : '无超标警报');
    await loadRecords();
  } catch (e) {
    ElMessage.error((e as Error).message || '扫描失败');
  } finally {
    scanLoading.value = false;
  }
};

onMounted(() => {
  loadPermits();
  loadRecords();
});
</script>

<style scoped>
.mb {
  margin-bottom: 12px;
}
.toolbar {
  display: flex;
  gap: 8px;
  align-items: center;
}
.w-full {
  width: 100%;
}
</style>
