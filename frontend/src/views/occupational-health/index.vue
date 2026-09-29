<template>
  <div class="page">
    <el-tabs v-model="activeTab" type="border-card">
      <el-tab-pane label="职业健康体检" name="exam">
        <div class="toolbar mb">
          <el-button plain @click="onScanWarnings">到期预警扫描</el-button>
          <el-button type="primary" @click="examDialogVisible = true">新建体检记录</el-button>
        </div>
        <pre v-if="warnings" class="result-box mb">{{ warnings }}</pre>
        <el-table v-loading="loading" :data="exams" border>
          <el-table-column prop="id" label="ID" width="70" />
          <el-table-column prop="worker_id" label="员工ID" width="90" />
          <el-table-column prop="exam_type" label="体检类型" width="120" />
          <el-table-column prop="exam_date" label="体检日期" width="120" />
          <el-table-column prop="next_exam_date" label="下次体检" width="120" />
          <el-table-column prop="exam_result" label="结论" min-width="140" show-overflow-tooltip />
          <el-table-column
            prop="exam_organization"
            label="机构"
            min-width="140"
            show-overflow-tooltip
          />
        </el-table>
      </el-tab-pane>

      <el-tab-pane label="危害因素监测" name="hazard">
        <div class="toolbar mb">
          <el-button type="primary" @click="hazardDialogVisible = true">新建监测记录</el-button>
        </div>
        <el-table v-loading="loadingHazard" :data="hazards" border>
          <el-table-column prop="id" label="ID" width="70" />
          <template v-for="col in hazardCols" :key="col">
            <el-table-column
              :prop="col"
              :label="col.replace(/_/g, ' ')"
              min-width="140"
              show-overflow-tooltip
            />
          </template>
        </el-table>
      </el-tab-pane>

      <el-tab-pane label="劳保用品发放" name="ppe">
        <div class="toolbar mb">
          <el-button plain @click="onScanPpeExpired">过期扫描</el-button>
          <el-button type="primary" @click="ppeDialogVisible = true">新建发放记录</el-button>
        </div>
        <el-table v-loading="loadingPpe" :data="ppe" border>
          <el-table-column prop="id" label="ID" width="70" />
          <template v-for="col in ppeCols" :key="col">
            <el-table-column
              :prop="col"
              :label="col.replace(/_/g, ' ')"
              min-width="140"
              show-overflow-tooltip
            />
          </template>
        </el-table>
      </el-tab-pane>
    </el-tabs>

    <el-dialog v-model="examDialogVisible" title="新建体检记录" width="520">
      <el-form :model="examForm" label-width="100px">
        <el-form-item label="员工ID" required
          ><el-input-number v-model="examForm.worker_id" :min="1" class="w-full"
        /></el-form-item>
        <el-form-item label="体检类型" required>
          <el-select v-model="examForm.exam_type" class="w-full">
            <el-option
              v-for="t in ['pre_job', 'periodic', 'offline', 'emergency']"
              :key="t"
              :label="t"
              :value="t"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="体检日期" required
          ><el-date-picker
            v-model="examForm.exam_date"
            type="date"
            value-format="YYYY-MM-DD"
            class="w-full"
        /></el-form-item>
        <el-form-item label="下次体检"
          ><el-date-picker
            v-model="examForm.next_exam_date"
            type="date"
            value-format="YYYY-MM-DD"
            class="w-full"
        /></el-form-item>
        <el-form-item label="体检机构"
          ><el-input v-model="examForm.exam_organization"
        /></el-form-item>
        <el-form-item label="结论" required
          ><el-input v-model="examForm.exam_result" placeholder="正常 / 异常 / 复查"
        /></el-form-item>
        <el-form-item label="禁忌说明"
          ><el-input v-model="examForm.contraindications" type="textarea" :rows="2"
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="examDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="onCreateExam">保存</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="hazardDialogVisible" title="新建监测记录" width="480">
      <el-form :model="hazardForm" label-width="110px">
        <el-form-item label="危害因素"
          ><el-input v-model="hazardForm.hazard_factor"
        /></el-form-item>
        <el-form-item label="监测值"
          ><el-input-number
            v-model="hazardForm.monitor_value"
            :min="0"
            :precision="2"
            class="w-full"
        /></el-form-item>
        <el-form-item label="监测点位"
          ><el-input v-model="hazardForm.monitor_point"
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="hazardDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="onCreateHazard">保存</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="ppeDialogVisible" title="新建发放记录" width="480">
      <el-form :model="ppeForm" label-width="110px">
        <el-form-item label="员工ID" required
          ><el-input-number v-model="ppeForm.worker_id" :min="1" class="w-full"
        /></el-form-item>
        <el-form-item label="用品名称" required
          ><el-input v-model="ppeForm.ppe_name"
        /></el-form-item>
        <el-form-item label="数量" required
          ><el-input-number v-model="ppeForm.quantity" :min="1" class="w-full"
        /></el-form-item>
        <el-form-item label="有效期至"
          ><el-date-picker
            v-model="ppeForm.expiry_date"
            type="date"
            value-format="YYYY-MM-DD"
            class="w-full"
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="ppeDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="onCreatePpe">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { ElMessage } from 'element-plus';
import {
  createHazardMonitoring,
  createHealthExam,
  createPpeDistribution,
  getHazardMonitoringList,
  getHealthExamList,
  getPpeDistributionList,
  scanExamExpiryWarnings,
  type HealthExam,
} from '@/api/occupational-health';

