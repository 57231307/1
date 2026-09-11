<!--
  RoleTab.vue - 角色管理 Tab
  来源：原 system/index.vue 第 115-148 行（template）+ 900-1026 行（script）
  拆分日期：2026-06-05
  说明：本文件由 system/index.vue 拆分而来，逻辑完整可独立运行
-->
<template>
  <div class="role-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('system.role.title') }}</h2>
      <el-button type="primary" @click="openRoleDialog()">
        <el-icon><Plus /></el-icon> {{ t('system.role.button.create') }}
      </el-button>
    </div>
    <el-card shadow="hover">
      <el-table
        v-loading="roleLoading"
        :data="roles"
        stripe
        :aria-label="t('system.role.aria.list')"
      >
        <el-table-column prop="name" :label="t('system.role.column.name')" width="150" />
        <el-table-column prop="code" :label="t('system.role.column.code')" width="150" />
        <el-table-column
          prop="description"
          :label="t('system.role.column.description')"
          min-width="200"
        />
        <!-- 角色无启用/停用语义（后端 roles 表无 status 列），展示系统内置标识 -->
        <el-table-column
          prop="is_system"
          :label="t('system.role.column.status')"
          width="80"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="row.is_system ? 'success' : 'info'" size="small">
              {{ row.is_system ? t('system.role.status.system') : t('system.role.status.custom') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_at" :label="t('system.role.column.createdAt')" width="160" />
        <el-table-column :label="t('system.role.column.action')" width="200" fixed="right">
          <template #default="{ row }">
            <!-- P2-17 修复（批次 86 v2 复审）：编辑/删除按钮补齐 v-permission -->
            <el-button
              v-permission="'role:update'"
              size="small"
              link
              @click="openRoleDialog(row as Role)"
              >{{ t('system.role.button.edit') }}</el-button
            >
            <el-button size="small" link @click="openPermissionDialog(row as Role)">{{
              t('system.role.button.permission')
            }}</el-button>
            <el-button
              v-permission="'role:delete'"
              size="small"
              link
              type="danger"
              @click="deleteRole(row as Role)"
              >{{ t('system.role.button.delete') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 角色编辑对话框 -->
    <el-dialog
      v-model="roleDialogVisible"
      :title="roleForm.id ? t('system.role.dialog.editTitle') : t('system.role.dialog.createTitle')"
      width="600px"
      :aria-label="t('system.role.dialog.aria')"
    >
      <el-form
        ref="roleFormRef"
        :model="roleForm"
        :rules="roleRules"
        label-width="100px"
        :aria-label="t('system.role.form.aria')"
      >
        <el-form-item :label="t('system.role.form.label.name')" prop="name">
          <el-input v-model="roleForm.name" :disabled="!!roleForm.id" />
        </el-form-item>
        <el-form-item :label="t('system.role.form.label.code')" prop="code">
          <el-input v-model="roleForm.code" :disabled="!!roleForm.id" />
        </el-form-item>
        <el-form-item :label="t('system.role.form.label.description')" prop="description">
          <el-input v-model="roleForm.description" type="textarea" :rows="3" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="roleDialogVisible = false">{{
          t('system.role.form.button.cancel')
        }}</el-button>
        <el-button type="primary" :loading="roleSubmitLoading" @click="submitRole">{{
          t('system.role.form.button.confirm')
        }}</el-button>
      </template>
    </el-dialog>

    <!-- 权限配置对话框 -->
    <el-dialog
      v-model="permissionDialogVisible"
      :title="t('system.role.permissionDialog.title', { name: currentRoleName })"
      width="600px"
      :aria-label="t('system.role.permissionDialog.aria')"
    >
      <el-card v-loading="permissionLoading">
        <el-tree
          :data="permissionTree"
          :props="{ label: 'label', children: 'children' }"
          show-checkbox
          node-key="key"
          :default-checked-keys="checkedKeys"
          @check="handlePermissionCheck"
        />
      </el-card>
      <template #footer>
        <el-button @click="permissionDialogVisible = false">{{
          t('system.role.permissionDialog.button.cancel')
        }}</el-button>
        <el-button type="primary" :loading="permissionSubmitLoading" @click="submitPermissions">{{
          t('system.role.permissionDialog.button.save')
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
  getRoleList,
  createRole,
  updateRole,
  deleteRole as deleteRoleApi,
  getRolePermissions,
  assignPermission,
  deletePermission,
  getPermissionList,
  type Role,
  type Permission,
} from '@/api/role';

const { t } = useI18n({ useScope: 'global' });

const roles = ref<Role[]>([]);
const roleLoading = ref(false);

const fetchRoles = async () => {
  roleLoading.value = true;
  try {
    const res = await getRoleList();
    // 后端 list_roles 真实结构为 data.roles[]（RoleListResponse），兼容 items/data/裸数组形态
    const d = res.data as
      | { roles?: Role[]; items?: Role[]; data?: Role[] }
      | Role[]
      | undefined;
    roles.value = (Array.isArray(d) ? d : d?.roles || d?.items || d?.data || []) as Role[];
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) || t('system.role.message.fetchFailed')
    );
  } finally {
    roleLoading.value = false;
  }
};

defineExpose({ refresh: fetchRoles });

const roleDialogVisible = ref(false);
const roleFormRef = ref<FormInstance>();
const roleSubmitLoading = ref(false);
const roleForm = reactive({
  id: 0,
  name: '',
  code: '',
  description: '',
});

const roleRules: FormRules = {
  name: [{ required: true, message: t('system.role.message.requiredName'), trigger: 'blur' }],
  code: [{ required: true, message: t('system.role.message.requiredCode'), trigger: 'blur' }],
};

const openRoleDialog = (row?: Role) => {
  roleFormRef.value?.resetFields();
  if (row) {
    Object.assign(roleForm, {
      id: row.id,
      name: row.name,
      code: row.code,
      description: row.description || '',
    });
  } else {
    Object.assign(roleForm, {
      id: 0,
      name: '',
      code: '',
      description: '',
    });
  }
  roleDialogVisible.value = true;
};

const submitRole = async () => {
  const valid = await roleFormRef.value?.validate();
  if (!valid) return;
  roleSubmitLoading.value = true;
  try {
    if (roleForm.id) {
      await updateRole(roleForm.id, {
        name: roleForm.name,
        description: roleForm.description,
      });
      ElMessage.success(t('system.role.message.updateSuccess'));
    } else {
      await createRole({
        name: roleForm.name,
        code: roleForm.code,
        description: roleForm.description,
      });
      ElMessage.success(t('system.role.message.createSuccess'));
    }
    roleDialogVisible.value = false;
    fetchRoles();
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) || t('system.role.message.operationFailed')
    );
  } finally {
    roleSubmitLoading.value = false;
  }
};

const deleteRole = async (row: Role) => {
  try {
    await ElMessageBox.confirm(
      t('system.role.message.deleteConfirm', { name: row.name }),
      t('system.role.message.deleteTitle'),
      { type: 'warning' }
    );
    await deleteRoleApi(row.id);
    ElMessage.success(t('system.role.message.deleteSuccess'));
    fetchRoles();
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    if (e !== 'cancel')
      ElMessage.error(
        (e instanceof Error ? e.message : String(e)) || t('system.role.message.deleteFailed')
      );
  }
};

// 权限配置。
// 后端契约（iam.rs roles 路由 + role_handler）：
// - GET  /permissions            → 权限目录 [{id,resource_type,action,allowed}]（id 为目录合成 id，非关联行 id）
// - GET  /roles/{id}/permissions → 角色权限关联行 [{id(关联行主键),resource_type,resource_id,action,allowed}]
// - POST /roles/{id}/permissions → 单条赋权 {resource_type,action,allowed}（幂等 upsert，admin 专用）
// - DELETE /roles/permissions/{id} → 按关联行 id 删除（系统内置角色拒绝）
// 因此树节点以 `${resource_type}::${action}` 为主键，提交时做增量 diff：
// 新增勾选项逐条 POST 赋权；取消勾选的既有授予逐条 DELETE 关联行。
const permissionDialogVisible = ref(false);
const currentRoleId = ref(0);
const currentRoleName = ref('');
const permissionTree = ref<PermissionTreeNode[]>([]);
// 勾选的树节点 key（`${rt}::${action}`），提交时与既有授予做 diff
const checkedKeys = ref<string[]>([]);
// 角色当前权限关联行（key → {id: 关联行主键, allowed}），用于 diff 与取消勾选时删除
const existingGrantRows = ref<Map<string, { id: number; allowed: boolean }>>(new Map());
const permissionLoading = ref(false);
const permissionSubmitLoading = ref(false);

const permKey = (rt: string, action: string): string => `${rt}::${action}`;

const openPermissionDialog = (row: Role) => {
  currentRoleId.value = row.id;
  currentRoleName.value = row.name;
  fetchRolePermissions(row.id);
  permissionDialogVisible.value = true;
};

const fetchRolePermissions = async (roleId: number) => {
  permissionLoading.value = true;
  try {
    const [treeRes, roleRes] = await Promise.all([
      getPermissionList(),
      getRolePermissions(roleId),
    ]);
    permissionTree.value = buildPermissionTree(treeRes.data || []);
    const grantRows = (roleRes.data || []) as Permission[];
    const map = new Map<string, { id: number; allowed: boolean }>();
    const keys: string[] = [];
    grantRows.forEach((p) => {
      const k = permKey(p.resource_type, p.action);
      map.set(k, { id: p.id, allowed: p.allowed });
      if (p.allowed) keys.push(k);
    });
    existingGrantRows.value = map;
    checkedKeys.value = keys;
  } catch (e) {
    const { logger } = await import('@/utils/logger');
    logger.error(`${t('system.role.message.fetchPermissionFailed')}:`, e);
  } finally {
    permissionLoading.value = false;
  }
};

interface PermissionTreeNode {
  key: string;
  label: string;
  children: PermissionTreeNode[];
}

// 权限目录按 resource_type 分组为两层树（后端 /permissions 无 parent_id 层级）
const buildPermissionTree = (perms: Permission[]): PermissionTreeNode[] => {
  const groups = new Map<string, PermissionTreeNode>();
  perms.forEach((p) => {
    if (!p.resource_type || !p.action) return;
    let g = groups.get(p.resource_type);
    if (!g) {
      g = { key: `group::${p.resource_type}`, label: p.resource_type, children: [] };
      groups.set(p.resource_type, g);
    }
    g.children.push({
      key: permKey(p.resource_type, p.action),
      label: `${p.resource_type}:${p.action}${p.allowed ? '' : ' (denied)'}`,
      children: [],
    });
  });
  return Array.from(groups.values()).sort((a, b) => a.label.localeCompare(b.label));
};

const handlePermissionCheck = (_: unknown, { checkedKeys: keys }: { checkedKeys: unknown[] }) => {
  checkedKeys.value = keys
    .map((k) => String(k))
    .filter((k) => !k.startsWith('group::'));
};

const submitPermissions = async () => {
  permissionSubmitLoading.value = true;
  const wanted = new Set(checkedKeys.value);
  const assignTasks: Array<Promise<unknown>> = [];
  const removeTasks: Array<Promise<unknown>> = [];
  // 勾选项：无关联行或既有行 allowed=false → 逐条 POST 单条赋权（幂等）
  checkedKeys.value.forEach((k) => {
    const existing = existingGrantRows.value.get(k);
    if (existing === undefined || !existing.allowed) {
      const sep = k.indexOf('::');
      const resourceType = k.slice(0, sep);
      const act = k.slice(sep + 2);
      assignTasks.push(
        assignPermission(currentRoleId.value, { resource_type: resourceType, action: act, allowed: true })
      );
    }
  });
  // 取消勾选的既有授予行：逐条 DELETE
  existingGrantRows.value.forEach((row, k) => {
    if (!wanted.has(k) && row.allowed) {
      removeTasks.push(deletePermission(currentRoleId.value, row.id));
    }
  });
  try {
    const results = await Promise.allSettled([...assignTasks, ...removeTasks]);
    const failed = results.filter((r) => r.status === 'rejected');
    if (failed.length > 0) {
      const first = failed[0] as PromiseRejectedResult;
      ElMessage.error(
        `${t('system.role.message.permissionFailed')}（${failed.length}/${results.length} ${t('system.role.message.permissionPartialFailed')}）: ` +
          (first.reason instanceof Error ? first.reason.message : String(first.reason))
      );
    } else {
      ElMessage.success(t('system.role.message.permissionSuccess'));
      permissionDialogVisible.value = false;
    }
  } finally {
    permissionSubmitLoading.value = false;
  }
};

onMounted(() => {
  fetchRoles();
});
</script>
