<template>
  <div class="textile-master-page">
    <el-card shadow="never">
      <el-tabs v-model="activeTab">
        <el-tab-pane label="检测项目主数据" name="items">
          <div class="toolbar">
            <el-select
              v-model="itemBasis"
              placeholder="全部依据"
              clearable
              style="width: 220px"
              @change="loadItems"
            >
              <el-option
                v-for="o in optionList(BASIS_STANDARD_LABELS)"
                :key="o.value"
                :label="o.label"
                :value="o.value"
              />
            </el-select>
            <el-button
              v-permission="'textile-test-items:create'"
              type="primary"
              @click="openItemDialog()"
              >新建项目</el-button
            >
          </div>
          <el-table v-loading="itemsLoading" :data="items" border size="small">
            <el-table-column prop="item_code" label="编码" width="150" />
            <el-table-column prop="item_name" label="名称" min-width="160" />
            <el-table-column label="测量类型" width="100">
              <template #default="{ row }">{{
                ITEM_KIND_LABELS[row.item_kind] ?? row.item_kind
              }}</template>
            </el-table-column>
            <el-table-column label="依据" width="180">
              <template #default="{ row }">{{
                BASIS_STANDARD_LABELS[row.basis_standard] ?? row.basis_standard
              }}</template>
            </el-table-column>
            <el-table-column label="核查形态" width="100">
              <template #default="{ row }">{{
                CHECK_KIND_LABELS[row.check_kind] ?? row.check_kind
              }}</template>
            </el-table-column>
            <el-table-column prop="method_standard" label="方法标准" width="140" />
            <el-table-column prop="unit" label="单位" width="80" />
            <el-table-column label="操作" width="140" fixed="right">
              <template #default="{ row }">
                <el-button
                  v-permission="'textile-test-items:update'"
                  link
                  type="primary"
                  @click="openItemDialog(row)"
                  >编辑</el-button
                >
                <el-button
                  v-permission="'textile-test-items:delete'"
                  link
                  type="danger"
                  @click="removeItem(row)"
                  >删除</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>

        <el-tab-pane label="限值判定表" name="limits">
          <div class="toolbar">
            <el-select
              v-model="limitCategory"
              placeholder="全部安全类别"
              clearable
              style="width: 200px"
              @change="loadLimits"
            >
              <el-option
                v-for="o in [
                  ...optionList(SAFETY_CATEGORY_LABELS),
                  { value: 'any', label: '通配 any' },
                ]"
                :key="o.value"
                :label="o.label"
                :value="o.value"
              />
            </el-select>
            <el-button
              v-permission="'textile-limit-rules:create'"
              type="primary"
              @click="openLimitDialog()"
              >新增限值行</el-button
            >
          </div>
          <el-table v-loading="limitsLoading" :data="limits" border size="small">
            <el-table-column label="项目" min-width="170">
              <template #default="{ row }">{{ itemName(row.item_id) }}</template>
            </el-table-column>
            <el-table-column label="类别" width="120">
              <template #default="{ row }">{{
                SAFETY_CATEGORY_LABELS[row.safety_category] ?? row.safety_category
              }}</template>
            </el-table-column>
            <el-table-column label="形态" width="100">
              <template #default="{ row }">{{
                PRODUCT_FORM_LABELS[row.product_form] ?? row.product_form
              }}</template>
            </el-table-column>
            <el-table-column prop="condition_tag" label="条件" width="120" />
            <el-table-column label="比较器" width="110">
              <template #default="{ row }">{{
                COMPARATOR_LABELS[row.comparator] ?? row.comparator
              }}</template>
            </el-table-column>
            <el-table-column prop="limit_low" label="下限" width="80" />
            <el-table-column prop="limit_high" label="上限" width="80" />
            <el-table-column prop="clause_ref" label="条款" width="140" />
            <el-table-column prop="remark" label="备注" min-width="180" show-overflow-tooltip />
            <el-table-column label="操作" width="140" fixed="right">
              <template #default="{ row }">
                <el-button
                  v-permission="'textile-limit-rules:update'"
                  link
                  type="primary"
                  @click="openLimitDialog(row)"
                  >编辑</el-button
                >
                <el-button
                  v-permission="'textile-limit-rules:delete'"
                  link
                  type="danger"
                  @click="removeLimit(row)"
                  >删除</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>

        <el-tab-pane label="出口市场要求登记" name="requirements">
          <div class="toolbar">
            <el-select
              v-model="reqMarket"
              placeholder="全部市场"
              clearable
              style="width: 160px"
              @change="loadRequirements"
            >
              <el-option
                v-for="o in optionList(TARGET_MARKET_LABELS)"
                :key="o.value"
                :label="o.label"
                :value="o.value"
              />
            </el-select>
            <el-button
              v-permission="'textile-export-requirements:create'"
              type="primary"
              @click="openReqDialog()"
              >登记要求</el-button
            >
          </div>
          <el-table v-loading="reqLoading" :data="requirements" border size="small">
            <el-table-column prop="requirement_code" label="编码" width="180" />
            <el-table-column label="市场" width="90">
              <template #default="{ row }">{{
                TARGET_MARKET_LABELS[row.market] ?? row.market
              }}</template>
            </el-table-column>
            <el-table-column label="类型" width="150">
              <template #default="{ row }">{{
                REQUIREMENT_TYPE_LABELS[row.requirement_type] ?? row.requirement_type
              }}</template>
            </el-table-column>
            <el-table-column label="OEKO 分级" width="130">
              <template #default="{ row }">{{
                OEKO_CLASS_LABELS[row.oeko_tex_class ?? ''] ?? '—'
              }}</template>
            </el-table-column>
            <el-table-column
              prop="standard_ref"
              label="标准依据"
              min-width="170"
              show-overflow-tooltip
            />
            <el-table-column prop="detail" label="细则" min-width="240" show-overflow-tooltip />
            <el-table-column label="强制" width="70">
              <template #default="{ row }">{{ row.mandatory ? '强制' : '自愿' }}</template>
            </el-table-column>
            <el-table-column label="操作" width="140" fixed="right">
              <template #default="{ row }">
                <el-button
                  v-permission="'textile-export-requirements:update'"
                  link
                  type="primary"
                  @click="openReqDialog(row)"
                  >编辑</el-button
                >
                <el-button
                  v-permission="'textile-export-requirements:delete'"
                  link
                  type="danger"
                  @click="removeReq(row)"
                  >删除</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>
      </el-tabs>
    </el-card>

    <el-dialog
      v-model="itemDialogVisible"
      :title="itemForm.id ? '编辑检测项目' : '新建检测项目'"
      width="620px"
      destroy-on-close
    >
      <el-form ref="itemFormRef" :model="itemForm" :rules="itemRules" label-width="110px">
        <el-form-item label="项目编码" prop="item_code"
          ><el-input
            v-model="itemForm.item_code"
            :disabled="!!itemForm.id"
            placeholder="大写蛇形，例如 HCHO"
        /></el-form-item>
        <el-form-item label="项目名称" prop="item_name"
          ><el-input v-model="itemForm.item_name"
        /></el-form-item>
        <el-form-item label="测量类型" prop="item_kind">
          <el-select v-model="itemForm.item_kind" style="width: 100%">
            <el-option
              v-for="o in optionList(ITEM_KIND_LABELS)"
              :key="o.value"
              :label="o.label"
              :value="o.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="依据标准" prop="basis_standard">
          <el-select v-model="itemForm.basis_standard" style="width: 100%">
            <el-option
              v-for="o in optionList(BASIS_STANDARD_LABELS)"
              :key="o.value"
              :label="o.label"
              :value="o.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="核查形态" prop="check_kind">
          <el-select v-model="itemForm.check_kind" style="width: 100%">
            <el-option
              v-for="o in optionList(CHECK_KIND_LABELS)"
              :key="o.value"
              :label="o.label"
              :value="o.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="方法标准"
          ><el-input v-model="itemForm.method_standard" placeholder="例如 GB/T 2912.1"
        /></el-form-item>
        <el-form-item label="单位"
          ><el-input v-model="itemForm.unit" placeholder="例如 mg/kg 或 级"
        /></el-form-item>
        <el-form-item label="说明"
          ><el-input
            v-model="itemForm.description"
            type="textarea"
            :rows="2"
            maxlength="500"
            show-word-limit
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="itemDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="saveItem">保存</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="limitDialogVisible"
      :title="limitForm.editId ? '编辑限值行' : '新增限值行'"
      width="620px"
      destroy-on-close
    >
      <el-form ref="limitFormRef" :model="limitForm" :rules="limitRules" label-width="110px">
        <el-form-item label="检测项目" prop="item_code">
          <el-select
            v-model="limitForm.item_code"
            filterable
            style="width: 100%"
            :disabled="!!limitForm.editId"
          >
            <el-option
              v-for="it in items"
              :key="it.item_code"
              :label="`${it.item_code} ${it.item_name}`"
              :value="it.item_code"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="安全类别" prop="safety_category">
          <el-select v-model="limitForm.safety_category" style="width: 100%">
            <el-option
              v-for="o in [
                ...optionList(SAFETY_CATEGORY_LABELS),
                { value: 'any', label: '通配 any（全类别同一限档）' },
              ]"
              :key="o.value"
              :label="o.label"
              :value="o.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="产品形态" prop="product_form">
          <el-select v-model="limitForm.product_form" style="width: 100%">
            <el-option
              v-for="o in [...optionList(PRODUCT_FORM_LABELS), { value: 'any', label: '通配 any' }]"
              :key="o.value"
              :label="o.label"
              :value="o.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="条件标志" prop="condition_tag">
          <el-select v-model="limitForm.condition_tag" style="width: 100%">
            <el-option label="无豁免 none" value="none" />
            <el-option
              v-for="o in optionList(EXEMPTION_TAG_LABELS)"
              :key="o.value"
              :label="o.label"
              :value="o.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="比较器" prop="comparator">
          <el-select v-model="limitForm.comparator" style="width: 100%">
            <el-option
              v-for="o in optionList(COMPARATOR_LABELS)"
              :key="o.value"
              :label="o.label"
              :value="o.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="下限">
          <el-input
            v-model="limitForm.limit_low"
            placeholder="数值或半级编码值（3-4 级记 3.5）"
            @input="onBoundInput('limit_low', $event)"
          />
        </el-form-item>
        <el-form-item label="上限">
          <el-input
            v-model="limitForm.limit_high"
            placeholder="数值或半级编码值"
            @input="onBoundInput('limit_high', $event)"
          />
        </el-form-item>
        <el-form-item label="条款引用"
          ><el-input v-model="limitForm.clause_ref" placeholder="例如 GB 18401-2010"
        /></el-form-item>
        <el-form-item label="备注"
          ><el-input
            v-model="limitForm.remark"
            type="textarea"
            :rows="2"
            maxlength="500"
            show-word-limit
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="limitDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="saveLimit">保存</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="reqDialogVisible"
      :title="reqForm.id ? '编辑出口要求' : '登记出口要求'"
      width="620px"
      destroy-on-close
    >
      <el-form ref="reqFormRef" :model="reqForm" :rules="reqRules" label-width="110px">
        <el-form-item label="要求编码" prop="requirement_code"
          ><el-input v-model="reqForm.requirement_code" :disabled="!!reqForm.id"
        /></el-form-item>
        <el-form-item label="市场" prop="market">
          <el-select v-model="reqForm.market" style="width: 100%">
            <el-option
              v-for="o in optionList(TARGET_MARKET_LABELS)"
              :key="o.value"
              :label="o.label"
              :value="o.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="要求类型" prop="requirement_type">
          <el-select v-model="reqForm.requirement_type" style="width: 100%">
            <el-option
              v-for="o in optionList(REQUIREMENT_TYPE_LABELS)"
              :key="o.value"
              :label="o.label"
              :value="o.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="OEKO 分级">
          <el-select v-model="reqForm.oeko_tex_class" clearable style="width: 100%">
            <el-option
              v-for="o in optionList(OEKO_CLASS_LABELS)"
              :key="o.value"
              :label="o.label"
              :value="o.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="标准依据"><el-input v-model="reqForm.standard_ref" /></el-form-item>
        <el-form-item label="细则" prop="detail"
          ><el-input
            v-model="reqForm.detail"
            type="textarea"
            :rows="3"
            maxlength="1000"
            show-word-limit
        /></el-form-item>
        <el-form-item label="强制项">
          <el-switch v-model="reqForm.mandatory" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="reqDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="saveReq">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { useRoute } from 'vue-router';
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus';
import {
  createExportRequirement,
  createLimitRule,
  createTestItem,
  deleteExportRequirement,
  deleteLimitRule,
  deleteTestItem,
  listExportRequirements,
  listLimitRules,
  listTestItems,
  updateExportRequirement,
  updateLimitRule,
  updateTestItem,
  type TextileExportRequirement,
  type TextileLimitRule,
  type TextileTestItem,
} from '@/api/textile-safety';
import {
  BASIS_STANDARD_LABELS,
  CHECK_KIND_LABELS,
  COMPARATOR_LABELS,
  EXEMPTION_TAG_LABELS,
  ITEM_KIND_LABELS,
  OEKO_CLASS_LABELS,
  PRODUCT_FORM_LABELS,
  REQUIREMENT_TYPE_LABELS,
  SAFETY_CATEGORY_LABELS,
  TARGET_MARKET_LABELS,
  optionList,
} from '@/constants/textile-safety';

