<template>
  <div v-loading="loading" class="detail-root">
    <el-descriptions :column="3" border size="small">
      <el-descriptions-item label="送检单号">{{
        detail?.submission.submission_no
      }}</el-descriptions-item>
      <el-descriptions-item label="状态">
        <el-tag>{{
          SUBMISSION_STATUS_LABELS[detail?.submission.status ?? ''] ?? detail?.submission.status
        }}</el-tag>
      </el-descriptions-item>
      <el-descriptions-item label="整单结论">
        <el-tag
          v-if="detail?.submission.overall_verdict"
          :type="verdictTag(detail.submission.overall_verdict)"
        >
          {{ VERDICT_LABELS[detail.submission.overall_verdict] }}
        </el-tag>
        <span v-else class="muted">未判定</span>
      </el-descriptions-item>
      <el-descriptions-item label="安全类别">{{
        SAFETY_CATEGORY_LABELS[detail?.submission.safety_category ?? '']
      }}</el-descriptions-item>
      <el-descriptions-item label="人群">{{
        END_USER_GROUP_LABELS[detail?.submission.end_user_group ?? '']
      }}</el-descriptions-item>
      <el-descriptions-item label="接触方式">{{
        CONTACT_TYPE_LABELS[detail?.submission.contact_type ?? '']
      }}</el-descriptions-item>
      <el-descriptions-item label="形态">{{
        PRODUCT_FORM_LABELS[detail?.submission.product_form ?? '']
      }}</el-descriptions-item>
      <el-descriptions-item label="市场">{{
        TARGET_MARKET_LABELS[detail?.submission.target_market ?? '']
      }}</el-descriptions-item>
      <el-descriptions-item label="批次">{{
        detail?.submission.batch_no || '未挂批次'
      }}</el-descriptions-item>
      <el-descriptions-item label="色号">{{ detail?.submission.color_no }}</el-descriptions-item>
      <el-descriptions-item label="工艺">{{ detail?.submission.process_no }}</el-descriptions-item>
      <el-descriptions-item label="豁免标签">{{ exemptionText }}</el-descriptions-item>
      <el-descriptions-item label="备注" :span="3">{{
        detail?.submission.remarks || '—'
      }}</el-descriptions-item>
    </el-descriptions>

    <div class="section">
      <div class="section-head">
        <h3>检测项目清单</h3>
        <el-button
          v-permission="'textile-submissions:update'"
          size="small"
          type="primary"
          :disabled="!editable"
          @click="regenerateItems"
        >
          按人群与市场推导清单
        </el-button>
      </div>
      <el-table :data="detail?.items ?? []" border size="small">
        <el-table-column prop="item_code" label="编码" width="140" />
        <el-table-column prop="item_name" label="项目" min-width="160" />
        <el-table-column label="依据" width="120">
          <template #default="{ row }">{{
            BASIS_STANDARD_LABELS[row.basis_standard] ?? row.basis_standard
          }}</template>
        </el-table-column>
        <el-table-column prop="source" label="归因" width="110" />
        <el-table-column label="测量类型" width="110">
          <template #default="{ row }">{{
            ITEM_KIND_LABELS[row.item_kind ?? ''] ?? row.item_kind
          }}</template>
        </el-table-column>
      </el-table>
    </div>

    <div class="section">
      <div class="section-head">
        <h3>婴童加考核查（GB 31701）</h3>
        <el-button
          v-permission="'textile-submissions:update'"
          size="small"
          type="primary"
          :disabled="!reportable"
          @click="openCheckDialog"
        >
          登记核查
        </el-button>
      </div>
      <el-table :data="detail?.child_checks ?? []" border size="small">
        <el-table-column prop="item_id" label="项目" width="90" />
        <el-table-column label="类型" width="110">
          <template #default="{ row }">{{
            CHECK_KIND_LABELS[row.check_kind] ?? row.check_kind
          }}</template>
        </el-table-column>
        <el-table-column prop="measured_value" label="实测值" width="110" />
        <el-table-column prop="measured_flag" label="证据 flag" width="110" />
        <el-table-column prop="check_desc" label="现场描述" min-width="160" show-overflow-tooltip />
        <el-table-column label="人工结论" width="110">
          <template #default="{ row }">{{
            VERDICT_LABELS[row.manual_conclusion ?? ''] ?? row.manual_conclusion ?? '待补'
          }}</template>
        </el-table-column>
        <el-table-column label="判定" width="90">
          <template #default="{ row }">{{ VERDICT_LABELS[row.verdict] ?? row.verdict }}</template>
        </el-table-column>
      </el-table>
    </div>

    <div class="section">
      <div class="section-head">
        <h3>第三方检测报告</h3>
        <el-button
          v-permission="'textile-reports:create'"
          size="small"
          type="primary"
          :disabled="!canRegisterReport"
          @click="openReportDialog"
        >
          登记报告
        </el-button>
      </div>
      <el-table :data="detail?.reports ?? []" border size="small">
        <el-table-column prop="report_no" label="报告号" min-width="140" />
        <el-table-column
          prop="inspection_agency"
          label="机构"
          min-width="130"
          show-overflow-tooltip
        />
        <el-table-column label="资质" width="110">
          <template #default="{ row }">
            <el-tag v-if="row.has_cma" size="small" type="success">CMA</el-tag>
            <el-tag v-if="row.has_cnas" size="small" type="success">CNAS</el-tag>
            <span v-if="!row.has_cma && !row.has_cnas" class="muted">无</span>
          </template>
        </el-table-column>
        <el-table-column prop="issue_date" label="签发" width="105" />
        <el-table-column prop="valid_until" label="有效期至" width="105" />
        <el-table-column label="受理态" width="100">
          <template #default="{ row }">
            <el-tag
              :type="
                row.acceptance_status === 'accepted'
                  ? 'success'
                  : row.acceptance_status === 'rejected'
                    ? 'danger'
                    : 'info'
              "
            >
              {{ ACCEPTANCE_STATUS_LABELS[row.acceptance_status] ?? row.acceptance_status }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="缺项点名" min-width="170">
          <template #default="{ row }">
            <span v-if="row.rejection_detail" class="missing">{{
              row.rejection_detail.split(',').map(codeName).join('、')
            }}</span>
            <span v-else class="muted">—</span>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="270" fixed="right">
          <template #default="{ row }">
            <el-button
              v-permission="'textile-reports:update'"
              link
              type="primary"
              @click="runGate(row)"
              >过完整性门</el-button
            >
            <el-button
              v-permission="'textile-reports:update'"
              link
              type="primary"
              @click="openResultsDialog(row)"
              >录入实测</el-button
            >
            <el-button
              v-permission="'textile-reports:update'"
              link
              type="success"
              @click="judgeReport(row)"
              >报告判定</el-button
            >
          </template>
        </el-table-column>
      </el-table>
      <div class="judge-bar">
        <el-button
          v-permission="'textile-submissions:update'"
          type="warning"
          :disabled="detail?.submission.status !== 'reported'"
          :loading="judging"
          @click="judgeSubmission"
        >
          整单判定归档
        </el-button>
        <span class="muted"
          >前置：状态为已收报告、至少一份已受理且在有效期内的报告、逐项目结果与核查行均已判定</span
        >
      </div>
    </div>

    <el-dialog v-model="checkDialogVisible" title="登记婴童加考核查" width="620px" destroy-on-close>
      <el-form label-width="110px">
        <el-form-item label="核查项目">
          <el-select v-model="checkForm.item_code" filterable style="width: 100%">
            <el-option
              v-for="it in gb31701Items"
              :key="it.item_code"
              :label="`${it.item_code} ${it.item_name}`"
              :value="it.item_code"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="实测数值">
          <el-input
            v-model="checkForm.measured_value"
            placeholder="数值型填数字；等级型请走下方等级选择"
          />
        </el-form-item>
        <el-form-item label="证据 flag">
          <el-select v-model="checkForm.measured_flag" clearable style="width: 100%">
            <el-option
              v-for="o in optionList(FLAG_VALUE_LABELS)"
              :key="o.value"
              :label="o.label"
              :value="o.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="现场描述">
          <el-input
            v-model="checkForm.check_desc"
            type="textarea"
            :rows="2"
            maxlength="500"
            show-word-limit
          />
        </el-form-item>
        <el-form-item label="人工结论">
          <el-select
            v-model="checkForm.manual_conclusion"
            clearable
            placeholder="仅人工核查项填写"
            style="width: 100%"
          >
            <el-option
              v-for="o in ['pass', 'fail', 'na']"
              :key="o"
              :label="VERDICT_LABELS[o]"
              :value="o"
            />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="checkDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="submitCheck">保存</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="reportDialogVisible" title="登记第三方报告" width="680px" destroy-on-close>
      <el-form ref="reportFormRef" :model="reportForm" :rules="reportRules" label-width="110px">
        <el-form-item label="报告编号" prop="report_no"
          ><el-input v-model="reportForm.report_no"
        /></el-form-item>
        <el-form-item label="检验机构" prop="inspection_agency"
          ><el-input v-model="reportForm.inspection_agency"
        /></el-form-item>
        <el-form-item label="资质标识">
          <el-checkbox v-model="reportForm.has_cma">CMA</el-checkbox>
          <el-checkbox v-model="reportForm.has_cnas">CNAS</el-checkbox>
        </el-form-item>
        <el-form-item label="样品名称"><el-input v-model="reportForm.sample_name" /></el-form-item>
        <el-form-item label="材质成分"
          ><el-input v-model="reportForm.material_composition" placeholder="例如 棉 95% 氨纶 5%"
        /></el-form-item>
        <el-form-item label="报告色号"><el-input v-model="reportForm.color_no" /></el-form-item>
        <el-form-item label="报告工艺号"><el-input v-model="reportForm.process_no" /></el-form-item>
        <el-form-item label="检验依据">
          <el-input
            v-model="reportForm.test_basis"
            placeholder="完整标准号加类别形态，例如 GB 18401-2010 B类"
          />
        </el-form-item>
        <el-form-item label="判定结论"
          ><el-input v-model="reportForm.conclusion" type="textarea" :rows="2"
        /></el-form-item>
        <el-form-item label="签发日期" prop="issue_date">
          <el-date-picker
            v-model="reportForm.issue_date"
            type="date"
            value-format="YYYY-MM-DD"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="有效期至" prop="valid_until">
          <el-date-picker
            v-model="reportForm.valid_until"
            type="date"
            value-format="YYYY-MM-DD"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="OEKO-TEX 分级">
          <el-select v-model="reportForm.oeko_tex_class" clearable style="width: 100%">
            <el-option
              v-for="o in optionList(OEKO_CLASS_LABELS)"
              :key="o.value"
              :label="o.label"
              :value="o.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="报告文件链接"
          ><el-input v-model="reportForm.report_file_url"
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="reportDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="submitReport">登记</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="resultsDialogVisible"
      title="逐项目实测结果录入"
      width="760px"
      destroy-on-close
    >
      <el-alert
        type="info"
        :closable="false"
        show-icon
        title="数值型填实测数值；等级型只选等级 token（半级编码自动换算）；感官与证据型只选 flag。项目必须已在送检清单内。"
        style="margin-bottom: 10px"
      />
      <el-table :data="resultRows" border size="small">
        <el-table-column label="项目" min-width="170">
          <template #default="{ row }">
            <el-select v-model="row.item_code" filterable style="width: 100%">
              <el-option
                v-for="it in detail?.items ?? []"
                :key="it.item_code"
                :label="`${it.item_code} ${it.item_name}`"
                :value="it.item_code"
              />
            </el-select>
          </template>
        </el-table-column>
        <el-table-column label="实测数值" width="140">
          <template #default="{ row }"><el-input v-model="row.measured_value" /></template>
        </el-table-column>
        <el-table-column label="等级 token" width="140">
          <template #default="{ row }">
            <el-select v-model="row.measured_token" clearable style="width: 100%">
              <el-option v-for="t in FASTNESS_GRADE_TOKENS" :key="t" :label="t" :value="t" />
            </el-select>
          </template>
        </el-table-column>
        <el-table-column label="flag" width="130">
          <template #default="{ row }">
            <el-select v-model="row.measured_text" clearable style="width: 100%">
              <el-option
                v-for="o in optionList(FLAG_VALUE_LABELS)"
                :key="o.value"
                :label="o.label"
                :value="o.value"
              />
            </el-select>
          </template>
        </el-table-column>
        <el-table-column label="检测方法" min-width="140">
          <template #default="{ row }"><el-input v-model="row.test_method" /></template>
        </el-table-column>
        <el-table-column width="60">
          <template #default="{ $index }">
            <el-button link type="danger" @click="resultRows.splice($index, 1)">删行</el-button>
          </template>
        </el-table-column>
      </el-table>
      <el-button size="small" style="margin-top: 8px" @click="addResultRow">加一行</el-button>
      <template #footer>
        <el-button @click="resultsDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="submitResults">保存并复位待判定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue';
import { ElMessage, type FormInstance, type FormRules } from 'element-plus';
import {
  acceptReport,
  generateItems,
  getSubmission,
  judgeReport as apiJudgeReport,
  judgeSubmission as apiJudgeSubmission,
  upsertChildChecks,
  upsertResults,
  createReport,
  type ChildCheckRowBody,
  type RegisterReportBody,
  type ResultRowBody,
  type SubmissionDetail as DetailPayload,
  type TextileReport,
} from '@/api/textile-safety';
import {
  ACCEPTANCE_STATUS_LABELS,
  BASIS_STANDARD_LABELS,
  CHECK_KIND_LABELS,
  CONTACT_TYPE_LABELS,
  END_USER_GROUP_LABELS,
  ELEMENT_CODE_LABELS,
  FASTNESS_GRADE_TOKENS,
  FLAG_VALUE_LABELS,
  ITEM_KIND_LABELS,
  OEKO_CLASS_LABELS,
  PRODUCT_FORM_LABELS,
  SAFETY_CATEGORY_LABELS,
  SUBMISSION_STATUS_LABELS,
  TARGET_MARKET_LABELS,
  VERDICT_LABELS,
  optionList,
} from '@/constants/textile-safety';

const props = defineProps<{ submissionId: number }>();
const emit = defineEmits<{ (e: 'changed'): void }>();

const loading = ref(false);
const judging = ref(false);
const detail = ref<DetailPayload | null>(null);

const editable = computed(() =>
  ['draft', 'sampling', 'sent'].includes(detail.value?.submission.status ?? '')
);
const reportable = computed(() =>
  ['draft', 'sampling', 'sent', 'reported'].includes(detail.value?.submission.status ?? '')
);
const canRegisterReport = computed(() =>
  ['sent', 'reported'].includes(detail.value?.submission.status ?? '')
);

const exemptionText = computed(() => {
  const raw = detail.value?.submission.exemption_tags;
  if (!raw) return '无';
  return raw
    .split(',')
    .map(t => EXEMPTION_TAG_TEXT[t] ?? t)
    .join('、');
});

const EXEMPTION_TAG_TEXT: Record<string, string> = {
  wet_processing: '后续湿处理',
  laundering_fade_non_final: '需洗涤褪色非最终产品',
  plain_or_bleached: '本色或漂白',
  traditional_hand_dyed: '扎染蜡染手工着色',
};

const gb31701Items = computed(() =>
  (detail.value?.items ?? []).filter(i => i.basis_standard === 'GB31701')
);

async function load() {
  loading.value = true;
  try {
    detail.value = await getSubmission(props.submissionId);
  } catch {
    ElMessage.error('加载送检单详情失败');
  } finally {
    loading.value = false;
  }
}

async function regenerateItems() {
  try {
    await generateItems(props.submissionId);
    ElMessage.success('清单已按人群与市场推导');
    await load();
    emit('changed');
  } catch (e: unknown) {
    ElMessage.error(extractError(e, '清单推导失败'));
  }
}

const checkDialogVisible = ref(false);
const checkForm = reactive<ChildCheckRowBody & { item_code: string }>({
  item_code: '',
  measured_value: '',
  measured_flag: '',
  check_desc: '',
  manual_conclusion: '',
});

function openCheckDialog() {
  checkDialogVisible.value = true;
}

async function submitCheck() {
  if (!checkForm.item_code) {
    ElMessage.warning('请选择核查项目');
    return;
  }
  const row: ChildCheckRowBody = {
    item_code: checkForm.item_code,
    measured_value: numericOrNull(checkForm.measured_value),
    measured_flag: checkForm.measured_flag || null,
    check_desc: checkForm.check_desc || null,
    manual_conclusion: checkForm.manual_conclusion || null,
  };
  try {
    await upsertChildChecks(props.submissionId, [row]);
    ElMessage.success('核查已登记（重录即复位待判定）');
    checkDialogVisible.value = false;
    await load();
  } catch (e: unknown) {
    ElMessage.error(extractError(e, '核查登记失败'));
  }
}

const reportDialogVisible = ref(false);
const reportFormRef = ref<FormInstance>();
const reportForm = reactive<RegisterReportBody>({
  report_no: '',
  submission_id: props.submissionId,
  inspection_agency: '',
  has_cma: false,
  has_cnas: false,
  sample_name: '',
  material_composition: '',
  color_no: '',
  process_no: '',
  test_basis: '',
  conclusion: '',
  oeko_tex_class: null,
  issue_date: '',
  valid_until: null,
  report_file_url: '',
});
const reportRules: FormRules = {
  report_no: [{ required: true, message: '报告编号必填', trigger: 'blur' }],
  inspection_agency: [{ required: true, message: '检验机构必填', trigger: 'blur' }],
  issue_date: [{ required: true, message: '签发日期必填', trigger: 'change' }],
};

function openReportDialog() {
  reportForm.submission_id = props.submissionId;
  reportForm.color_no = detail.value?.submission.color_no ?? '';
  reportForm.process_no = detail.value?.submission.process_no ?? '';
  reportDialogVisible.value = true;
}

async function submitReport() {
  const valid = await reportFormRef.value?.validate().catch(() => false);
  if (!valid) return;
  try {
    await createReport({ ...reportForm });
    ElMessage.success('报告已登记（受理态待完整性门核对）');
    reportDialogVisible.value = false;
    await load();
    emit('changed');
  } catch (e: unknown) {
    ElMessage.error(extractError(e, '报告登记失败'));
  }
}

async function runGate(row: TextileReport) {
  try {
    const result = await acceptReport(row.id);
    if (result.missing_elements.length === 0) {
      ElMessage.success(`报告 ${row.report_no} 已过完整性门（受理）`);
    } else {
      ElMessage.warning(
        `报告 ${row.report_no} 被退回，缺项：${result.missing_elements.map(codeName).join('、')}`
      );
    }
    await load();
  } catch (e: unknown) {
    ElMessage.error(extractError(e, '完整性门核对失败'));
  }
}

const resultsDialogVisible = ref(false);
const resultsReportId = ref<number | null>(null);
const resultRows = ref<ResultRowBody[]>([]);

function openResultsDialog(row: TextileReport) {
  resultsReportId.value = row.id;
  resultRows.value = [];
  resultsDialogVisible.value = true;
}

function addResultRow() {
  resultRows.value.push({
    item_code: '',
    measured_value: null,
    measured_token: null,
    measured_text: null,
    test_method: '',
  });
}

async function submitResults() {
  if (!resultsReportId.value) return;
  const rows = resultRows.value.filter(r => r.item_code);
  if (rows.length === 0) {
    ElMessage.warning('至少录入一行且必须选择项目');
    return;
  }
  for (const r of rows) {
    if (
      r.measured_value !== null &&
      r.measured_value !== undefined &&
      String(r.measured_value).trim() !== ''
    ) {
      if (Number.isNaN(Number(r.measured_value))) {
        ElMessage.error(`项目 ${r.item_code} 的实测数值不是合法数字`);
        return;
      }
    }
  }
  try {
    await upsertResults(
      resultsReportId.value,
      rows.map(r => ({
        ...r,
        measured_value: numericOrNull(r.measured_value),
        measured_token: r.measured_token || null,
        measured_text: r.measured_text || null,
        test_method: r.test_method || null,
      }))
    );
    ElMessage.success('实测已保存，结论复位为待判定');
    resultsDialogVisible.value = false;
    await load();
  } catch (e: unknown) {
    ElMessage.error(extractError(e, '实测保存失败'));
  }
}

async function judgeReport(row: TextileReport) {
  try {
    const judged = await apiJudgeReport(row.id);
    const fails = judged.filter(r => r.verdict === 'fail').length;
    ElMessage.success(`报告判定完成 ${judged.length} 行，其中不合格 ${fails} 行`);
    await load();
  } catch (e: unknown) {
    ElMessage.error(extractError(e, '报告判定失败'));
  }
}

async function judgeSubmission() {
  judging.value = true;
  try {
    const outcome = await apiJudgeSubmission(props.submissionId);
    const parts = [
      `整单结论：${VERDICT_LABELS[outcome.submission.overall_verdict ?? ''] ?? '未产出'}`,
    ];
    parts.push(outcome.stock_link.scope_note);
    if (outcome.expired_reports_named.length > 0) {
      parts.push(`过期已受理报告已点名排除：${outcome.expired_reports_named.join('、')}`);
    }
    if (outcome.submission.overall_verdict === 'fail') {
      ElMessage.warning(parts.join('；'));
    } else {
      ElMessage.success(parts.join('；'));
    }
    await load();
    emit('changed');
  } catch (e: unknown) {
    ElMessage.error(extractError(e, '整单判定失败'));
  } finally {
    judging.value = false;
  }
}

function codeName(code: string): string {
  return ELEMENT_CODE_LABELS[code.trim()] ?? code;
}

function numericOrNull(v: unknown): number | null {
  if (v === null || v === undefined) return null;
  const s = String(v).trim();
  if (s === '') return null;
  const n = Number(s);
  return Number.isNaN(n) ? null : n;
}

function verdictTag(v: string) {
  return v === 'pass' ? 'success' : v === 'fail' ? 'danger' : 'info';
}

function extractError(e: unknown, fallback: string): string {
  const msg = (e as { response?: { data?: { message?: string } } })?.response?.data?.message;
  return msg || fallback;
}

onMounted(load);
</script>

<style scoped>
.section {
  margin-top: 18px;
}
.section-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}
.section-head h3 {
  margin: 0;
  font-size: 15px;
}
.judge-bar {
  margin-top: 12px;
  display: flex;
  gap: 12px;
  align-items: center;
}
.muted {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}
.missing {
  color: var(--el-color-danger);
  font-size: 12px;
}
</style>