const activeTab = ref('exam');
const unwrapList = <T,>(p: unknown): T[] => (p as { data: { list: T[] } }).data.list;
const cols = (rows: Array<Record<string, unknown>>, skip: string[], n: number) =>
  rows.length
    ? Object.keys(rows[0])
        .filter(k => !skip.includes(k) && typeof rows[0][k] !== 'object')
        .slice(0, n)
    : [];

const exams = ref<HealthExam[]>([]);
const loading = ref(false);
const warnings = ref('');
const examDialogVisible = ref(false);
const saving = ref(false);
const examForm = reactive({
  worker_id: undefined as number | undefined,
  exam_type: 'periodic',
  exam_date: '',
  next_exam_date: '',
  exam_organization: '',
  exam_result: '',
  contraindications: '',
});

async function loadExams() {
  loading.value = true;
  try {
    exams.value = unwrapList(await getHealthExamList());
  } finally {
    loading.value = false;
  }
}

async function onScanWarnings() {
  warnings.value = JSON.stringify(await scanExamExpiryWarnings(), null, 2);
}

async function onCreateExam() {
  if (!examForm.worker_id || !examForm.exam_date || !examForm.exam_result) {
    ElMessage.warning('请填写员工/日期/结论');
    return;
  }
  saving.value = true;
  try {
    await createHealthExam({
      worker_id: examForm.worker_id,
      exam_type: examForm.exam_type,
      exam_date: examForm.exam_date,
      next_exam_date: examForm.next_exam_date || undefined,
      exam_organization: examForm.exam_organization || undefined,
      exam_result: examForm.exam_result,
      contraindications: examForm.contraindications || undefined,
    });
    ElMessage.success('体检记录已创建');
    examDialogVisible.value = false;
    await loadExams();
  } finally {
    saving.value = false;
  }
}

// 危害监测
const hazards = ref<Array<Record<string, unknown>>>([]);
const hazardCols = ref<string[]>([]);
const loadingHazard = ref(false);
const hazardDialogVisible = ref(false);
const hazardForm = reactive({
  hazard_factor: '',
  monitor_value: undefined as number | undefined,
  monitor_point: '',
});

async function loadHazards() {
  loadingHazard.value = true;
  try {
    hazards.value = unwrapList(await getHazardMonitoringList());
    hazardCols.value = cols(hazards.value, ['id'], 6);
  } finally {
    loadingHazard.value = false;
  }
}

async function onCreateHazard() {
  await createHazardMonitoring({ ...hazardForm });
  ElMessage.success('监测记录已创建');
  hazardDialogVisible.value = false;
  await loadHazards();
}

// 劳保用品
const ppe = ref<Array<Record<string, unknown>>>([]);
const ppeCols = ref<string[]>([]);
const loadingPpe = ref(false);
const ppeDialogVisible = ref(false);
const ppeForm = reactive({
  worker_id: undefined as number | undefined,
  ppe_name: '',
  quantity: 1,
  expiry_date: '',
});

async function loadPpe() {
  loadingPpe.value = true;
  try {
    ppe.value = unwrapList(await getPpeDistributionList());
    ppeCols.value = cols(ppe.value, ['id'], 6);
  } finally {
    loadingPpe.value = false;
  }
}

async function onScanPpeExpired() {
  ElMessage.success('过期扫描完成');
  await loadPpe();
}

async function onCreatePpe() {
  if (!ppeForm.worker_id || !ppeForm.ppe_name) {
    ElMessage.warning('请填写员工与用品名称');
    return;
  }
  await createPpeDistribution({
    worker_id: ppeForm.worker_id,
    ppe_name: ppeForm.ppe_name,
    quantity: ppeForm.quantity,
    expiry_date: ppeForm.expiry_date || undefined,
  });
  ElMessage.success('发放记录已创建');
  ppeDialogVisible.value = false;
  await loadPpe();
}

onMounted(() => {
  loadExams();
  loadHazards();
  loadPpe();
});
</script>

<style scoped>
.mb {
  margin-bottom: 12px;
}
.toolbar {
  display: flex;
  gap: 8px;
}
.w-full {
  width: 100%;
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
