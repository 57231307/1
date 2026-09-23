<template>
  <div class="page">
    <el-card shadow="never">
      <template #header>
        <div class="card-header">
          <span>验布管理</span>
          <el-button type="primary" @click="openCreate">新建验布单</el-button>
        </div>
      </template>
      <el-table v-loading="loading" :data="inspections" border>
        <el-table-column prop="id" label="ID" width="70" />
        <el-table-column prop="inspection_no" label="验布单号" width="150" />
        <el-table-column label="状态" width="110">
          <template #default="{ row }">
            <el-tag :type="statusTag(row.status)">{{
              INSPECTION_STATUS_LABEL[row.status] ?? row.status
            }}</el-tag>
          </template>
        </el-table-column>
        <template v-for="col in extraCols" :key="col">
          <el-table-column
            :prop="col"
            :label="col.replace(/_/g, ' ')"
            min-width="130"
            show-overflow-tooltip
          />
        </template>
        <el-table-column label="操作" width="300" fixed="right">
          <template #default="{ row }">
            <el-button size="small" @click="openDetail(row)">详情</el-button>
            <el-button
              v-if="row.status === 'pending'"
              size="small"
              type="primary"
              plain
              @click="openEdit(row)"
              >编辑</el-button
            >
            <el-button
              v-if="row.status === 'inspecting'"
              size="small"
              type="warning"
              plain
              @click="openDefect(row)"
              >登记疵点</el-button
            >
            <el-button
              v-if="row.status !== 'closed'"
              size="small"
              type="primary"
              @click="onGrade(row)"
              >定级</el-button
            >
            <el-button
              v-if="row.status === 'graded'"
              size="small"
              type="success"
              @click="openRoll(row)"
              >打卷</el-button
            >
            <el-button
              v-if="row.status === 'graded'"
              size="small"
              type="success"
              @click="onClose(row)"
              >关闭</el-button
            >
            <el-button
              v-if="row.status === 'pending'"
              size="small"
              type="danger"
              plain
              @click="onDelete(row)"
              >删除</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="dialogVisible" :title="editingId ? '编辑验布单' : '新建验布单'" width="480">
      <el-form :model="form" label-width="100px">
        <!-- 字段与后端 CreateInspectionRequest 对齐：验布日期必填（缺它后端直接 422）；
             缸号/色号/日期在编辑态不可改（UpdateInspectionRequest 不含这三项） -->
        <el-form-item label="验布日期" required>
          <el-date-picker
            v-model="form.inspection_date"
            type="date"
            value-format="YYYY-MM-DD"
            :disabled="!!editingId"
            class="w-full"
          />
        </el-form-item>
        <el-form-item label="缸号">
          <el-input v-model="form.dye_lot_no" :disabled="!!editingId" />
        </el-form-item>
        <el-form-item label="色号">
          <el-input v-model="form.color_no" :disabled="!!editingId" />
        </el-form-item>
        <el-form-item label="验布员">
          <el-input v-model="form.inspector_name" />
        </el-form-item>
        <el-form-item label="机台号">
          <el-input v-model="form.machine_no" />
        </el-form-item>
        <el-form-item label="评分制式">
          <el-select v-model="form.scoring_system" class="w-full">
            <el-option
              v-for="opt in FABRIC_SCORING_OPTIONS"
              :key="opt"
              :label="FABRIC_SCORING_LABEL[opt]"
              :value="opt"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="门幅(英寸)">
          <el-input-number
            v-model="form.fabric_width_inches"
            :min="0"
            :precision="2"
            class="w-full"
          />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="form.remarks" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="onSave">{{
          editingId ? '更新' : '保存'
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="detailVisible" title="验布单详情" width="600">
      <el-descriptions v-if="detailRow" :column="2" border>
        <el-descriptions-item label="验布单号">{{ detailRow.inspection_no }}</el-descriptions-item>
        <el-descriptions-item label="状态">{{ detailRow.status }}</el-descriptions-item>
        <el-descriptions-item label="缸号">{{ detailRow.dye_lot_no || '-' }}</el-descriptions-item>
        <el-descriptions-item label="色号">{{ detailRow.color_no || '-' }}</el-descriptions-item>
        <el-descriptions-item label="验布日期">{{
          detailRow.inspection_date || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="机台">{{ detailRow.machine_no || '-' }}</el-descriptions-item>
        <el-descriptions-item label="评分制式">{{
          FABRIC_SCORING_LABEL[detailRow.scoring_system as FabricScoringValue] ??
          detailRow.scoring_system ??
          '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="门幅(英寸)">{{
          detailRow.fabric_width_inches ?? '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="总扣分">{{
          detailRow.total_defect_points ?? '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="每百平方码分数">{{
          detailRow.points_per_100_sq_yards ?? '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="检验码数">{{
          detailRow.inspected_yards ?? '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="合格率 %">{{
          detailRow.qualification_rate ?? '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="等级">{{ detailRow.grade || '-' }}</el-descriptions-item>
        <el-descriptions-item label="备注" :span="2">{{
          detailRow.remarks || '-'
        }}</el-descriptions-item>
      </el-descriptions>

      <h4 class="section-title">疵点明细</h4>
      <el-table
        v-loading="defectListLoading"
        :data="defectList"
        border
        size="small"
        max-height="240"
      >
        <el-table-column prop="id" label="ID" width="60" />
        <el-table-column prop="defect_type" label="类型" width="110" />
        <el-table-column prop="position_yards" label="位置(码)" width="90" />
        <el-table-column prop="defect_length_inches" label="长度(英寸)" width="100" />
        <el-table-column prop="direction" label="方向" width="70">
          <template #default="{ row }">{{ row.direction || '-' }}</template>
        </el-table-column>
        <el-table-column prop="description" label="描述" min-width="130" show-overflow-tooltip>
          <template #default="{ row }">{{ row.description || '-' }}</template>
        </el-table-column>
        <el-table-column label="操作" width="130" fixed="right">
          <template #default="{ row }">
            <el-button link size="small" @click="viewDefect(row)">查看</el-button>
            <el-button
              v-if="detailRow?.status === 'inspecting'"
              link
              type="danger"
              size="small"
              @click="removeDefect(row)"
              >删除</el-button
            >
          </template>
        </el-table-column>
      </el-table>
      <template #footer>
        <el-button @click="detailVisible = false">关闭</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="rollVisible" title="验布打卷入库" width="500">
      <el-form :model="rollForm" label-width="130px">
        <el-form-item label="仓库 ID" required>
          <el-input-number v-model="rollForm.warehouse_id" :min="1" class="w-full" />
        </el-form-item>
        <el-form-item label="卷长 m" required>
          <el-input-number
            v-model="rollForm.roll_length"
            :min="0.01"
            :precision="2"
            class="w-full"
          />
        </el-form-item>
        <el-form-item label="卷重 kg">
          <el-input-number v-model="rollForm.roll_weight" :min="0" :precision="2" class="w-full" />
        </el-form-item>
        <el-form-item label="幅宽 cm">
          <el-input-number v-model="rollForm.roll_width" :min="0" :precision="1" class="w-full" />
        </el-form-item>
        <el-form-item label="克重 g/m2">
          <el-input-number
            v-model="rollForm.roll_gram_weight"
            :min="0"
            :precision="1"
            class="w-full"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="rollVisible = false">取消</el-button>
        <el-button type="primary" :loading="rollSaving" @click="onRollSubmit">提交</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="defectVisible" title="疵点登记" width="500">
      <el-form :model="defectForm" label-width="130px">
        <el-form-item label="疵点类型" required>
          <el-select v-model="defectForm.defect_type" class="w-full">
            <el-option label="断经" value="broken_end" />
            <el-option label="断纬" value="broken_pick" />
            <el-option label="污渍" value="stain" />
            <el-option label="破洞" value="hole" />
            <el-option label="色档" value="color_bar" />
            <el-option label="其他" value="other" />
          </el-select>
        </el-form-item>
        <el-form-item label="位置(码)" required>
          <el-input-number
            v-model="defectForm.position_yards"
            :min="0"
            :precision="1"
            class="w-full"
          />
        </el-form-item>
        <el-form-item label="疵点长度(英寸)" required>
          <el-input-number
            v-model="defectForm.defect_length_inches"
            :min="0"
            :precision="1"
            class="w-full"
          />
        </el-form-item>
        <el-form-item label="方向">
          <el-select v-model="defectForm.direction" class="w-full" clearable>
            <el-option label="经向" value="warp" />
            <el-option label="纬向" value="weft" />
          </el-select>
        </el-form-item>
        <el-form-item label="破洞"><el-switch v-model="defectForm.is_hole" /></el-form-item>
        <el-form-item label="连续性疵点"
          ><el-switch v-model="defectForm.is_continuous"
        /></el-form-item>
        <el-form-item label="半幅疵点"
          ><el-switch v-model="defectForm.is_half_width"
        /></el-form-item>
        <el-form-item label="描述"
          ><el-input v-model="defectForm.description" type="textarea" :rows="2"
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="defectVisible = false">取消</el-button>
        <el-button type="primary" :loading="defectSaving" @click="onDefectSubmit">提交</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="gradeVisible" title="验布定级" width="440">
      <el-form :model="gradeForm" label-width="120px">
        <el-form-item label="检验码数" required>
          <el-input-number
            v-model="gradeForm.inspected_yards"
            :min="0.01"
            :precision="1"
            class="w-full"
          />
        </el-form-item>
        <el-form-item label="合格率 %">
          <el-input-number
            v-model="gradeForm.qualification_rate"
            :min="0"
            :max="100"
            :precision="2"
            class="w-full"
          />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="gradeForm.remarks" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="gradeVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="onGradeSubmit">提交</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue';
import {
  FABRIC_SCORING,
  FABRIC_SCORING_LABEL,
  FABRIC_SCORING_OPTIONS,
  type FabricScoringValue,
} from '@/constants/fabric-scoring';
import { logger } from '@/utils/logger';
import { ElMessage, ElMessageBox } from 'element-plus';
import { useI18n } from 'vue-i18n';
import {
  closeFabricInspection,
  createFabricInspection,
  deleteFabricInspection,
  getFabricInspectionList,
  gradeFabricInspection,
  getFabricInspectionDetail,
  updateFabricInspection,
  addFabricRoll,
  createFabricDefect,
  listFabricDefectsByInspection,
  getFabricDefect,
  deleteFabricDefect,
  INSPECTION_STATUS_LABEL,
  type FabricInspection,
} from '@/api/fabric-inspection';

const { t } = useI18n({ useScope: 'global' });
const inspections = ref<FabricInspection[]>([]);
const loading = ref(false);
const saving = ref(false);
const dialogVisible = ref(false);
const gradeVisible = ref(false);
const gradingId = ref<number | null>(null);

const form = reactive({
  inspection_date: '',
  dye_lot_no: '',
  color_no: '',
  inspector_name: '',
  machine_no: '',
  scoring_system: FABRIC_SCORING.fourPoint as string,
  fabric_width_inches: undefined as number | undefined,
  remarks: '',
});

/** 后端 inspection_date 是 NaiveDate（必填，缺即 422），界面按当天预填、用户可改 */
const todayIso = () => new Date().toISOString().split('T')[0];
const gradeForm = reactive({
  inspected_yards: undefined as number | undefined,
  qualification_rate: undefined as number | undefined,
  remarks: '',
});

const unwrapList = (p: unknown): FabricInspection[] =>
  (p as { data: { items: FabricInspection[] } }).data.items;

const statusTag = (s: string) =>
  ({ draft: 'info', inspecting: 'warning', graded: 'success', closed: 'info' })[s] ?? 'info';

const skipCols = new Set(['id', 'inspection_no', 'status', 'created_at', 'updated_at']);
const extraCols = computed(() => {
  const sample = inspections.value[0] ?? {};
  return Object.keys(sample)
    .filter(k => !skipCols.has(k) && typeof sample[k] !== 'object' && sample[k] !== null)
    .slice(0, 5);
});

async function load() {
  loading.value = true;
  try {
    inspections.value = unwrapList(await getFabricInspectionList());
  } finally {
    loading.value = false;
  }
}

function openCreate() {
  editingId.value = null;
  Object.assign(form, {
    inspection_date: todayIso(),
    dye_lot_no: '',
    color_no: '',
    inspector_name: '',
    machine_no: '',
    scoring_system: FABRIC_SCORING.fourPoint as string,
    fabric_width_inches: undefined,
    remarks: '',
  });
  dialogVisible.value = true;
}

async function onCreate() {
  saving.value = true;
  try {
    await createFabricInspection({
      inspection_date: form.inspection_date,
      dye_lot_no: form.dye_lot_no || undefined,
      color_no: form.color_no || undefined,
      inspector_name: form.inspector_name || undefined,
      machine_no: form.machine_no || undefined,
      scoring_system: form.scoring_system || undefined,
      fabric_width_inches: form.fabric_width_inches ?? undefined,
      remarks: form.remarks || undefined,
    });
    ElMessage.success('验布单已创建');
    dialogVisible.value = false;
    await load();
  } finally {
    saving.value = false;
  }
}

// ===== 编辑（pending 态，updateFabricInspection） =====
const editingId = ref<number | null>(null);

function openEdit(row: FabricInspection) {
  editingId.value = row.id;
  Object.assign(form, {
    inspection_date: (row.inspection_date as string) || todayIso(),
    dye_lot_no: (row.dye_lot_no as string) || '',
    color_no: (row.color_no as string) || '',
    inspector_name: (row.inspector_name as string) || '',
    machine_no: (row.machine_no as string) || '',
    scoring_system: (row.scoring_system as string) || FABRIC_SCORING.fourPoint,
    fabric_width_inches: (row.fabric_width_inches as number | undefined) ?? undefined,
    remarks: row.remarks || '',
  });
  dialogVisible.value = true;
}

async function onSave() {
  if (editingId.value) {
    saving.value = true;
    try {
      await updateFabricInspection(editingId.value, {
        inspector_name: form.inspector_name || undefined,
        machine_no: form.machine_no || undefined,
        scoring_system: form.scoring_system || undefined,
        fabric_width_inches: form.fabric_width_inches ?? undefined,
        remarks: form.remarks || undefined,
      });
      ElMessage.success('验布单已更新');
      dialogVisible.value = false;
      editingId.value = null;
      await load();
    } finally {
      saving.value = false;
    }
  } else {
    await onCreate();
  }
}

// ===== 详情回源（getFabricInspectionDetail）+ 疵点明细（list/get/deleteFabricDefect） =====
const detailVisible = ref(false);
const detailRow = ref<Record<string, unknown> | null>(null);
const defectList = ref<Array<Record<string, unknown>>>([]);
const defectListLoading = ref(false);

async function openDetail(row: FabricInspection) {
  detailRow.value = row as unknown as Record<string, unknown>;
  detailVisible.value = true;
  try {
    // 回源接口未声明返回类型，调用处按 ApiResponse 结构标注
    const res = (await getFabricInspectionDetail(row.id)) as { data?: FabricInspection };
    if (res.data) {
      detailRow.value = res.data;
      detailVisible.value = true;
    }
  } catch (error) {
    logger.error(t('message.loadFabricInspectionDetailFailed'), error);
  }
  defectListLoading.value = true;
  try {
    const res = (await listFabricDefectsByInspection(row.id)) as { data?: unknown };
    const d = res.data;
    defectList.value = Array.isArray(d)
      ? d
      : ((d as { items?: Array<Record<string, unknown>> })?.items ?? []);
  } catch (error) {
    logger.error(t('message.loadFabricDefectsFailed'), error);
    defectList.value = [];
  } finally {
    defectListLoading.value = false;
  }
}

// 疵点详情回源
async function viewDefect(row: Record<string, unknown>) {
  try {
    const res = (await getFabricDefect(row.id as number)) as { data?: Record<string, unknown> };
    const d = res.data ?? {};
    ElMessageBox.alert(
      Object.entries(d)
        .map(([k, v]) => `${k}: ${v ?? '-'}`)
        .join('\n'),
      `疵点 #${row.id}`
    );
  } catch (e) {
    ElMessage.error((e as Error).message || '查询疵点失败');
  }
}

// 疵点删除（仅 inspecting 态）
async function removeDefect(row: Record<string, unknown>) {
  try {
    await ElMessageBox.confirm(`确认删除疵点 #${row.id}？`, '删除确认', { type: 'warning' });
  } catch {
    return;
  }
  try {
    await deleteFabricDefect(row.id as number);
    ElMessage.success('疵点已删除');
    if (detailRow.value?.id) await openDetail({ id: detailRow.value.id } as FabricInspection);
  } catch (e) {
    if (e !== 'cancel') ElMessage.error((e as Error).message || '删除失败');
  }
}

// ===== 打卷入库（addFabricRoll，仅 graded 态） =====
const rollVisible = ref(false);
const rollSaving = ref(false);
const rollingId = ref<number | null>(null);
const rollForm = reactive({
  warehouse_id: 1,
  roll_length: undefined as number | undefined,
  roll_weight: undefined as number | undefined,
  roll_width: undefined as number | undefined,
  roll_gram_weight: undefined as number | undefined,
});

function openRoll(row: FabricInspection) {
  rollingId.value = row.id;
  rollVisible.value = true;
}

async function onRollSubmit() {
  if (!rollingId.value || !rollForm.warehouse_id || !rollForm.roll_length) {
    ElMessage.warning('请填写仓库与卷长');
    return;
  }
  rollSaving.value = true;
  try {
    await addFabricRoll(rollingId.value, {
      warehouse_id: rollForm.warehouse_id,
      roll_length: rollForm.roll_length,
      roll_weight: rollForm.roll_weight ?? undefined,
      roll_width: rollForm.roll_width ?? undefined,
      roll_gram_weight: rollForm.roll_gram_weight ?? undefined,
    });
    ElMessage.success('打卷入库成功');
    rollVisible.value = false;
    await load();
  } catch (e) {
    ElMessage.error((e as Error).message || '打卷失败');
  } finally {
    rollSaving.value = false;
  }
}

// ===== 疵点登记（createFabricDefect，inspecting 态） =====
const defectVisible = ref(false);
const defectSaving = ref(false);
const defectTargetId = ref<number | null>(null);
const defectForm = reactive({
  defect_type: 'broken_end',
  position_yards: undefined as number | undefined,
  defect_length_inches: undefined as number | undefined,
  direction: '',
  is_hole: false,
  is_continuous: false,
  is_half_width: false,
  description: '',
});

function openDefect(row: FabricInspection) {
  defectTargetId.value = row.id;
  Object.assign(defectForm, {
    defect_type: 'broken_end',
    position_yards: undefined,
    defect_length_inches: undefined,
    direction: '',
    is_hole: false,
    is_continuous: false,
    is_half_width: false,
    description: '',
  });
  defectVisible.value = true;
}

async function onDefectSubmit() {
  if (!defectTargetId.value) {
    ElMessage.warning('请选择验布中的验布单');
    return;
  }
  if (
    !defectForm.defect_type ||
    defectForm.position_yards == null ||
    defectForm.defect_length_inches == null
  ) {
    ElMessage.warning('请填写疵点类型/位置/长度');
    return;
  }
  defectSaving.value = true;
  try {
    await createFabricDefect({
      inspection_id: defectTargetId.value,
      defect_type: defectForm.defect_type,
      position_yards: defectForm.position_yards,
      defect_length_inches: defectForm.defect_length_inches,
      direction: defectForm.direction || undefined,
      is_hole: defectForm.is_hole || undefined,
      is_continuous: defectForm.is_continuous || undefined,
      is_half_width: defectForm.is_half_width || undefined,
      description: defectForm.description || undefined,
    });
    ElMessage.success('疵点已登记');
    defectVisible.value = false;
    await load();
  } catch (e) {
    ElMessage.error((e as Error).message || '疵点登记失败');
  } finally {
    defectSaving.value = false;
  }
}

function onGrade(row: FabricInspection) {
  gradingId.value = row.id;
  gradeForm.inspected_yards = undefined;
  gradeForm.qualification_rate = undefined;
  gradeForm.remarks = '';
  gradeVisible.value = true;
}

async function onGradeSubmit() {
  if (gradingId.value === null) return;
  if (!gradeForm.inspected_yards) {
    ElMessage.warning('请填写检验码数');
    return;
  }
  saving.value = true;
  try {
    await gradeFabricInspection(gradingId.value, {
      inspected_yards: gradeForm.inspected_yards,
      qualification_rate: gradeForm.qualification_rate ?? undefined,
    });
    ElMessage.success('定级完成');
    gradeVisible.value = false;
    await load();
  } finally {
    saving.value = false;
  }
}

async function onClose(row: FabricInspection) {
  await ElMessageBox.confirm('确认关闭该验布单？', '关闭确认');
  await closeFabricInspection(row.id);
  ElMessage.success('已关闭');
  await load();
}

async function onDelete(row: FabricInspection) {
  await ElMessageBox.confirm('确认删除该验布单？', '删除确认');
  await deleteFabricInspection(row.id);
  ElMessage.success('已删除');
  await load();
}

onMounted(load);
</script>

<style scoped>
.section-title {
  margin: 14px 0 8px;
  font-size: 14px;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.w-full {
  width: 100%;
}
</style>
