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
        <el-table-column label="操作" width="360" fixed="right">
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
            <!-- #220：打卷成功即产出染色匹(inventory_piece, piece_type=dyed)，
                 标签入口挂在 rolled 行；打开后按缸号+dyed 过滤真实匹列表逐匹下载 docx。
                 权限键 pieces:print 与后端 URL 段推导一致（constants/permissions.ts 注释） -->
            <el-button
              v-if="row.status === 'rolled'"
              v-permission="PERMISSIONS.INVENTORY_PIECE_PRINT"
              size="small"
              type="primary"
              plain
              @click="openPieceLabelDialog(row)"
              >{{ t('fabricInspections.label.button') }}</el-button
            >
            <el-button
              v-if="['graded', 'rolled'].includes(row.status)"
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
      <el-form ref="rollFormRef" :model="rollForm" :rules="rollRules" label-width="130px">
        <el-form-item label="仓库 ID" prop="warehouse_id" required>
          <el-input-number v-model="rollForm.warehouse_id" :min="1" class="w-full" />
        </el-form-item>
        <el-form-item label="卷长 m" prop="roll_length" required>
          <el-input-number
            v-model="rollForm.roll_length"
            :min="0.01"
            :precision="2"
            class="w-full"
          />
        </el-form-item>
        <!-- #220 裁定 §3：卷重/幅宽/克重由可选改必填，口径与后端 RollFabricRequest
             逐字段一致（validate(required) + >0 范围校验，无上限）；初值为空不预填 0，
             杜绝以假默认值冒充实测值（成品布标签三列 fail-closed 依赖实测值） -->
        <el-form-item label="卷重 kg" prop="roll_weight" required>
          <el-input-number v-model="rollForm.roll_weight" :min="0" :precision="2" class="w-full" />
        </el-form-item>
        <el-form-item label="幅宽 cm" prop="roll_width" required>
          <el-input-number v-model="rollForm.roll_width" :min="0" :precision="1" class="w-full" />
        </el-form-item>
        <el-form-item label="克重 g/m2" prop="roll_gram_weight" required>
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

    <!-- #220 成品布入库标签：数据源为 GET /inventory/pieces 按 缸号+piece_type=dyed
         下推查询的真实匹行（inventory_piece.id 即打印端点入参），不手写死数据 -->
    <el-dialog v-model="labelVisible" :title="t('fabricInspections.label.dialogTitle')" width="760">
      <p class="label-scope">
        {{ t('fabricInspections.label.scope', { dyeLotNo: labelDyeLotNo }) }}
      </p>
      <el-table
        v-loading="labelLoading"
        :data="labelPieces"
        border
        size="small"
        max-height="360"
        :empty-text="t('fabricInspections.label.empty')"
      >
        <el-table-column
          prop="piece_no"
          :label="t('fabricInspections.label.pieceNo')"
          min-width="140"
        />
        <el-table-column :label="t('fabricInspections.label.colorNo')" width="110">
          <template #default="{ row }">{{ row.color_no || '-' }}</template>
        </el-table-column>
        <el-table-column :label="t('fabricInspections.label.length')" width="100">
          <!-- Decimal 序列化为字符串，Number() 归一展示，禁 .toFixed 造数 -->
          <template #default="{ row }">{{ Number(row.length) }}</template>
        </el-table-column>
        <el-table-column :label="t('fabricInspections.label.weight')" width="110">
          <template #default="{ row }">{{
            row.weight === null ? '-' : Number(row.weight)
          }}</template>
        </el-table-column>
        <el-table-column :label="t('fabricInspections.label.width')" min-width="110">
          <template #default="{ row }">{{
            row.width == null ? t('fabricInspections.label.notRecorded') : Number(row.width)
          }}</template>
        </el-table-column>
        <el-table-column :label="t('fabricInspections.label.gramWeight')" min-width="120">
          <template #default="{ row }">{{
            row.gram_weight == null
              ? t('fabricInspections.label.notRecorded')
              : Number(row.gram_weight)
          }}</template>
        </el-table-column>
        <el-table-column :label="t('fabricInspections.label.warehouseInAt')" min-width="170">
          <template #default="{ row }">{{ row.warehouse_in_at || '-' }}</template>
        </el-table-column>
        <el-table-column :label="t('fabricInspections.label.action')" width="110" fixed="right">
          <template #default="{ row }">
            <el-button
              size="small"
              link
              type="primary"
              :loading="printingPieceId === row.id"
              @click="onPrintPieceLabel(row)"
              >{{ t('fabricInspections.label.button') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
      <template #footer>
        <el-button @click="labelVisible = false">{{ t('common.close') }}</el-button>
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
import type { FormInstance, FormRules } from 'element-plus';
import { useI18n } from 'vue-i18n';
import { PERMISSIONS } from '@/constants/permissions';
import {
  extractAppErrorEnvelope,
  listInventoryPieces,
  PIECE_TYPE,
  printInventoryPieceLabelDocx,
  type InventoryPieceRow,
} from '@/api/inventory-transfer';
import {
  closeFabricInspection,
  createFabricInspection,
  deleteFabricInspection,
  getFabricInspectionList,
  gradeFabricInspection,
  getFabricInspectionDetail,
  startFabricInspection,
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
const gradingStatus = ref<string>('');

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
  (
    ({
      pending: 'info',
      inspecting: 'warning',
      graded: 'success',
      rolled: 'success',
      closed: '',
    }) as Record<string, string>
  )[s] ?? 'info';

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
  } catch (e) {
    logger.error('加载验布列表失败', e);
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
  } catch (e) {
    ElMessage.error(
      (e as { response?: { data?: { message?: string } } })?.response?.data?.message ?? '创建失败'
    );
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
    } catch (e) {
      ElMessage.error(
        (e as { response?: { data?: { message?: string } } })?.response?.data?.message ?? '更新失败'
      );
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
const rollFormRef = ref<FormInstance>();
// #220：三实测值必填且无假默认——初值为空（undefined），置空即"未填"，不以 0 冒充实测值
const rollForm = reactive({
  warehouse_id: undefined as number | undefined,
  roll_length: undefined as number | undefined,
  roll_weight: undefined as number | undefined,
  roll_width: undefined as number | undefined,
  roll_gram_weight: undefined as number | undefined,
});

/**
 * >0 范围校验：与后端 validate_roll_preconditions（打卷长度/重量/幅宽/克重均须 >0）
 * 同口径——required + 正数，无上限、无精度收紧（前端校验严于后端是缺陷）。
 * 空值交给同字段的 required 规则提示"必须填写"，本规则只拦 0/负数/非有限数
 * （async-validator 的 required 对数字 0 视为有效值，拦不住 0，必须补本条）。
 */
const positiveValidator =
  (message: string) => (_rule: unknown, value: unknown, callback: (err?: Error) => void) => {
    if (value === undefined || value === null) {
      callback();
      return;
    }
    const num = Number(value);
    if (Number.isFinite(num) && num > 0) callback();
    else callback(new Error(message));
  };

const rollRules: FormRules = {
  warehouse_id: [
    { required: true, message: t('fabricInspections.roll.warehouseRequired'), trigger: 'change' },
  ],
  roll_length: [
    { required: true, message: t('fabricInspections.roll.lengthRequired'), trigger: 'change' },
    {
      validator: positiveValidator(t('fabricInspections.roll.lengthPositive')),
      trigger: 'change',
    },
  ],
  roll_weight: [
    { required: true, message: t('fabricInspections.roll.weightRequired'), trigger: 'change' },
    {
      validator: positiveValidator(t('fabricInspections.roll.weightPositive')),
      trigger: 'change',
    },
  ],
  roll_width: [
    { required: true, message: t('fabricInspections.roll.widthRequired'), trigger: 'change' },
    {
      validator: positiveValidator(t('fabricInspections.roll.widthPositive')),
      trigger: 'change',
    },
  ],
  roll_gram_weight: [
    { required: true, message: t('fabricInspections.roll.gramWeightRequired'), trigger: 'change' },
    {
      validator: positiveValidator(t('fabricInspections.roll.gramWeightPositive')),
      trigger: 'change',
    },
  ],
};

function openRoll(row: FabricInspection) {
  rollingId.value = row.id;
  Object.assign(rollForm, {
    warehouse_id: undefined,
    roll_length: undefined,
    roll_weight: undefined,
    roll_width: undefined,
    roll_gram_weight: undefined,
  });
  // 重开对话框不带上一单的校验红字/残值
  rollFormRef.value?.clearValidate();
  rollVisible.value = true;
}

async function onRollSubmit() {
  if (!rollingId.value) return;
  const formEl = rollFormRef.value;
  if (!formEl) return;
  try {
    await formEl.validate();
  } catch (invalidFields) {
    // ElForm.validate() 的 reject 必须在此接住：未捕获会沿 Promise 冒成整页
    // "页面加载出错"（本仓实证崩溃族）。给出用户可懂的补全提示并留日志。
    logger.error(t('fabricInspections.roll.validateFailed'), invalidFields);
    ElMessage.warning(t('fabricInspections.roll.formIncomplete'));
    return;
  }
  // 提交前守卫：与 rules 同口径（必填 + >0）。rules 通过后 TS 并不收窄类型，
  // 此处显式拒发异常值（校验被绕过属真缺陷），不做 ?? 兜底掩盖。
  const { warehouse_id, roll_length, roll_weight, roll_width, roll_gram_weight } = rollForm;
  if (
    !warehouse_id ||
    roll_length === undefined ||
    roll_length <= 0 ||
    roll_weight === undefined ||
    roll_weight <= 0 ||
    roll_width === undefined ||
    roll_width <= 0 ||
    roll_gram_weight === undefined ||
    roll_gram_weight <= 0
  ) {
    logger.error(t('fabricInspections.roll.formIncomplete'), { ...rollForm });
    ElMessage.warning(t('fabricInspections.roll.formIncomplete'));
    return;
  }
  rollSaving.value = true;
  try {
    await addFabricRoll(rollingId.value, {
      warehouse_id,
      roll_length,
      roll_weight,
      roll_width,
      roll_gram_weight,
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

// ===== #220 成品布入库标签打印（rolled 态入口，权限码 pieces:print 见 constants/permissions.ts） =====
const labelVisible = ref(false);
const labelLoading = ref(false);
const labelPieces = ref<InventoryPieceRow[]>([]);
const labelDyeLotNo = ref('');
const printingPieceId = ref<number | null>(null);

/**
 * 打开标签清单：验布打卷产出的是染色匹（后端 build_piece_active_model
 * piece_type="dyed"，fabric_inspection_service.rs:671），按 dye_lot_no + piece_type=dyed
 * 下推 GET /inventory/pieces（词表值引自 api/inventory-transfer.ts::PIECE_TYPE，
 * 与 piece_domain_service.rs:18-19 逐字符相同）。
 * 后端列表无 inspection_id 过滤维度，清单范围=该缸号全部染色匹：其中委外收回/拆匹
 * 来源的匹三实测列为 NULL，打印会被后端 fail-closed 逐列点名拒绝——这是后端报告 §⑤
 * 声明的既有真实现状（后续"收回确认补录"另行立项），前端不预先隐藏也不掩饰提示。
 */
async function openPieceLabelDialog(row: FabricInspection) {
  const dyeLotNo = row.dye_lot_no;
  if (typeof dyeLotNo !== 'string' || dyeLotNo.trim() === '') {
    // rolled 验布单必有缸号（后端打卷前置校验），走到这里即契约异状：显式报错，不发空参查询
    logger.error(t('fabricInspections.label.noDyeLot'), row);
    ElMessage.warning(t('fabricInspections.label.noDyeLot'));
    return;
  }
  labelDyeLotNo.value = dyeLotNo;
  labelPieces.value = [];
  labelVisible.value = true;
  labelLoading.value = true;
  try {
    const res = await listInventoryPieces({
      dye_lot_no: dyeLotNo,
      piece_type: PIECE_TYPE.DYED,
      page: 1,
      page_size: 100,
    });
    // 后端返回 ApiResponse<PaginatedResponse>，列表键恒为 data.items；缺键即抛，
    // 不用 || [] 把契约漂移静默成"清单恒空"
    const items = res.data?.items;
    if (!Array.isArray(items)) {
      throw new Error(`GET /inventory/pieces 响应缺少 data.items 数组：${JSON.stringify(res)}`);
    }
    labelPieces.value = items;
  } catch (error) {
    logger.error(t('fabricInspections.label.loadFailed'), error);
    ElMessage.error(t('fabricInspections.label.loadFailed'));
  } finally {
    labelLoading.value = false;
  }
}

/**
 * 单匹下载标签 docx（不做批量——后端端点即单匹形状，勿臆想功能）。
 * 失败分支对齐后端 fail-closed 文案族（拒绝口径见 backend/src/services/print_service.rs::get_inventory_piece_label_print_data）：
 * - BUSINESS_ERROR：出参脱敏为固定文案，真实原因仅服务端日志——前端只陈述公开规则，
 *   不臆造后端原因（样布/非染色匹门控即此族）；
 * - VALIDATION_ERROR（缺维逐列点名）等可外显信封：message 本身即用户可读文案，原样展示；
 * - 信封不可解析（非 JSON 体/网络层失败）：通用失败文案并留 status/原始错误日志，
 *   绝不"解析失败当成功"或静默。
 */
async function onPrintPieceLabel(row: InventoryPieceRow) {
  printingPieceId.value = row.id;
  try {
    const blob = await printInventoryPieceLabelDocx(row.id);
    // 下载范式照 ap/tabs/PaymentTab.vue printPayment（createObjectURL + a[download] + revoke）
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${row.piece_no}.docx`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    // 后端已记 OperationType::Print 审计留痕；前端侧留成功回执日志（logger.info 仅开发态，
    // 可观测主链路在服务端审计，不双写假账）
    logger.info(t('fabricInspections.label.printSuccess'), { pieceNo: row.piece_no });
    ElMessage.success(t('fabricInspections.label.printSuccess'));
  } catch (e) {
    const status = (e as { response?: { status?: number } })?.response?.status;
    const envelope = await extractAppErrorEnvelope(e);
    if (envelope && envelope.code === 'BUSINESS_ERROR') {
      logger.error(t('fabricInspections.label.printBusinessRejected'), {
        pieceNo: row.piece_no,
        traceId: envelope.trace_id,
        status,
      });
      ElMessage.error(t('fabricInspections.label.printBusinessRejected'));
    } else if (envelope) {
      logger.error(envelope.message, {
        pieceNo: row.piece_no,
        code: envelope.code,
        traceId: envelope.trace_id,
        status,
        error: e,
      });
      ElMessage.error(envelope.message);
    } else {
      logger.error(t('fabricInspections.label.printFailed'), {
        pieceNo: row.piece_no,
        status,
        error: e,
      });
      ElMessage.error(t('fabricInspections.label.printFailed'));
    }
  } finally {
    printingPieceId.value = null;
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
  gradingStatus.value = row.status;
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
    if (gradingStatus.value === 'pending') {
      await startFabricInspection(gradingId.value);
    }
    await gradeFabricInspection(gradingId.value, {
      inspected_yards: gradeForm.inspected_yards,
      qualification_rate: gradeForm.qualification_rate ?? undefined,
    });
    ElMessage.success('定级完成');
    gradeVisible.value = false;
    await load();
  } catch (e) {
    ElMessage.error(
      (e as { response?: { data?: { message?: string } } })?.response?.data?.message ?? '定级失败'
    );
  } finally {
    saving.value = false;
  }
}

async function onClose(row: FabricInspection) {
  try {
    await ElMessageBox.confirm('确认关闭该验布单？', '关闭确认');
  } catch {
    return;
  }
  try {
    await closeFabricInspection(row.id);
    ElMessage.success('已关闭');
    await load();
  } catch (e) {
    ElMessage.error(
      (e as { response?: { data?: { message?: string } } })?.response?.data?.message ?? '关闭失败'
    );
  }
}

async function onDelete(row: FabricInspection) {
  try {
    await ElMessageBox.confirm('确认删除该验布单？', '删除确认');
  } catch {
    return;
  }
  try {
    await deleteFabricInspection(row.id);
    ElMessage.success('已删除');
    await load();
  } catch (e) {
    ElMessage.error(
      (e as { response?: { data?: { message?: string } } })?.response?.data?.message ?? '删除失败'
    );
  }
}

onMounted(load);
</script>

<style scoped>
.section-title {
  margin: 14px 0 8px;
  font-size: 14px;
}
.label-scope {
  margin: 0 0 10px;
  font-size: 13px;
  color: #606266;
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