const route = useRoute();
const activeTab = ref((route.query.tab as string) || 'items');

const items = ref<TextileTestItem[]>([]);
const itemsLoading = ref(false);
const itemBasis = ref('');

const limits = ref<TextileLimitRule[]>([]);
const limitsLoading = ref(false);
const limitCategory = ref('');

const requirements = ref<TextileExportRequirement[]>([]);
const reqLoading = ref(false);
const reqMarket = ref('');

function itemName(id: number): string {
  const it = items.value.find(i => i.id === id);
  return it ? `${it.item_code} ${it.item_name}` : `#${id}`;
}

async function loadItems() {
  itemsLoading.value = true;
  try {
    const data = await listTestItems(itemBasis.value || undefined);
    items.value = data.items;
  } catch {
    ElMessage.error('加载检测项目主数据失败');
  } finally {
    itemsLoading.value = false;
  }
}

async function loadLimits() {
  limitsLoading.value = true;
  try {
    const data = await listLimitRules({ safety_category: limitCategory.value || undefined });
    limits.value = data.rules;
  } catch {
    ElMessage.error('加载限值判定表失败');
  } finally {
    limitsLoading.value = false;
  }
}

async function loadRequirements() {
  reqLoading.value = true;
  try {
    const data = await listExportRequirements(reqMarket.value || undefined);
    requirements.value = data.requirements;
  } catch {
    ElMessage.error('加载出口市场要求失败');
  } finally {
    reqLoading.value = false;
  }
}

