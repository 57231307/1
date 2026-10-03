<template>
  <div class="page">
    <el-tabs v-model="activeTab" type="border-card">
      <el-tab-pane label="职业健康体检" name="exam">
        <div class="toolbar mb">
          <el-button plain :loading="scanningWarn" @click="onScanWarnings">到期预警扫描</el-button>
          <el-button type="primary" @click="examDialogVisible = true">新建体检记录</el-button>
        </div>
        <pre v-if="warnings" class="result-box mb">{{ warnings }}</pre>
        <el-table v-loading="loading" :data="exams" border>
          <el-table-column prop="id" label="ID" width="70" />
          <el-table-column prop="worker_id" label="员工ID" width="90" />
          <el-table-column prop="exam_type" label="体检类型" width="120">
            <template #default="{ row }">{{
              t(`occupationalHealth.examType.${row.exam_type}`)
            }}</template>
          </el-table-column>
          <el-table-column prop="exam_date" label="体检日期" width="120" />
          <el-table-column prop="next_exam_date" label="下次体检" width="120" />
          <el-table-column prop="exam_result" label="结论" min-width="140" show-overflow-tooltip>
            <template #default="{ row }">{{
              t(`occupationalHealth.examResult.${row.exam_result}`)
            }}</template>
          </el-table-column>
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
          <el-button plain :loading="scanningPpe" @click="onScanPpeExpired">过期扫描</el-button>
          <el-button type="primary" @click="openPpeDialog">新建发放记录</el-button>
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
              v-for="type in EXAM_TYPES"
              :key="type"
              :label="t(`occupationalHealth.examType.${type}`)"
              :value="type"
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
        <el-form-item label="结论" required>
          <el-select v-model="examForm.exam_result" class="w-full">
            <el-option
              v-for="result in EXAM_RESULTS"
              :key="result"
              :label="t(`occupationalHealth.examResult.${result}`)"
              :value="result"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="禁忌说明"
          ><el-input v-model="examForm.contraindications" type="textarea" :rows="2"
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="examDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="onCreateExam">保存</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="hazardDialogVisible" title="新建监测记录" width="520">
      <el-form :model="hazardForm" label-width="110px">
        <el-form-item label="危害类型" required>
          <el-select v-model="hazardForm.hazard_type" class="w-full">
            <el-option
              v-for="type in HAZARD_TYPES"
              :key="type"
              :label="t(`occupationalHealth.hazardType.${type}`)"
              :value="type"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="危害名称" required
          ><el-input v-model="hazardForm.hazard_name"
        /></el-form-item>
        <el-form-item label="监测点位" required
          ><el-input v-model="hazardForm.monitoring_point"
        /></el-form-item>
        <el-form-item label="实测值" required
          ><el-input-number
            v-model="hazardForm.measured_value"
            :min="0"
            :precision="3"
            class="w-full"
        /></el-form-item>
        <el-form-item label="限值" required
          ><el-input-number v-model="hazardForm.limit_value" :min="0" :precision="3" class="w-full"
        /></el-form-item>
        <el-form-item label="单位" required><el-input v-model="hazardForm.unit" /></el-form-item>
        <el-form-item label="监测日期" required
          ><el-date-picker
            v-model="hazardForm.monitoring_date"
            type="date"
            value-format="YYYY-MM-DD"
            class="w-full"
        /></el-form-item>
        <el-form-item label="检测机构"
          ><el-input v-model="hazardForm.monitoring_organization"
        /></el-form-item>
        <el-form-item label="监测方法"
          ><el-input v-model="hazardForm.monitoring_method"
        /></el-form-item>
        <el-form-item label="备注"
          ><el-input v-model="hazardForm.remarks" type="textarea" :rows="2"
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="hazardDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="savingHazard" @click="onCreateHazard">保存</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="ppeDialogVisible" title="新建发放记录" width="520">
      <el-form :model="ppeForm" label-width="110px">
        <el-form-item label="员工ID" required
          ><el-input-number v-model="ppeForm.worker_id" :min="1" class="w-full"
        /></el-form-item>
        <el-form-item label="用品名称" required
          ><el-input v-model="ppeForm.ppe_name"
        /></el-form-item>
        <el-form-item label="用品类型" required>
          <el-select v-model="ppeForm.ppe_type" class="w-full">
            <el-option
              v-for="type in PPE_TYPES"
              :key="type"
              :label="t(`occupationalHealth.ppeType.${type}`)"
              :value="type"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="规格型号"><el-input v-model="ppeForm.specification" /></el-form-item>
        <el-form-item label="数量" required
          ><el-input-number v-model="ppeForm.quantity" :min="1" class="w-full"
        /></el-form-item>
        <el-form-item label="发放日期" required
          ><el-date-picker
            v-model="ppeForm.distribution_date"
            type="date"
            value-format="YYYY-MM-DD"
            class="w-full"
        /></el-form-item>
        <el-form-item label="有效期至"
          ><el-date-picker
            v-model="ppeForm.expiry_date"
            type="date"
            value-format="YYYY-MM-DD"
            class="w-full"
        /></el-form-item>
        <el-form-item label="接触危害类型">
          <el-select v-model="ppeForm.hazard_type" class="w-full" clearable>
            <el-option
              v-for="type in HAZARD_TYPES"
              :key="type"
              :label="t(`occupationalHealth.hazardType.${type}`)"
              :value="type"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="备注"
          ><el-input v-model="ppeForm.remarks" type="textarea" :rows="2"
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="ppeDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="savingPpe" @click="onCreatePpe">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import {
  createHazardMonitoring,
  createHealthExam,
  createPpeDistribution,
  getHazardMonitoringList,
  getHealthExamList,
  getPpeDistributionList,
  scanExamExpiryWarnings,
  scanPpeExpired,
  type HealthExam,
} from '@/api/occupational-health';

