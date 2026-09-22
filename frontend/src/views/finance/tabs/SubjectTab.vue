<!--
  SubjectTab.vue - 会计科目 Tab
  来源：原 finance/index.vue 中 科目管理 tab 内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="subject-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('finance.subjectTab.pageTitle') }}</h2>
      <div class="header-actions">
        <el-button type="primary" @click="openSubjectDialog()">
          <el-icon><Plus /></el-icon>
          {{ t('finance.subjectTab.buttonNewSubject') }}
        </el-button>
        <el-button v-permission="'finance.subject.print'" @click="handlePrintSubjects">
          <el-icon><Printer /></el-icon>
          {{ t('finance.subjectTab.buttonPrint') }}
        </el-button>
        <el-button v-permission="'finance.subject.export'" @click="handleExportSubjects">
          <el-icon><Download /></el-icon>
          {{ t('finance.subjectTab.buttonExport') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover">
      <el-table
        v-loading="subjectLoading"
        :data="subjects"
        stripe
        row-key="id"
        default-expand-all
        :aria-label="t('finance.subjectTab.ariaLabel')"
      >
        <el-table-column prop="code" :label="t('finance.subjectTab.columnCode')" width="120" />
        <el-table-column prop="name" :label="t('finance.subjectTab.columnName')" min-width="200" />
        <el-table-column
          prop="category"
          :label="t('finance.subjectTab.columnCategory')"
          width="100"
        >
          <template #default="{ row }">
            <el-tag size="small">{{ getCategoryLabel(row.category) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="direction"
          :label="t('finance.subjectTab.columnDirection')"
          width="100"
        >
          <template #default="{ row }">
            <el-tag :type="row.direction === 'debit' ? 'success' : 'danger'" size="small">
              {{
                row.direction === 'debit'
                  ? t('finance.subjectTab.directionDebit')
                  : t('finance.subjectTab.directionCredit')
              }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="level"
          :label="t('finance.subjectTab.columnLevel')"
          width="80"
          align="center"
        />
        <el-table-column
          prop="is_leaf"
          :label="t('finance.subjectTab.columnIsLeaf')"
          width="80"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="row.is_leaf ? 'success' : 'info'" size="small">
              {{ row.is_leaf ? t('finance.subjectTab.yes') : t('finance.subjectTab.no') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="status"
          :label="t('finance.subjectTab.columnStatus')"
          width="80"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="row.status === 1 ? 'success' : 'info'" size="small">
              {{
                row.status === 1
                  ? t('finance.subjectTab.statusActive')
                  : t('finance.subjectTab.statusInactive')
              }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('finance.subjectTab.columnAction')" width="150" fixed="right">
          <template #default="{ row }">
            <!-- P2-17 修复（批次 86 v2 复审）：编辑/删除按钮补齐 v-permission -->
            <el-button
              v-permission="'finance_subject:update'"
              type="primary"
              link
              size="small"
              @click="openSubjectDialog(row)"
              >{{ t('finance.subjectTab.buttonEdit') }}</el-button
            >
            <el-button
              v-permission="'finance_subject:delete'"
              type="danger"
              link
              size="small"
              @click="deleteSubject(row)"
              >{{ t('finance.subjectTab.buttonDelete') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog
      v-model="subjectDialogVisible"
      :title="
        subjectForm.id
          ? t('finance.subjectTab.dialogTitleEdit')
          : t('finance.subjectTab.dialogTitleNew')
      "
      width="500px"
      :aria-label="
        subjectForm.id
          ? t('finance.subjectTab.dialogAriaLabelEdit')
          : t('finance.subjectTab.dialogAriaLabelNew')
      "
    >
      <el-form
        ref="subjectFormRef"
        :model="subjectForm"
        :rules="subjectRules"
        label-width="80px"
        :aria-label="t('finance.subjectTab.formAriaLabel')"
      >
        <el-form-item :label="t('finance.subjectTab.labelCode')" prop="code">
          <el-input
            v-model="subjectForm.code"
            :placeholder="t('finance.subjectTab.placeholderCode')"
          />
        </el-form-item>
        <el-form-item :label="t('finance.subjectTab.labelName')" prop="name">
          <el-input
            v-model="subjectForm.name"
            :placeholder="t('finance.subjectTab.placeholderName')"
          />
        </el-form-item>
        <el-form-item :label="t('finance.subjectTab.labelParent')">
          <el-tree-select
            v-model="subjectForm.parent_id"
            :data="subjectTreeData"
            :props="{ label: 'name', value: 'id' }"
            :placeholder="t('finance.subjectTab.placeholderParent')"
            clearable
            check-strictly
          />
        </el-form-item>
        <el-form-item :label="t('finance.subjectTab.labelDirection')" prop="balance_direction">
          <el-radio-group v-model="subjectForm.balance_direction">
            <el-radio value="debit">{{ t('finance.subjectTab.directionDebit') }}</el-radio>
            <el-radio value="credit">{{ t('finance.subjectTab.directionCredit') }}</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="辅助核算">
          <el-checkbox v-model="subjectForm.assist_customer">{{
            t('finance.subjectTab.assistCustomer')
          }}</el-checkbox>
          <el-checkbox v-model="subjectForm.assist_supplier">{{
            t('finance.subjectTab.assistSupplier')
          }}</el-checkbox>
          <el-checkbox v-model="subjectForm.assist_batch">{{
            t('finance.subjectTab.assistBatch')
          }}</el-checkbox>
          <el-checkbox v-model="subjectForm.assist_color_no">{{
            t('finance.subjectTab.assistColorNo')
          }}</el-checkbox>
          <el-checkbox v-model="subjectForm.enable_dual_unit">{{
            t('finance.subjectTab.enableDualUnit')
          }}</el-checkbox>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="subjectDialogVisible = false">{{
          t('finance.subjectTab.buttonCancel')
        }}</el-button>
        <el-button type="primary" :loading="subjectSubmitLoading" @click="submitSubject">{{
          t('finance.subjectTab.buttonConfirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus, Printer, Download } from '@element-plus/icons-vue';
import printJS from 'print-js';
import type { FormInstance, FormRules } from 'element-plus';
import {
  getSubjectTree,
  createSubject,
  updateSubject,
  deleteSubject as deleteSubjectApi,
  type AccountSubject,
} from '@/api/finance';
import { exportFromBackend } from '@/utils/export';

const { t } = useI18n({ useScope: 'global' });

const subjects = ref<AccountSubject[]>([]);
const subjectLoading = ref(false);
const subjectSubmitLoading = ref(false);
const subjectDialogVisible = ref(false);
const subjectFormRef = ref<FormInstance>();

const blankSubjectForm = () => ({
  id: 0,
  code: '',
  name: '',
  parent_id: undefined as number | undefined,
  balance_direction: 'debit',
  assist_customer: false,
  assist_supplier: false,
  assist_batch: false,
  assist_color_no: false,
  enable_dual_unit: false,
});

const subjectForm = reactive(blankSubjectForm());

// 级次按上级科目派生：无上级=1 级，有上级=上级级次 +1（后端 level 必填）
const findNodeLevel = (list: AccountSubject[], id: number): number | undefined => {
  for (const node of list) {
    if (node.id === id) return node.level;
    if (node.children?.length) {
      const found = findNodeLevel(node.children, id);
      if (found !== undefined) return found;
    }
  }
  return undefined;
};
const resolveLevel = (parentId?: number): number => {
  if (parentId === undefined) return 1;
  const parentLevel = findNodeLevel(subjects.value, parentId);
  return parentLevel === undefined ? 2 : parentLevel + 1;
};

const subjectRules = computed<FormRules>(() => ({
  code: [{ required: true, message: t('finance.subjectTab.ruleCodeRequired'), trigger: 'blur' }],
  name: [{ required: true, message: t('finance.subjectTab.ruleNameRequired'), trigger: 'blur' }],
  balance_direction: [
    { required: true, message: t('finance.subjectTab.ruleDirectionRequired'), trigger: 'change' },
  ],
}));

const subjectTreeData = computed(() => subjects.value);

const getCategoryLabel = (category: string) => {
  const map: Record<string, string> = {
    asset: t('finance.subjectTab.categoryAsset'),
    liability: t('finance.subjectTab.categoryLiability'),
    equity: t('finance.subjectTab.categoryEquity'),
    cost: t('finance.subjectTab.categoryCost'),
    profit_loss: t('finance.subjectTab.categoryProfitLoss'),
  };
  return map[category] || category;
};

const fetchSubjects = async () => {
  subjectLoading.value = true;
  try {
    const res = await getSubjectTree();
    const d = res.data as AccountSubject[] | { items?: AccountSubject[]; data?: AccountSubject[] };
    subjects.value = Array.isArray(d) ? d : d?.items || d?.data || [];
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('finance.subjectTab.messageFetchFailed'));
  } finally {
    subjectLoading.value = false;
  }
};

const openSubjectDialog = (row?: AccountSubject) => {
  subjectFormRef.value?.resetFields();
  Object.assign(subjectForm, blankSubjectForm());
  if (row) {
    subjectForm.id = row.id;
    subjectForm.code = row.code;
    subjectForm.name = row.name;
    subjectForm.parent_id = row.parent_id;
    subjectForm.balance_direction = row.direction || 'debit';
    // 注意：/subjects/tree 仅返回 id/code/name/level/children，不含辅助核算位，
    // 编辑时无法回填既有辅助位，需后端补齐树字段或前端改调 GET /subjects/:id（见交付报告）。
  }
  subjectDialogVisible.value = true;
};

const submitSubject = async () => {
  const valid = await subjectFormRef.value?.validate();
  if (!valid) return;

  subjectSubmitLoading.value = true;
  try {
    if (subjectForm.id) {
      await updateSubject(subjectForm.id, {
        name: subjectForm.name,
        balance_direction: subjectForm.balance_direction,
        assist_customer: subjectForm.assist_customer,
        assist_supplier: subjectForm.assist_supplier,
        assist_batch: subjectForm.assist_batch,
        assist_color_no: subjectForm.assist_color_no,
        enable_dual_unit: subjectForm.enable_dual_unit,
      });
      ElMessage.success(t('finance.subjectTab.messageUpdateSuccess'));
    } else {
      await createSubject({
        code: subjectForm.code,
        name: subjectForm.name,
        level: resolveLevel(subjectForm.parent_id),
        parent_id: subjectForm.parent_id,
        balance_direction: subjectForm.balance_direction,
        assist_customer: subjectForm.assist_customer,
        assist_supplier: subjectForm.assist_supplier,
        assist_batch: subjectForm.assist_batch,
        assist_color_no: subjectForm.assist_color_no,
        enable_dual_unit: subjectForm.enable_dual_unit,
      });
      ElMessage.success(t('finance.subjectTab.messageCreateSuccess'));
    }
    subjectDialogVisible.value = false;
    fetchSubjects();
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('finance.subjectTab.messageOperationFailed'));
  } finally {
    subjectSubmitLoading.value = false;
  }
};

const deleteSubject = async (row: AccountSubject) => {
  try {
    await ElMessageBox.confirm(
      t('finance.subjectTab.messageDeleteConfirm', { name: row.name }),
      t('finance.subjectTab.messageDeleteTitle'),
      { type: 'warning' }
    );
    await deleteSubjectApi(row.id);
    ElMessage.success(t('finance.subjectTab.messageDeleteSuccess'));
    fetchSubjects();
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error;
      ElMessage.error(err.message || t('finance.subjectTab.messageDeleteFailed'));
    }
  }
};

const handlePrintSubjects = () => {
  const printData = subjects.value.map((item, index) => ({
    [t('finance.subjectTab.exportColIndex')]: index + 1,
    [t('finance.subjectTab.exportColCode')]: item.code,
    [t('finance.subjectTab.exportColName')]: item.name,
    [t('finance.subjectTab.exportColCategory')]: getCategoryLabel(item.category),
    [t('finance.subjectTab.exportColDirection')]:
      item.direction === 'debit'
        ? t('finance.subjectTab.directionDebit')
        : t('finance.subjectTab.directionCredit'),
    [t('finance.subjectTab.exportColLevel')]: `L${item.level}`,
  }));
  printJS({
    printable: printData,
    properties: Object.keys(printData[0] || {}) as string[],
    type: 'json',
    header: t('finance.subjectTab.printHeader'),
    style: 'padding: 20px; font-size: 14px;',
    headerStyle: 'font-size: 18px; font-weight: bold; margin-bottom: 20px;',
    gridHeaderStyle: 'font-weight: bold; background-color: #f5f7fa;',
    gridStyle: 'border-collapse: collapse; width: 100%;',
  } as never);
};

const handleExportSubjects = () => {
  exportFromBackend('/subjects/export', {}, t('finance.subjectTab.exportFilename'));
};

onMounted(() => {
  fetchSubjects();
});
</script>