const itemDialogVisible = ref(false);
const itemFormRef = ref<FormInstance>();
const itemForm = reactive({
  id: null as number | null,
  item_code: '',
  item_name: '',
  item_kind: 'numeric',
  basis_standard: 'GB18401',
  check_kind: 'measurable',
  method_standard: '',
  unit: '',
  description: '',
});
const itemRules: FormRules = {
  item_code: [{ required: true, message: '项目编码必填', trigger: 'blur' }],
  item_name: [{ required: true, message: '项目名称必填', trigger: 'blur' }],
};

function openItemDialog(row?: TextileTestItem) {
  if (row) {
    Object.assign(itemForm, {
      id: row.id,
      item_code: row.item_code,
      item_name: row.item_name,
      item_kind: row.item_kind,
      basis_standard: row.basis_standard,
      check_kind: row.check_kind,
      method_standard: row.method_standard ?? '',
      unit: row.unit ?? '',
      description: row.description,
    });
  } else {
    Object.assign(itemForm, {
      id: null,
      item_code: '',
      item_name: '',
      item_kind: 'numeric',
      basis_standard: 'GB18401',
      check_kind: 'measurable',
      method_standard: '',
      unit: '',
      description: '',
    });
  }
  itemDialogVisible.value = true;
}

async function saveItem() {
  const valid = await itemFormRef.value?.validate().catch(() => false);
  if (!valid) return;
  const body = {
    item_code: itemForm.item_code,
    item_name: itemForm.item_name,
    item_kind: itemForm.item_kind,
    basis_standard: itemForm.basis_standard,
    check_kind: itemForm.check_kind,
    method_standard: itemForm.method_standard || null,
    unit: itemForm.unit || null,
    description: itemForm.description,
  };
  try {
    if (itemForm.id) {
      await updateTestItem(itemForm.id, body);
    } else {
      await createTestItem(body);
    }
    ElMessage.success('项目主数据已保存');
    itemDialogVisible.value = false;
    await loadItems();
  } catch (e: unknown) {
    ElMessage.error(extractError(e, '保存失败'));
  }
}

