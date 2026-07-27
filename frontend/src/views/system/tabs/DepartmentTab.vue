<!--
  DepartmentTab.vue - 部门管理 Tab
  来源：原 system/index.vue 中 部门管理 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="department-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('system.department.title') }}</h2>
      <el-button type="primary" @click="openDeptDialog()">
        <el-icon><Plus /></el-icon> {{ t('system.department.button.create') }}
      </el-button>
    </div>
    <el-card shadow="hover">
      <el-table
        v-loading="deptLoading"
        :data="departments"
        stripe
        row-key="id"
        default-expand-all
        :aria-label="t('system.department.aria.list')"
      >
        <el-table-column prop="name" :label="t('system.department.column.name')" min-width="200" />
        <el-table-column prop="code" :label="t('system.department.column.code')" width="120" />
        <el-table-column
          prop="manager_name"
          :label="t('system.department.column.manager')"
          width="100"
        />
        <el-table-column
          prop="sort_order"
          :label="t('system.department.column.sort')"
          width="80"
          align="center"
        />
        <el-table-column
          prop="status"
          :label="t('system.department.column.status')"
          width="80"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="row.status === 1 ? 'success' : 'info'" size="small">
              {{ getStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('system.department.column.action')" width="150" fixed="right">
          <template #default="{ row }">
            <!-- P2-17 修复（批次 86 v2 复审）：编辑/删除按钮补齐 v-permission -->
            <el-button
              v-permission="'department:update'"
              size="small"
              link
              @click="openDeptDialog(row as Department)"
              >{{ t('system.department.button.edit') }}</el-button
            >
            <el-button
              v-permission="'department:delete'"
              size="small"
              link
              type="danger"
              @click="deleteDept(row as Department)"
              >{{ t('system.department.button.delete') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog
      v-model="deptDialogVisible"
      :title="
        deptForm.id
          ? t('system.department.dialog.editTitle')
          : t('system.department.dialog.createTitle')
      "
      width="500px"
      :aria-label="t('system.department.dialog.aria')"
    >
      <el-form
        ref="deptFormRef"
        :model="deptForm"
        :rules="deptRules"
        label-width="80px"
        :aria-label="t('system.department.form.aria')"
      >
        <el-form-item :label="t('system.department.form.label.name')" prop="name">
          <el-input v-model="deptForm.name" />
        </el-form-item>
        <el-form-item :label="t('system.department.form.label.code')" prop="code">
          <el-input v-model="deptForm.code" />
        </el-form-item>
        <el-form-item :label="t('system.department.form.label.parent')">
          <el-tree-select
            v-model="deptForm.parent_id"
            :data="departments"
            :props="{ label: 'name', value: 'id' }"
            clearable
            check-strictly
          />
        </el-form-item>
        <el-form-item :label="t('system.department.form.label.sort')">
          <el-input-number v-model="deptForm.sort_order" :min="0" />
        </el-form-item>
        <el-form-item :label="t('system.department.form.label.status')">
          <el-switch v-model="deptForm.status" :active-value="1" :inactive-value="0" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="deptDialogVisible = false">{{
          t('system.department.form.button.cancel')
        }}</el-button>
        <el-button type="primary" :loading="deptSubmitLoading" @click="submitDept">{{
          t('system.department.form.button.confirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import type { FormInstance, FormRules } from 'element-plus';
import {
  createDepartment,
  updateDepartment,
  deleteDepartment as deleteDeptApi,
  getDepartmentTree,
  type Department,
} from '@/api/department';

const { t } = useI18n({ useScope: 'global' });

// 状态标签映射（响应式）
const getStatusLabel = (status: number): string =>
  status === 1 ? t('system.department.status.enabled') : t('system.department.status.disabled');

const departments = ref<Department[]>([]);
const deptLoading = ref(false);

const fetchDepartments = async () => {
  deptLoading.value = true;
  try {
    const res = await getDepartmentTree();
    const d = res.data as { items?: Department[]; data?: Department[] } | Department[];
    departments.value =
      (d as { items?: Department[] })?.items ||
      (d as { data?: Department[] })?.data ||
      (d as Department[]) ||
      [];
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('system.department.message.fetchFailed'));
  } finally {
    deptLoading.value = false;
  }
};

defineExpose({ refresh: fetchDepartments });

const deptDialogVisible = ref(false);
const deptFormRef = ref<FormInstance>();
const deptSubmitLoading = ref(false);
const deptForm = reactive({
  id: 0,
  name: '',
  code: '',
  parent_id: undefined as number | undefined,
  sort_order: 0,
  status: 1,
});

const deptRules: FormRules = {
  name: [{ required: true, message: t('system.department.message.requiredName'), trigger: 'blur' }],
  code: [{ required: true, message: t('system.department.message.requiredCode'), trigger: 'blur' }],
};

const openDeptDialog = (row?: Department) => {
  deptFormRef.value?.resetFields();
  if (row) {
    Object.assign(deptForm, {
      id: row.id,
      name: row.name,
      code: row.code,
      parent_id: row.parent_id,
      sort_order: row.sort_order,
      status: row.status,
    });
  } else {
    Object.assign(deptForm, {
      id: 0,
      name: '',
      code: '',
      parent_id: undefined,
      sort_order: 0,
      status: 1,
    });
  }
  deptDialogVisible.value = true;
};

const submitDept = async () => {
  const valid = await deptFormRef.value?.validate();
  if (!valid) return;
  deptSubmitLoading.value = true;
  try {
    if (deptForm.id) {
      await updateDepartment(deptForm.id, {
        name: deptForm.name,
        sort_order: deptForm.sort_order,
        status: deptForm.status,
      });
      ElMessage.success(t('system.department.message.updateSuccess'));
    } else {
      await createDepartment({
        name: deptForm.name,
        code: deptForm.code,
        parent_id: deptForm.parent_id,
        sort_order: deptForm.sort_order,
      });
      ElMessage.success(t('system.department.message.createSuccess'));
    }
    deptDialogVisible.value = false;
    fetchDepartments();
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('system.department.message.operationFailed'));
  } finally {
    deptSubmitLoading.value = false;
  }
};

const deleteDept = async (row: Department) => {
  try {
    await ElMessageBox.confirm(
      t('system.department.message.deleteConfirm', { name: row.name }),
      t('system.department.message.deleteTitle'),
      { type: 'warning' }
    );
    await deleteDeptApi(row.id);
    ElMessage.success(t('system.department.message.deleteSuccess'));
    fetchDepartments();
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string };
      ElMessage.error(err.message || t('system.department.message.deleteFailed'));
    }
  }
};

onMounted(() => {
  fetchDepartments();
});
</script>
