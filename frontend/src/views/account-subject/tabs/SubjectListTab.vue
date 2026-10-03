<!--
  SubjectListTab.vue - 会计科目 Tab
  来源：原 accountSubject/index.vue 主体内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="subject-list-tab">
    <div class="page-header">
      <h2 class="page-title">{{ $t('accountSubject.title') }}</h2>
      <div>
        <el-button type="primary" @click="openDialog()">
          <el-icon><Plus /></el-icon>{{ $t('accountSubject.create') }}
        </el-button>
        <el-button v-permission="'account_subject.export'" @click="handleExport">
          <el-icon><Download /></el-icon>{{ $t('accountSubject.export') }}
        </el-button>
      </div>
    </div>
    <el-card shadow="hover" class="filter-card">
      <el-form
        :inline="true"
        :model="queryForm"
        :aria-label="$t('accountSubject.filter.ariaLabel')"
      >
        <el-form-item :label="$t('accountSubject.filter.code')">
          <el-input
            v-model="queryForm.code"
            :placeholder="$t('accountSubject.filter.codePlaceholder')"
            clearable
          />
        </el-form-item>
        <el-form-item :label="$t('accountSubject.filter.name')">
          <el-input
            v-model="queryForm.name"
            :placeholder="$t('accountSubject.filter.namePlaceholder')"
            clearable
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleSearch">{{
            $t('accountSubject.filter.query')
          }}</el-button>
          <el-button @click="handleReset">{{ $t('accountSubject.filter.reset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover">
      <el-table
        v-loading="loading"
        :data="filteredSubjects"
        stripe
        row-key="id"
        default-expand-all
        :tree-props="{ children: 'children' }"
        :aria-label="$t('accountSubject.table.ariaLabel')"
      >
        <el-table-column prop="code" :label="$t('accountSubject.table.code')" width="120" />
        <el-table-column prop="name" :label="$t('accountSubject.table.name')" min-width="200" />
        <el-table-column
          prop="balance_direction"
          :label="$t('accountSubject.table.balanceType')"
          width="100"
        >
          <template #default="{ row }">
            <el-tag
              v-if="row.balance_direction === 'debit' || row.balance_direction === 'credit'"
              :type="row.balance_direction === 'debit' ? 'success' : 'danger'"
              size="small"
            >
              {{
                row.balance_direction === 'debit'
                  ? $t('accountSubject.balanceType.debit')
                  : $t('accountSubject.balanceType.credit')
              }}
            </el-tag>
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column
          prop="level"
          :label="$t('accountSubject.table.level')"
          width="80"
          align="center"
        />
        <el-table-column
          prop="status"
          :label="$t('accountSubject.table.status')"
          width="80"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{
                row.status === 'active'
                  ? $t('accountSubject.status.enabled')
                  : $t('accountSubject.status.disabled')
              }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="$t('accountSubject.table.operation')" width="160" fixed="right">
          <template #default="{ row }">
            <el-button
              v-permission="'account_subject:update'"
              type="primary"
              link
              size="small"
              @click="openDialog(row)"
              >{{ $t('accountSubject.table.edit') }}</el-button
            >
            <el-button
              v-permission="'account_subject:delete'"
              type="danger"
              link
              size="small"
              @click="deleteSubject(row)"
              >{{ $t('accountSubject.table.delete') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog
      v-model="dialogVisible"
      :title="
        form.id ? $t('accountSubject.dialog.editTitle') : $t('accountSubject.dialog.createTitle')
      "
      width="500px"
      :aria-label="
        form.id
          ? $t('accountSubject.dialog.editAriaLabel')
          : $t('accountSubject.dialog.createAriaLabel')
      "
    >
      <el-form
        ref="formRef"
        :model="form"
        :rules="rules"
        label-width="120px"
        :aria-label="$t('accountSubject.dialog.ariaLabel')"
      >
        <el-form-item :label="$t('accountSubject.filter.code')" prop="code">
          <el-input v-model="form.code" :disabled="!!form.id" />
        </el-form-item>
        <el-form-item :label="$t('accountSubject.filter.name')" prop="name">
          <el-input v-model="form.name" />
        </el-form-item>
        <el-form-item :label="$t('accountSubject.dialog.parentSubject')">
          <el-tree-select
            v-model="form.parent_id"
            :data="parentSubjectOptions"
            :props="{ label: 'name', value: 'id' }"
            :placeholder="$t('accountSubject.dialog.parentPlaceholder')"
            clearable
            check-strictly
          />
        </el-form-item>
        <el-form-item :label="$t('accountSubject.table.balanceType')" prop="balance_direction">
          <el-radio-group v-model="form.balance_direction">
            <el-radio value="debit">{{ $t('accountSubject.balanceType.debit') }}</el-radio>
            <el-radio value="credit">{{ $t('accountSubject.balanceType.credit') }}</el-radio>
          </el-radio-group>
        </el-form-item>
        <!-- 辅助核算开关：CreateSubjectRequestDto 带 serde(default)=false 可省略，
             UpdateSubjectRequestDto 则为非 Option 必填 —— 更新必须回传当前真实值，
             编辑态由列表行数据带入（Object.assign），不允许用 false 兜底覆盖库值 -->
        <el-form-item :label="$t('accountSubject.dialog.assistCustomer')">
          <el-switch v-model="form.assist_customer" />
        </el-form-item>
        <el-form-item :label="$t('accountSubject.dialog.assistSupplier')">
          <el-switch v-model="form.assist_supplier" />
        </el-form-item>
        <el-form-item :label="$t('accountSubject.dialog.assistBatch')">
          <el-switch v-model="form.assist_batch" />
        </el-form-item>
        <el-form-item :label="$t('accountSubject.dialog.assistColorNo')">
          <el-switch v-model="form.assist_color_no" />
        </el-form-item>
        <el-form-item :label="$t('accountSubject.dialog.dualUnit')">
          <el-switch v-model="form.enable_dual_unit" />
        </el-form-item>
        <!-- status（active/inactive）仅更新端点支持；创建端点 DTO 无该字段，创建态隐藏 -->
        <el-form-item v-if="form.id" :label="$t('accountSubject.dialog.enable')">
          <el-switch v-model="form.status" :active-value="'active'" :inactive-value="'inactive'" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{
          $t('accountSubject.dialog.cancel')
        }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
          $t('accountSubject.dialog.confirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus';
import { Plus, Download } from '@element-plus/icons-vue';
import {
  getAccountSubjectList,
  createAccountSubject,
  updateAccountSubject,
  deleteAccountSubject,
  type AccountSubjectEntity,
  type CreateSubjectPayload,
  type UpdateSubjectPayload,
} from '@/api/account-subject';
import { logger } from '@/utils/logger';
import { exportFromBackend } from '@/utils/export';

const { t } = useI18n({ useScope: 'global' });

const loading = ref(false);
const submitLoading = ref(false);
const dialogVisible = ref(false);
const subjectList = ref<AccountSubjectEntity[]>([]);
const formRef = ref<FormInstance>();

// 后端 SubjectQuery 只读 level/parent_id/status/keyword，无 code/name/category 参数，
// 原「编码/名称/类别」筛选是发送即被忽略的假筛选：请求侧不再携带，编码/名称改为本地过滤。
// 科目类别在 account_subjects 表无对应列，类别筛选控件一并移除。
const queryForm = reactive({
  code: '',
  name: '',
});

/**
 * 表单模型：仅采集后端 Create/Update DTO 真实存在的字段。
 * 原 category / type / description 采集项在 account_subjects 表与两个 DTO 中均无对应列，
 * 提交即被 serde 丢弃（假保存），已从对话框移除。
 */
interface SubjectForm {
  id?: number;
  code: string;
  name: string;
  parent_id?: number;
  balance_direction: string;
  assist_customer: boolean;
  assist_supplier: boolean;
  assist_batch: boolean;
  assist_color_no: boolean;
  enable_dual_unit: boolean;
  status: string;
}

const form = reactive<SubjectForm>({
  id: undefined,
  code: '',
  name: '',
  parent_id: undefined,
  balance_direction: 'debit',
  assist_customer: false,
  assist_supplier: false,
  assist_batch: false,
  assist_color_no: false,
  enable_dual_unit: false,
  status: 'active',
});

const rules = computed<FormRules>(() => ({
  code: [{ required: true, message: t('accountSubject.validation.codeRequired'), trigger: 'blur' }],
  name: [{ required: true, message: t('accountSubject.validation.nameRequired'), trigger: 'blur' }],
  balance_direction: [
    {
      required: true,
      message: t('accountSubject.validation.balanceTypeRequired'),
      trigger: 'change',
    },
  ],
}));

const parentSubjectOptions = computed(() => subjectList.value);

// 后端 GET /subjects 无 code/name/category 查询实现（SubjectQuery 只有
// level/parent_id/status/keyword）：编码/名称筛选在前端本地完成，不随请求发送假参数。
const filteredSubjects = computed(() => {
  let list = subjectList.value;
  if (queryForm.code) {
    list = list.filter(s => s.code.includes(queryForm.code));
  }
  if (queryForm.name) {
    list = list.filter(s => s.name.includes(queryForm.name));
  }
  return list;
});

const fetchSubjects = async () => {
  loading.value = true;
  try {
    const res = await getAccountSubjectList();
    const d = (res as { data?: unknown }).data as
      | AccountSubjectEntity[]
      | {
          items?: AccountSubjectEntity[];
          data?: AccountSubjectEntity[];
          list?: AccountSubjectEntity[];
        };
    if (Array.isArray(d)) {
      subjectList.value = d;
    } else {
      subjectList.value = d?.items || d?.data || d?.list || [];
    }
  } catch (e) {
    const err = e as Error;
    ElMessage.error(err.message || t('accountSubject.message.fetchListFailed'));
  } finally {
    loading.value = false;
  }
};

const handleSearch = () => {
  fetchSubjects();
};

const handleReset = () => {
  queryForm.code = '';
  queryForm.name = '';
  fetchSubjects();
};

const openDialog = (row?: AccountSubjectEntity) => {
  formRef.value?.resetFields();
  if (row) {
    // 编辑：assist 开关与 enable_dual_unit 必须是库中当前真实值（PUT 必填，兜底 false 会翻转数据）
    form.id = row.id;
    form.code = row.code;
    form.name = row.name;
    form.parent_id = row.parent_id ?? undefined;
    form.balance_direction = row.balance_direction ?? 'debit';
    form.assist_customer = row.assist_customer;
    form.assist_supplier = row.assist_supplier;
    form.assist_batch = row.assist_batch;
    form.assist_color_no = row.assist_color_no;
    form.enable_dual_unit = row.enable_dual_unit;
    form.status = row.status;
  } else {
    form.id = undefined;
    form.code = '';
    form.name = '';
    form.parent_id = undefined;
    form.balance_direction = 'debit';
    form.assist_customer = false;
    form.assist_supplier = false;
    form.assist_batch = false;
    form.assist_color_no = false;
    form.enable_dual_unit = false;
    form.status = 'active';
  }
  dialogVisible.value = true;
};

/** level 为创建端点必填：按所选上级科目推导（顶级=1，子级=父级+1），非硬编码 */
const resolveLevel = (): number => {
  if (!form.parent_id) return 1;
  const parent = subjectList.value.find(s => s.id === form.parent_id);
  if (!parent) {
    // 上级科目数据不在已加载列表中：拒绝提交而非猜层级
    throw new Error(t('accountSubject.message.parentNotFound'));
  }
  return parent.level + 1;
};

const handleSubmit = async () => {
  if (!formRef.value) return;
  await formRef.value.validate(async valid => {
    if (!valid) return;
    submitLoading.value = true;
    try {
      if (form.id) {
        const payload: UpdateSubjectPayload = {
          name: form.name,
          // Option<String>：未选择方向时省略键，空串会写入 DB 的 Option 字段
          ...(form.balance_direction ? { balance_direction: form.balance_direction } : {}),
          assist_customer: form.assist_customer,
          assist_supplier: form.assist_supplier,
          assist_batch: form.assist_batch,
          assist_color_no: form.assist_color_no,
          enable_dual_unit: form.enable_dual_unit,
          status: form.status,
        };
        await updateAccountSubject(form.id, payload);
        ElMessage.success(t('accountSubject.message.updateSuccess'));
      } else {
        const payload: CreateSubjectPayload = {
          code: form.code,
          name: form.name,
          level: resolveLevel(),
          ...(form.parent_id ? { parent_id: form.parent_id } : {}),
          ...(form.balance_direction ? { balance_direction: form.balance_direction } : {}),
          assist_customer: form.assist_customer,
          assist_supplier: form.assist_supplier,
          assist_batch: form.assist_batch,
          assist_color_no: form.assist_color_no,
          enable_dual_unit: form.enable_dual_unit,
        };
        await createAccountSubject(payload);
        ElMessage.success(t('accountSubject.message.createSuccess'));
      }
      dialogVisible.value = false;
      fetchSubjects();
    } catch (e) {
      const err = e as Error;
      ElMessage.error(err.message || t('accountSubject.message.operationFailed'));
    } finally {
      submitLoading.value = false;
    }
  });
};

const deleteSubject = async (row: AccountSubjectEntity) => {
  try {
    await ElMessageBox.confirm(
      t('accountSubject.message.deleteConfirm', { name: row.name }),
      t('accountSubject.message.deleteConfirmTitle'),
      { type: 'warning' }
    );
    await deleteAccountSubject(row.id);
    ElMessage.success(t('accountSubject.message.deleteSuccess'));
    fetchSubjects();
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error;
      ElMessage.error(err.message || t('accountSubject.message.deleteFailed'));
    }
  }
};

const handleExport = () => {
  exportFromBackend('/subjects/export', {}, t('accountSubject.exportFile.filename'));
  logger.info(t('accountSubject.exportedLog'));
};

onMounted(() => {
  fetchSubjects();
});
</script>
