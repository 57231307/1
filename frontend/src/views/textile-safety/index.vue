<template>
  <div class="textile-safety-page">
    <el-card shadow="never">
      <div class="page-header">
        <h2>纺织品安全送检管理</h2>
        <div class="header-actions">
          <el-select
            v-model="queryStatus"
            placeholder="全部状态"
            clearable
            style="width: 140px"
            @change="loadList"
          >
            <el-option
              v-for="(label, key) in SUBMISSION_STATUS_LABELS"
              :key="key"
              :label="label"
              :value="key"
            />
          </el-select>
          <el-select
            v-model="queryMarket"
            placeholder="全部市场"
            clearable
            style="width: 130px"
            @change="loadList"
          >
            <el-option
              v-for="(label, key) in TARGET_MARKET_LABELS"
              :key="key"
              :label="label"
              :value="key"
            />
          </el-select>
          <el-button type="primary" @click="openCreate">新建送检单</el-button>
        </div>
      </div>

      <el-table v-loading="loading" :data="rows" border>
        <el-table-column prop="submission_no" label="送检单号" min-width="150" />
        <el-table-column label="安全类别" width="150">
          <template #default="{ row }">
            <el-tag size="small">{{
              SAFETY_CATEGORY_LABELS[row.safety_category] ?? row.safety_category
            }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="batch_no" label="批次" min-width="110" show-overflow-tooltip>
          <template #default="{ row }">{{ row.batch_no || '未挂批次' }}</template>
        </el-table-column>
        <el-table-column prop="color_no" label="色号" width="90" />
        <el-table-column label="人群/接触" min-width="150">
          <template #default="{ row }">
            {{ END_USER_GROUP_LABELS[row.end_user_group] ?? row.end_user_group }} /
            {{ CONTACT_TYPE_LABELS[row.contact_type] ?? row.contact_type }}
          </template>
        </el-table-column>
        <el-table-column label="市场" width="90">
          <template #default="{ row }">{{
            TARGET_MARKET_LABELS[row.target_market] ?? row.target_market
          }}</template>
        </el-table-column>
        <el-table-column label="状态" width="100">
          <template #default="{ row }">
            <el-tag :type="statusTagType(row.status)">{{
              SUBMISSION_STATUS_LABELS[row.status] ?? row.status
            }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="整单结论" width="100">
          <template #default="{ row }">
            <el-tag v-if="row.overall_verdict" :type="verdictTagType(row.overall_verdict)">
              {{ VERDICT_LABELS[row.overall_verdict] ?? row.overall_verdict }}
            </el-tag>
            <span v-else class="muted">未判定</span>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="220" fixed="right">
          <template #default="{ row }">
            <el-button link type="primary" @click="openDetail(row.id)">详情</el-button>
            <el-button
              v-for="next in legalNext(row.status)"
              :key="next"
              v-permission="'textile-submissions:update'"
              link
              type="success"
              @click="advance(row, next)"
            >
              {{ actionLabel(row.status, next) }}
            </el-button>
          </template>
        </el-table-column>
      </el-table>

      <el-pagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :total="total"
        layout="total, prev, pager, next, sizes"
        :page-sizes="[10, 20, 50]"
        style="margin-top: 12px; justify-content: flex-end"
        @current-change="loadList"
        @size-change="loadList"
      />
    </el-card>

    <el-dialog v-model="createVisible" title="新建送检单" width="720px" destroy-on-close>
      <el-form ref="createFormRef" :model="createForm" :rules="createRules" label-width="120px">
        <el-form-item label="送检单号" prop="submission_no">
          <el-input v-model="createForm.submission_no" placeholder="例如 SUB-2026-0001" />
        </el-form-item>
        <el-form-item label="产品 ID" prop="product_id">
          <el-input
            v-model="createForm.product_id"
            placeholder="送检单必须挂真实品种，填写产品数字 ID"
            @input="onProductIdInput"
          />
        </el-form-item>
        <el-form-item label="批次号" prop="batch_no">
          <el-input
            v-model="createForm.batch_no"
            placeholder="留空表示工艺级送检，不联动冻结具体批次库存"
          />
        </el-form-item>
        <el-form-item label="色号" prop="color_no">
          <el-input v-model="createForm.color_no" placeholder="取样口径为同品种同色号同工艺" />
        </el-form-item>
        <el-form-item label="工艺号" prop="process_no">
          <el-input v-model="createForm.process_no" />
        </el-form-item>
        <el-form-item label="终端人群" prop="end_user_group">
          <el-select v-model="createForm.end_user_group" style="width: 100%">
            <el-option
              v-for="o in optionList(END_USER_GROUP_LABELS)"
              :key="o.value"
              :label="o.label"
              :value="o.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="接触方式" prop="contact_type">
          <el-select v-model="createForm.contact_type" style="width: 100%">
            <el-option
              v-for="o in optionList(CONTACT_TYPE_LABELS)"
              :key="o.value"
              :label="o.label"
              :value="o.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="安全类别" prop="safety_category">
          <el-select v-model="createForm.safety_category" style="width: 100%">
            <el-option
              v-for="o in optionList(SAFETY_CATEGORY_LABELS)"
              :key="o.value"
              :label="o.label"
              :value="o.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="产品形态" prop="product_form">
          <el-select v-model="createForm.product_form" style="width: 100%">
            <el-option
              v-for="o in optionList(PRODUCT_FORM_LABELS)"
              :key="o.value"
              :label="o.label"
              :value="o.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="目标市场" prop="target_market">
          <el-select v-model="createForm.target_market" style="width: 100%">
            <el-option
              v-for="o in optionList(TARGET_MARKET_LABELS)"
              :key="o.value"
              :label="o.label"
              :value="o.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="色号角色" prop="color_role">
          <el-select v-model="createForm.color_role" style="width: 100%">
            <el-option
              v-for="o in optionList(COLOR_ROLE_LABELS)"
              :key="o.value"
              :label="o.label"
              :value="o.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="豁免标签">
          <el-select
            v-model="createForm.exemption_tags"
            multiple
            style="width: 100%"
            placeholder="仅当法定豁免情形成立时选择"
          >
            <el-option
              v-for="o in optionList(EXEMPTION_TAG_LABELS)"
              :key="o.value"
              :label="o.label"
              :value="o.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="备注">
          <el-input
            v-model="createForm.remarks"
            type="textarea"
            :rows="2"
            maxlength="500"
            show-word-limit
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="createVisible = false">取消</el-button>
        <el-button type="primary" :loading="creating" @click="submitCreate">创建</el-button>
      </template>
    </el-dialog>

    <el-drawer v-model="detailVisible" title="送检单详情" size="72%" destroy-on-close>
      <SubmissionDetail v-if="detailId" :submission-id="detailId" @changed="onDetailChanged" />
    </el-drawer>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus';
import {
  createSubmission,
  listSubmissions,
  setSubmissionStatus,
  type CreateSubmissionBody,
  type TextileSubmission,
} from '@/api/textile-safety';
import {
  COLOR_ROLE_LABELS,
  CONTACT_TYPE_LABELS,
  END_USER_GROUP_LABELS,
  EXEMPTION_TAG_LABELS,
  PRODUCT_FORM_LABELS,
  SAFETY_CATEGORY_LABELS,
  STATUS_TRANSITIONS,
  SUBMISSION_STATUS_LABELS,
  TARGET_MARKET_LABELS,
  VERDICT_LABELS,
  optionList,
} from '@/constants/textile-safety';
import SubmissionDetail from './components/SubmissionDetail.vue';

const loading = ref(false);
const rows = ref<TextileSubmission[]>([]);
const total = ref(0);
const page = ref(1);
const pageSize = ref(20);
const queryStatus = ref('');
const queryMarket = ref('');

const createVisible = ref(false);
const creating = ref(false);
const createFormRef = ref<FormInstance>();
const createForm = reactive<CreateSubmissionBody & { color_role: string }>({
  submission_no: '',
  product_id: 0,
  batch_no: '',
  color_no: '',
  process_no: '',
  color_role: 'full',
  end_user_group: 'adult',
  contact_type: 'direct',
  safety_category: 'GB18401_B',
  product_form: 'final',
  target_market: 'dom',
  exemption_tags: [],
  remarks: '',
});

const createRules: FormRules = {
  submission_no: [{ required: true, message: '送检单号必填', trigger: 'blur' }],
  product_id: [
    {
      validator: (_rule, value, callback) => {
        const n = Number(value);
        if (!Number.isInteger(n) || n <= 0) {
          callback(new Error('产品 ID 必须是正整数（送检单必须挂真实品种）'));
        } else {
          callback();
        }
      },
      trigger: 'blur',
    },
  ],
  color_no: [
    { required: true, message: '色号必填（取样按同品种同色号同工艺一批）', trigger: 'blur' },
  ],
  process_no: [
    { required: true, message: '工艺号必填（取样按同品种同色号同工艺一批）', trigger: 'blur' },
  ],
};

function onProductIdInput(v: string) {
  createForm.product_id = v === '' ? 0 : Number(v.replace(/[^\d]/g, ''));
}

const detailVisible = ref(false);
const detailId = ref<number | null>(null);

function openDetail(id: number) {
  detailId.value = id;
  detailVisible.value = true;
}

async function loadList() {
  loading.value = true;
  try {
    const data = await listSubmissions({
      status: queryStatus.value || undefined,
      target_market: queryMarket.value || undefined,
      page: page.value,
      page_size: pageSize.value,
    });
    rows.value = data.items;
    total.value = data.total;
  } catch {
    ElMessage.error('加载送检单列表失败');
  } finally {
    loading.value = false;
  }
}

function openCreate() {
  createVisible.value = true;
}

async function submitCreate() {
  const valid = await createFormRef.value?.validate().catch(() => false);
  if (!valid) return;
  creating.value = true;
  try {
    await createSubmission({
      ...createForm,
      batch_no: createForm.batch_no?.trim() ? createForm.batch_no.trim() : null,
      remarks: createForm.remarks?.trim() ? createForm.remarks.trim() : null,
    });
    ElMessage.success('送检单已创建（草稿）');
    createVisible.value = false;
    await loadList();
  } catch (e: unknown) {
    ElMessage.error(extractError(e, '创建失败'));
  } finally {
    creating.value = false;
  }
}

function legalNext(status: string): string[] {
  return STATUS_TRANSITIONS[status] ?? [];
}

function actionLabel(from: string, to: string): string {
  if (to === 'withdrawn') return '撤回';
  if (from === 'sampling' && to === 'sent') return '确认送出';
  return `推进到${SUBMISSION_STATUS_LABELS[to] ?? to}`;
}

async function advance(row: TextileSubmission, next: string) {
  try {
    await ElMessageBox.confirm(
      `确认把送检单 ${row.submission_no} 推进到「${SUBMISSION_STATUS_LABELS[next] ?? next}」？`,
      '状态迁移',
      { type: 'warning' }
    );
  } catch {
    return;
  }
  try {
    const result = await setSubmissionStatus(row.id, next);
    const link = result.stock_link;
    if (link.matched > 0 || link.updated_stock_ids.length > 0) {
      ElMessage.success(`${link.scope_note}（批次库存已联动，可在库存台账读回）`);
    } else {
      ElMessage.warning(link.scope_note);
    }
    await loadList();
  } catch (e: unknown) {
    ElMessage.error(extractError(e, '状态迁移失败'));
  }
}

function onDetailChanged() {
  loadList();
}

function statusTagType(status: string) {
  if (status === 'judged') return 'warning';
  if (status === 'closed') return 'success';
  if (status === 'withdrawn') return 'info';
  if (status === 'reported' || status === 'sent') return 'primary';
  return 'info';
}

function verdictTagType(verdict: string) {
  if (verdict === 'pass') return 'success';
  if (verdict === 'fail') return 'danger';
  return 'info';
}

function extractError(e: unknown, fallback: string): string {
  const msg = (e as { response?: { data?: { message?: string } } })?.response?.data?.message;
  return msg || fallback;
}

onMounted(loadList);
</script>

<style scoped>
.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}
.header-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}
.muted {
  color: var(--el-text-color-secondary);
}
</style>