async function removeItem(row: TextileTestItem) {
  try {
    await ElMessageBox.confirm(
      `删除项目 ${row.item_code}？被送检清单引用时后端会拒绝并点名原因`,
      '删除确认',
      { type: 'warning' }
    );
    await deleteTestItem(row.id);
    ElMessage.success('已删除');
    await loadItems();
  } catch (e: unknown) {
    if (e !== 'cancel') ElMessage.error(extractError(e, '删除失败'));
  }
}

const limitDialogVisible = ref(false);
const limitFormRef = ref<FormInstance>();
const limitForm = reactive({
  editId: null as number | null,
  item_code: '',
  safety_category: 'GB18401_A',
  product_form: 'any',
  condition_tag: 'none',
  comparator: 'lte',
  limit_low: '',
  limit_high: '',
  clause_ref: '',
  remark: '',
});

const numericField = (label: string) => ({
  validator: (_r: unknown, value: string, cb: (e?: Error) => void) => {
    if (value === '' || value === null || value === undefined) return cb();
    if (Number.isNaN(Number(value)))
      return cb(new Error(`${label}必须是合法数字（等级型按半级编码值录入）`));
    return cb();
  },
  trigger: 'blur',
});
const limitRules: FormRules = {
  item_code: [{ required: true, message: '必须挂真实项目', trigger: 'change' }],
  limit_low: [numericField('下限')],
  limit_high: [numericField('上限')],
};

