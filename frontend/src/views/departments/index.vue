<template>
  <div class="departments-page">
    <div class="header">
      <h2>{{ t('departments.index.pageTitle') }}</h2>
      <el-button type="primary" @click="handleCreate">{{
        t('departments.index.buttonCreate')
      }}</el-button>
    </div>

    <el-table
      v-loading="loading"
      :data="deptTreeData"
      border
      row-key="id"
      default-expand-all
      :tree-props="{ children: 'children' }"
      :aria-label="t('departments.index.ariaTable')"
    >
      <el-table-column prop="name" :label="t('departments.index.colName')" />
      <el-table-column prop="code" :label="t('departments.index.colCode')" />
      <el-table-column :label="t('departments.index.colParent')">
        <template #default="{ row }">
          {{ getParentName(row.parent_id) }}
        </template>
      </el-table-column>
      <el-table-column prop="manager_name" :label="t('departments.index.colManager')" />
      <el-table-column prop="sort_order" :label="t('departments.index.colSortOrder')" width="80" />
      <el-table-column :label="t('departments.index.colStatus')">
        <template #default="{ row }">
          <el-tag :type="row.is_active ? 'success' : 'danger'">
            {{
              row.is_active
                ? t('departments.index.optionEnabled')
                : t('departments.index.optionDisabled')
            }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('departments.index.colOperation')" width="200">
        <template #default="{ row }">
          <el-button
            v-permission="PERMISSIONS.DEPARTMENT_UPDATE"
            size="small"
            @click="handleEdit(row as DepartmentTreeNode)"
            >{{ t('departments.index.buttonEdit') }}</el-button
          >
          <el-button
            v-permission="PERMISSIONS.DEPARTMENT_DELETE"
            size="small"
            type="danger"
            @click="handleDelete(row as DepartmentTreeNode)"
            >{{ t('departments.index.buttonDelete') }}</el-button
          >
        </template>
      </el-table-column>
    </el-table>

    <!-- 新建/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="
        dialogMode === 'create'
          ? t('departments.index.titleCreate')
          : t('departments.index.titleEdit')
      "
      width="600px"
      :aria-label="
        dialogMode === 'create'
          ? t('departments.index.ariaCreateDialog')
          : t('departments.index.ariaEditDialog')
      "
      @close="handleDialogClose"
    >
      <el-form
        ref="formRef"
        :model="formData"
        :rules="formRules"
        label-width="100px"
        :aria-label="t('departments.index.ariaForm')"
      >
        <el-form-item :label="t('departments.index.colName')" prop="name">
          <el-input v-model="formData.name" :placeholder="t('departments.index.placeholderName')" />
        </el-form-item>
        <el-form-item :label="t('departments.index.colCode')" prop="code">
          <el-input v-model="formData.code" :placeholder="t('departments.index.placeholderCode')" />
        </el-form-item>
        <el-form-item :label="t('departments.index.colParent')" prop="parent_id">
          <el-tree-select
            v-model="formData.parent_id"
            :data="deptTreeData"
            :props="{ label: 'name', value: 'id' }"
            :placeholder="t('departments.index.placeholderParent')"
            clearable
            check-strictly
          />
        </el-form-item>
        <el-form-item :label="t('departments.index.colManager')" prop="manager_name">
          <el-input
            v-model="formData.manager_name"
            :placeholder="t('departments.index.placeholderManager')"
          />
        </el-form-item>
        <el-form-item :label="t('departments.index.colSortOrder')" prop="sort_order">
          <NumericInput v-model="formData.sort_order" :min="0" />
        </el-form-item>
        <el-form-item :label="t('departments.index.colStatus')" prop="is_active">
          <el-select
            v-model="formData.is_active"
            :placeholder="t('departments.index.placeholderStatus')"
          >
            <el-option :label="t('departments.index.optionEnabled')" :value="true" />
            <el-option :label="t('departments.index.optionDisabled')" :value="false" />
          </el-select>
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="dialogVisible = false">{{
          t('departments.index.buttonCancel')
        }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">
          {{ t('departments.index.buttonConfirm') }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { isDialogDismissal } from '@/utils/monitor';
import NumericInput from '@/components/NumericInput.vue';
import { PERMISSIONS } from '@/constants/permissions';
import { ref, onMounted, reactive, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import {
  getDepartment,
  createDepartment,
  updateDepartment,
  deleteDepartment,
  getDepartmentTree,
  type Department,
  type DepartmentTreeNode,
  type DepartmentCreateRequest,
  type DepartmentUpdateRequest,
} from '@/api/department';

const { t } = useI18n({ useScope: 'global' });

const loading = ref(false);
const submitLoading = ref(false);
const dialogVisible = ref(false);
const dialogMode = ref<'create' | 'edit'>('create');
const formRef = ref<FormInstance>();
// 表格数据源与上级选择器共用同一份后端树出参：DepartmentTreeNode 自带 children 与
// code/sort_order/is_active，同级先后由后端按 sort_order,id 定序，前端不再组装也不再排序
const deptTreeData = ref<DepartmentTreeNode[]>([]);
// id → name 的全量映射（从树扁平化一次得到），供"上级"列取名：树形下按 parent_id 查父级名
// 不能只查当前层，故必须在扁平映射上查
const deptNameById = computed<Map<number, string>>(() => {
  const map = new Map<number, string>();
  const walk = (nodes: DepartmentTreeNode[]) => {
    for (const node of nodes) {
      map.set(node.id, node.name);
      if (node.children.length > 0) walk(node.children);
    }
  };
  walk(deptTreeData.value);
  return map;
});

// v11 批次 170 P2-1 修复：reactive<any> 改为具体类型
interface DeptFormData {
  id: number | null;
  name: string;
  code: string;
  // 可空外键：el-tree-select clearable 清空后回写 undefined；
  // 提交时归一化为显式 null（后端三态：null=清空父级，键缺席=保持原值）
  parent_id: number | null | undefined;
  manager_name: string;
  sort_order: number;
  is_active: boolean;
}

const formData = reactive<DeptFormData>({
  id: null,
  name: '',
  code: '',
  parent_id: undefined,
  manager_name: '',
  sort_order: 0,
  is_active: true,
});

const formRules: FormRules = {
  name: [{ required: true, message: t('departments.index.ruleNameRequired'), trigger: 'blur' }],
  code: [{ required: true, message: t('departments.index.ruleCodeRequired'), trigger: 'blur' }],
  is_active: [
    { required: true, message: t('departments.index.ruleStatusRequired'), trigger: 'change' },
  ],
};

const loadDepartments = async () => {
  loading.value = true;
  try {
    // 后端 get_department_tree 返回 Vec<DepartmentTreeNode>（data 为裸数组）：一次取全量，
    // 出参已含 code/sort_order/is_active 与 children，父级缺席或成环的部门由后端按根节点补回并记 WARN
    const treeRes = await getDepartmentTree();
    deptTreeData.value = treeRes.data;
  } catch (error: unknown) {
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) ||
        t('departments.index.messageLoadListFailed')
    );
  } finally {
    loading.value = false;
  }
};

const getParentName = (parentId: number | null): string => {
  // 顶级部门 parent_id 为 null：如实显示"无上级"
  if (parentId == null) return t('departments.index.placeholderNoParent');
  const parentName = deptNameById.value.get(parentId);
  // 上级 id 存在但不在后端树出参的任何一层（数据悬挂外键，或父级成环后该子树未被回收）：
  // 显示"未在部门档案中"这一可区分的真实语义，绝不复用"无上级"——那会把确有父级的部门谎报成顶级部门
  return parentName ?? t('departments.index.parentNotInArchive');
};

const handleCreate = () => {
  dialogMode.value = 'create';
  Object.assign(formData, {
    id: null,
    name: '',
    code: '',
    parent_id: undefined,
    manager_name: '',
    sort_order: 0,
    is_active: true,
  });
  dialogVisible.value = true;
};

const handleEdit = async (row: DepartmentTreeNode) => {
  dialogMode.value = 'edit';
  // 编辑初值唯一真源 = 详情端点（department::Model 全列出参）：以 row.id 拉全量详情回填，
  // 不从列表/树行载体直读字段，避免行载体缺项时改动静默丢失；取数失败显式报错并中止打开
  let detail: Department;
  try {
    detail = (await getDepartment(row.id)).data;
  } catch (error: unknown) {
    ElMessage.error(
      (error instanceof Error ? error.message : String(error)) ||
        t('departments.index.messageLoadDetailFailed')
    );
    return;
  }
  Object.assign(formData, {
    id: detail.id,
    name: detail.name,
    code: detail.code,
    // parent_id 为可空外键：详情出参 null 归一为 tree-select 空态 undefined
    parent_id: detail.parent_id ?? undefined,
    // manager_name 为后端回填展示瞬态字段：未指派为 null，表单绑 string 归一为空串
    manager_name: detail.manager_name ?? '',
    sort_order: detail.sort_order,
    is_active: detail.is_active,
  });
  dialogVisible.value = true;
};

const handleDelete = async (row: DepartmentTreeNode) => {
  if (!row.id) return;

  try {
    await ElMessageBox.confirm(
      t('departments.index.messageConfirmDelete', { name: row.name }),
      t('departments.index.titleDeleteConfirm'),
      { type: 'warning' }
    );
    await deleteDepartment(row.id);
    ElMessage.success(t('departments.index.messageDeleteSuccess'));
    await loadDepartments();
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    if (!isDialogDismissal(error)) {
      ElMessage.error(
        (error instanceof Error ? error.message : String(error)) ||
          t('departments.index.messageDeleteFailed')
      );
    }
  }
};

const handleSubmit = async () => {
  if (!formRef.value) return;

  await formRef.value.validate(async (valid: boolean) => {
    if (!valid) return;

    submitLoading.value = true;
    try {
      if (dialogMode.value === 'create') {
        // 载荷按 DepartmentCreateRequest 键集显式构造：id/manager_name/is_active 非创建 DTO
        // 字段，整表单直传只会被 serde 静默丢弃（manager_name 为后端回填展示字段，禁提交）
        const payload: DepartmentCreateRequest = {
          name: formData.name,
          code: formData.code,
          sort_order: formData.sort_order,
        };
        if (formData.parent_id != null) payload.parent_id = formData.parent_id;
        await createDepartment(payload);
        ElMessage.success(t('departments.index.messageCreateSuccess'));
      } else {
        // 三态语义（后端 UpdateDepartmentRequest DoubleOption）：
        // parent_id 清空须送显式 null（=脱离父级），undefined（tree-select 清空态）归一为 null；
        // NOT NULL 列 name/code/sort_order/is_active 恒覆盖；description/manager_id
        // 对话框未采集 → 省略键=保持原值
        const payload: DepartmentUpdateRequest = {
          name: formData.name,
          code: formData.code,
          parent_id: formData.parent_id ?? null,
          sort_order: formData.sort_order,
          is_active: formData.is_active,
        };
        await updateDepartment(formData.id!, payload);
        ElMessage.success(t('departments.index.messageUpdateSuccess'));
      }
      dialogVisible.value = false;
      await loadDepartments();
    } catch (error) {
      ElMessage.error(
        dialogMode.value === 'create'
          ? t('departments.index.messageCreateFailed')
          : t('departments.index.messageUpdateFailed')
      );
    } finally {
      submitLoading.value = false;
    }
  });
};

const handleDialogClose = () => {
  formRef.value?.resetFields();
};

onMounted(() => {
  loadDepartments();
});
</script>

<style scoped>
.departments-page {
  padding: 20px;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
</style>
