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
        <el-table-column
          prop="manager_name"
          :label="t('system.department.column.manager')"
          width="120"
        />
        <el-table-column :label="t('system.department.column.action')" width="150" fixed="right">
          <template #default="{ row }">
            <!-- 编辑/删除按钮权限门控；行载体为树节点（六键），行上不可得的字段由编辑时详情端点补齐 -->
            <el-button
              v-permission="'department:update'"
              size="small"
              link
              @click="openDeptDialog(row as DepartmentTreeNode)"
              >{{ t('system.department.button.edit') }}</el-button
            >
            <el-button
              v-permission="'department:delete'"
              size="small"
              link
              type="danger"
              @click="deleteDept(row as DepartmentTreeNode)"
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
          <el-switch v-model="deptForm.is_active" />
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
import { isDialogDismissal } from '@/utils/monitor';
import { ref, reactive, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import type { FormInstance, FormRules } from 'element-plus';
import {
  createDepartment,
  updateDepartment,
  deleteDepartment as deleteDeptApi,
  getDepartment,
  getDepartmentTree,
  type Department,
  type DepartmentTreeNode,
} from '@/api/department';

const { t } = useI18n({ useScope: 'global' });

// 行载体 = 后端 DepartmentTreeNode（id/name/description/parent_id/manager_name/children 六键）；
// 树出参不含 code/sort_order/is_active，表格不展示无载体字段，编辑初值经详情端点补真源
const departments = ref<DepartmentTreeNode[]>([]);
const deptLoading = ref(false);

const fetchDepartments = async () => {
  deptLoading.value = true;
  try {
    const res = await getDepartmentTree();
    // 后端树端点出参 data 恒为裸数组（ApiResponse<Vec<DepartmentTreeNode>>），无分页信封形态
    departments.value = res.data;
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
  is_active: true,
});

const deptRules: FormRules = {
  name: [{ required: true, message: t('system.department.message.requiredName'), trigger: 'blur' }],
  code: [{ required: true, message: t('system.department.message.requiredCode'), trigger: 'blur' }],
};

const openDeptDialog = async (row?: DepartmentTreeNode) => {
  deptFormRef.value?.resetFields();
  if (row) {
    // 树节点六键不含 code/sort_order/is_active：编辑初值唯一真源 = 详情端点
    // （department::Model 全列出参）。读取失败显式报错并中止打开，禁止空表单伪装可编辑
    let detail: Department;
    try {
      detail = (await getDepartment(row.id)).data;
    } catch (e) {
      const err = e as { message?: string };
      ElMessage.error(err.message || t('system.department.message.fetchFailed'));
      return;
    }
    Object.assign(deptForm, {
      id: detail.id,
      name: detail.name,
      code: detail.code,
      parent_id: detail.parent_id ?? undefined,
      sort_order: detail.sort_order,
      is_active: detail.is_active,
    });
  } else {
    Object.assign(deptForm, {
      id: 0,
      name: '',
      code: '',
      parent_id: undefined,
      sort_order: 0,
      is_active: true,
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
        // 编码输入框编辑态可改（后端 UpdateDepartmentRequest.code 已支持，查重在服务层）：
        // 不随 payload 提交则改动被 serde 静默丢弃
        code: deptForm.code,
        // 上级下拉可改可清空：三态语义下清空须送显式 null（=脱离父级成顶级），
        // undefined（tree-select 清空态）归一为 null；原实现漏送 parent_id ⇒ 改父静默丢失
        parent_id: deptForm.parent_id ?? null,
        sort_order: deptForm.sort_order,
        is_active: deptForm.is_active,
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

const deleteDept = async (row: DepartmentTreeNode) => {
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
    if (!isDialogDismissal(e)) {
      const err = e as { message?: string };
      ElMessage.error(err.message || t('system.department.message.deleteFailed'));
    }
  }
};

onMounted(() => {
  fetchDepartments();
});
</script>