function onBoundInput(field: 'limit_low' | 'limit_high', v: string) {
  limitForm[field] = v.replace(/[^\d.-]/g, '');
}

function openLimitDialog(row?: TextileLimitRule) {
  if (row) {
    const it = items.value.find(i => i.id === row.item_id);
    Object.assign(limitForm, {
      editId: row.id,
      item_code: it?.item_code ?? '',
      safety_category: row.safety_category,
      product_form: row.product_form,
      condition_tag: row.condition_tag,
      comparator: row.comparator,
      limit_low: row.limit_low === null ? '' : String(row.limit_low),
      limit_high: row.limit_high === null ? '' : String(row.limit_high),
      clause_ref: row.clause_ref,
      remark: row.remark,
    });
  } else {
    Object.assign(limitForm, {
      editId: null,
      item_code: '',
      safety_category: 'GB18401_A',
      product_form: 'any',
      condition_tag: 'none',
      comparator: 'lte',
      limit_low: '',
      limit_high: '',
      clause_ref: '',
      remark: '',
    });
  }
  limitDialogVisible.value = true;
}

async function saveLimit() {
  const valid = await limitFormRef.value?.validate().catch(() => false);
  if (!valid) return;
  const body = {
    item_code: limitForm.item_code,
    safety_category: limitForm.safety_category,
    product_form: limitForm.product_form,
    condition_tag: limitForm.condition_tag,
    comparator: limitForm.comparator,
    limit_low: limitForm.limit_low === '' ? null : Number(limitForm.limit_low),
    limit_high: limitForm.limit_high === '' ? null : Number(limitForm.limit_high),
    clause_ref: limitForm.clause_ref,
    remark: limitForm.remark,
  };
  try {
    if (limitForm.editId) {
      await updateLimitRule(limitForm.editId, body);
    } else {
      await createLimitRule(body);
    }
    ElMessage.success('限值行已保存（判定链即刻按新行取数）');
    limitDialogVisible.value = false;
    await loadLimits();
  } catch (e: unknown) {
    ElMessage.error(extractError(e, '保存失败'));
  }
}