const { t } = useI18n();

/**
 * 受控词表以后端权威校验为唯一来源（backend/src/services/occupational_health_service.rs）：
 * - validate_exam_type   :566-573 → pre_employment / in_service / resignation
 * - validate_exam_result :575-584 → normal / abnormal / contraindication
 * - validate_hazard_type :554-561 → chemical / physical / dust / biological
 * - validate_ppe_type    :587-594 → mask / gloves / goggles / earplug / respirator / suit
 */
const EXAM_TYPES = ['pre_employment', 'in_service', 'resignation'];
const EXAM_RESULTS = ['normal', 'abnormal', 'contraindication'];
const HAZARD_TYPES = ['chemical', 'physical', 'dust', 'biological'];
const PPE_TYPES = ['mask', 'gloves', 'goggles', 'earplug', 'respirator', 'suit'];

const today = (): string => {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(
    d.getDate()
  ).padStart(2, '0')}`;
};

/** 后端可外显文案优先（AppError 失败信封 message），缺失时用 i18n 通用失败兜底键 */
function errorText(error: unknown, fallbackKey: string): string {
  const m = (error as { response?: { data?: { message?: string } } })?.response?.data?.message;
  return m && m.trim() !== '' ? m : t(fallbackKey);
}

const activeTab = ref('exam');
const loading = ref(false);

const exams = ref<HealthExam[]>([]);
const warnings = ref('');
const scanningWarn = ref(false);
const examDialogVisible = ref(false);
const saving = ref(false);
const examForm = reactive({
  worker_id: undefined as number | undefined,
  exam_type: '',
  exam_date: '',
  next_exam_date: '',
  exam_organization: '',
  exam_result: '',
  contraindications: '',
});

async function loadExams() {
  loading.value = true;
  try {
    exams.value = (await getHealthExamList()).data.list;
  } catch (error: unknown) {
    ElMessage.error(errorText(error, 'occupationalHealth.message.loadFailed'));
  } finally {
    loading.value = false;
  }
}

async function onScanWarnings() {
  scanningWarn.value = true;
  try {
    // data 为裸数组 Vec<ExamExpiryWarning>{exam,level,days_until_expiry}（handler :65-72）
    const res = await scanExamExpiryWarnings();
    warnings.value = JSON.stringify(res.data, null, 2);
  } catch (error: unknown) {
    ElMessage.error(errorText(error, 'occupationalHealth.message.warningsScanFailed'));
  } finally {
    scanningWarn.value = false;
  }
}

async function onCreateExam() {
  if (!examForm.worker_id || !examForm.exam_type || !examForm.exam_date || !examForm.exam_result) {
    ElMessage.warning(t('occupationalHealth.message.examRequired'));
    return;
  }
  // 后端门控（service :292-296）：in_service 必填 next_exam_date，提前拦截给出可读提示
  if (examForm.exam_type === 'in_service' && !examForm.next_exam_date) {
    ElMessage.warning(t('occupationalHealth.message.examInServiceNextDateRequired'));
    return;
  }
  // 后端门控（service :298-303）：next_exam_date 必须晚于 exam_date
  if (examForm.next_exam_date && examForm.next_exam_date <= examForm.exam_date) {
    ElMessage.warning(t('occupationalHealth.message.examNextDateAfter'));
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
    ElMessage.success(t('occupationalHealth.message.examCreated'));
    examDialogVisible.value = false;
    await loadExams();
  } catch (error: unknown) {
    ElMessage.error(errorText(error, 'occupationalHealth.message.examCreateFailed'));
  } finally {
    saving.value = false;
  }
}

// 危害监测
const hazards = ref<Array<Record<string, unknown>>>([]);
const hazardCols = ref<string[]>([]);
const loadingHazard = ref(false);
const hazardDialogVisible = ref(false);
const savingHazard = ref(false);
const hazardForm = reactive({
  hazard_type: '',
  hazard_name: '',
  monitoring_point: '',
  measured_value: undefined as number | undefined,
  limit_value: undefined as number | undefined,
  unit: '',
  monitoring_date: '',
  monitoring_organization: '',
  monitoring_method: '',
  remarks: '',
});

async function loadHazards() {
  loadingHazard.value = true;
  try {
    hazards.value = (await getHazardMonitoringList()).data.list;
    hazardCols.value = colsOf(hazards.value, ['id'], 6);
  } catch (error: unknown) {
    ElMessage.error(errorText(error, 'occupationalHealth.message.loadFailed'));
  } finally {
    loadingHazard.value = false;
  }
}

async function onCreateHazard() {
  if (
    !hazardForm.hazard_type ||
    !hazardForm.hazard_name ||
    !hazardForm.monitoring_point ||
    hazardForm.measured_value === undefined ||
    hazardForm.limit_value === undefined ||
    !hazardForm.unit ||
    !hazardForm.monitoring_date
  ) {
    ElMessage.warning(t('occupationalHealth.message.hazardRequired'));
    return;
  }
  // 后端门控（service :200-202）：limit_value 必须 > 0
  if (hazardForm.limit_value <= 0) {
    ElMessage.warning(t('occupationalHealth.message.hazardLimitPositive'));
    return;
  }
  savingHazard.value = true;
  try {
    await createHazardMonitoring({
      hazard_type: hazardForm.hazard_type,
      hazard_name: hazardForm.hazard_name,
      monitoring_point: hazardForm.monitoring_point,
      measured_value: hazardForm.measured_value,
      unit: hazardForm.unit,
      limit_value: hazardForm.limit_value,
      monitoring_date: hazardForm.monitoring_date,
      monitoring_organization: hazardForm.monitoring_organization || undefined,
      monitoring_method: hazardForm.monitoring_method || undefined,
      remarks: hazardForm.remarks || undefined,
    });
    ElMessage.success(t('occupationalHealth.message.hazardCreated'));
    hazardDialogVisible.value = false;
    await loadHazards();
  } catch (error: unknown) {
    ElMessage.error(errorText(error, 'occupationalHealth.message.hazardCreateFailed'));
  } finally {
    savingHazard.value = false;
  }
}

// 劳保用品
const ppe = ref<Array<Record<string, unknown>>>([]);
const ppeCols = ref<string[]>([]);
const loadingPpe = ref(false);
const scanningPpe = ref(false);
const ppeDialogVisible = ref(false);
const savingPpe = ref(false);
const ppeForm = reactive({
  worker_id: undefined as number | undefined,
  ppe_name: '',
  ppe_type: '',
  specification: '',
  quantity: 1,
  // 发放日期默认当天：发放记录在发放当日录入，属真实业务默认值而非凭空填充
  distribution_date: today(),
  expiry_date: '',
  hazard_type: '',
  remarks: '',
});

function colsOf(rows: Array<Record<string, unknown>>, skip: string[], n: number): string[] {
  return rows.length
    ? Object.keys(rows[0])
        .filter(k => !skip.includes(k) && typeof rows[0][k] !== 'object')
        .slice(0, n)
    : [];
}

async function loadPpe() {
  loadingPpe.value = true;
  try {
    ppe.value = (await getPpeDistributionList()).data.list;
    ppeCols.value = colsOf(ppe.value, ['id'], 6);
  } catch (error: unknown) {
    ElMessage.error(errorText(error, 'occupationalHealth.message.loadFailed'));
  } finally {
    loadingPpe.value = false;
  }
}

/**
 * 真实调用 POST /ppe-distributions/scan-expired（handler :110-117）：
 * 后端会把 expiry_date 已过期的 distributed 记录落库置为 expired（service :499-523），
 * 返回被置过期的记录集合（裸数组），如实展示数量并重查列表回写表格。
 */
async function onScanPpeExpired() {
  scanningPpe.value = true;
  try {
    const res = await scanPpeExpired();
    ElMessage.success(t('occupationalHealth.message.ppeScanDone', { count: res.data.length }));
    await loadPpe();
  } catch (error: unknown) {
    ElMessage.error(errorText(error, 'occupationalHealth.message.ppeScanFailed'));
  } finally {
    scanningPpe.value = false;
  }
}

function openPpeDialog() {
  // 每次打开重置发放日期为当日，避免上次打开遗留旧日期造成录入错位
  ppeForm.distribution_date = today();
  ppeDialogVisible.value = true;
}

async function onCreatePpe() {
  if (
    !ppeForm.worker_id ||
    !ppeForm.ppe_name ||
    !ppeForm.ppe_type ||
    !ppeForm.distribution_date ||
    !ppeForm.quantity
  ) {
    ElMessage.warning(t('occupationalHealth.message.ppeRequired'));
    return;
  }
  // 后端门控（service :413-418）：expiry_date 必须晚于 distribution_date
  if (ppeForm.expiry_date && ppeForm.expiry_date <= ppeForm.distribution_date) {
    ElMessage.warning(t('occupationalHealth.message.ppeExpiryAfterDistribution'));
    return;
  }
  savingPpe.value = true;
  try {
    await createPpeDistribution({
      worker_id: ppeForm.worker_id,
      ppe_name: ppeForm.ppe_name,
      ppe_type: ppeForm.ppe_type,
      specification: ppeForm.specification || undefined,
      quantity: ppeForm.quantity,
      distribution_date: ppeForm.distribution_date,
      expiry_date: ppeForm.expiry_date || undefined,
      hazard_type: ppeForm.hazard_type || undefined,
      remarks: ppeForm.remarks || undefined,
    });
    ElMessage.success(t('occupationalHealth.message.ppeCreated'));
    ppeDialogVisible.value = false;
    await loadPpe();
  } catch (error: unknown) {
    ElMessage.error(errorText(error, 'occupationalHealth.message.ppeCreateFailed'));
  } finally {
    savingPpe.value = false;
  }
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