async function removeLimit(row: TextileLimitRule) {
  try {
    await ElMessageBox.confirm(
      `删除 ${itemName(row.item_id)} 的这条限值行？删除后该组合判定将报"矩阵无行"`,
      '删除确认',
      { type: 'warning' }
    );
    await deleteLimitRule(row.id);
    ElMessage.success('已删除');
    await loadLimits();
  } catch (e: unknown) {
    if (e !== 'cancel') ElMessage.error(extractError(e, '删除失败'));
  }
}

const reqDialogVisible = ref(false);
const reqFormRef = ref<FormInstance>();
const reqForm = reactive({
  id: null as number | null,
  requirement_code: '',
  market: 'eu',
  requirement_type: 'oeko_tex',
  oeko_tex_class: null as string | null,
  standard_ref: '',
  detail: '',
  mandatory: true,
});
const reqRules: FormRules = {
  requirement_code: [{ required: true, message: '要求编码必填', trigger: 'blur' }],
  detail: [{ required: true, message: '细则必填', trigger: 'blur' }],
};

function openReqDialog(row?: TextileExportRequirement) {
  if (row) {
    Object.assign(reqForm, {
      id: row.id,
      requirement_code: row.requirement_code,
      market: row.market,
      requirement_type: row.requirement_type,
      oeko_tex_class: row.oeko_tex_class,
      standard_ref: row.standard_ref,
      detail: row.detail,
      mandatory: row.mandatory,
    });
  } else {
    Object.assign(reqForm, {
      id: null,
      requirement_code: '',
      market: 'eu',
      requirement_type: 'oeko_tex',
      oeko_tex_class: null,
      standard_ref: '',
      detail: '',
      mandatory: true,
    });
  }
  reqDialogVisible.value = true;
}

async function saveReq() {
  const valid = await reqFormRef.value?.validate().catch(() => false);
  if (!valid) return;
  const body = {
    requirement_code: reqForm.requirement_code,
    market: reqForm.market,
    requirement_type: reqForm.requirement_type,
    oeko_tex_class: reqForm.oeko_tex_class,
    standard_ref: reqForm.standard_ref,
    detail: reqForm.detail,
    mandatory: reqForm.mandatory,
  };
  try {
    if (reqForm.id) {
      await updateExportRequirement(reqForm.id, body);
    } else {
      await createExportRequirement(body);
    }
    ElMessage.success('出口要求已保存');
    reqDialogVisible.value = false;
    await loadRequirements();
  } catch (e: unknown) {
    ElMessage.error(extractError(e, '保存失败'));
  }
}

async function removeReq(row: TextileExportRequirement) {
  try {
    await ElMessageBox.confirm(`删除要求 ${row.requirement_code}？`, '删除确认', {
      type: 'warning',
    });
    await deleteExportRequirement(row.id);
    ElMessage.success('已删除');
    await loadRequirements();
  } catch (e: unknown) {
    if (e !== 'cancel') ElMessage.error(extractError(e, '删除失败'));
  }
}

function extractError(e: unknown, fallback: string): string {
  const msg = (e as { response?: { data?: { message?: string } } })?.response?.data?.message;
  return msg || fallback;
}

onMounted(async () => {
  await loadItems();
  await Promise.all([loadLimits(), loadRequirements()]);
});
</script>

<style scoped>
.page-header {
  margin-bottom: 12px;
}
.toolbar {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 10px;
}
</style>
