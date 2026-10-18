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
        <!-- 整批替换端点仅管理员可用：非管理员不渲染保存按钮，避免留下必然被拒的入口 -->
        <el-button
          v-if="canManageGrants"
          type="primary"
          :loading="permissionSubmitLoading"
          @click="submitPermissions"
          >{{ t('system.role.permissionDialog.button.save') }}</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { isDialogDismissal } from '@/utils/monitor';
import { ref, reactive, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import type { FormInstance, FormRules } from 'element-plus';
import { logger } from '@/utils/logger';
import { useUserStore } from '@/store/user';
import {
  getRoleList,
  createRole,
  updateRole,
  deleteRole as deleteRoleApi,
  getRolePermissions,
  getPermissionList,
  replaceRolePermissions,
  type Role,
  type Permission,
  type RolePermissionBatchEntry,
} from '@/api/role';

const { t } = useI18n({ useScope: 'global' });

const roles = ref<Role[]>([]);
const roleLoading = ref(false);

const fetchRoles = async () => {
  roleLoading.value = true;
  try {
    const res = await getRoleList();
    // 后端 list_roles 真实结构为 data.roles[]（RoleListResponse { roles, total }）
    roles.value = res.data.roles;
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
    if (!isDialogDismissal(e))
      ElMessage.error(
        (e instanceof Error ? e.message : String(e)) || t('system.role.message.deleteFailed')
      );
  }
};

// 权限配置。
// 后端契约（routes/iam.rs 的 roles 挂载 + handlers/role_handler.rs）：
// - GET  /permissions            → 权限目录（资源注册表 × 可授予动作派生的裸数组）
// - GET  /roles/{id}/permissions → 角色级授予行（resource_id IS NULL）裸数组
//                                  [{id(关联行主键),resource_type,resource_id,action,allowed}]
// - PUT  /roles/{id}/permissions → 整批替换角色级授予：请求体是期望的完整终集合
//                                  [{resource_type,action,allowed}] 裸数组（非对象包装），
//                                  服务端单事务做新增/更新/删除三态复算并在最终集合上复算
//                                  职责分离校验，出参为替换后的角色级授予全集
// - POST /roles/{id}/permissions、DELETE /roles/permissions/{id} 只服务记录级授权行，
//                                  不在角色级配置面内
// 树节点以 `${resource_type}::${action}` 为主键，保存时由「勾选集合 + 既有授予」算出目标
// 终集合，一次 PUT 提交，随后 GET 读回终集合并据实提示。
const permissionDialogVisible = ref(false);
const currentRoleId = ref(0);
const currentRoleName = ref('');
const permissionTree = ref<PermissionTreeNode[]>([]);
// 勾选的树节点 key（`${rt}::${action}`），与既有授予一起算出整批替换的目标终集合
const checkedKeys = ref<string[]>([]);
// 角色当前的角色级授予（key → allowed）；GET 返回的每一行都在替换作用面内
const existingGrants = ref<Map<string, boolean>>(new Map());
const permissionLoading = ref(false);
const permissionSubmitLoading = ref(false);
// 角色级授予是否已读取成功：没读到现状就禁止提交，否则目标终集合会凭残缺状态算出，
// 整批替换会把未加载到的授予当"期望集外的行"删掉。
const grantsLoaded = ref(false);

const userStore = useUserStore();
// 整批替换端点在后端首行 require_admin_role，非管理员调用必返 403，因此按管理员身份决定
// 保存按钮是否渲染。判定口径与同域数据权限页一致：读后端为管理员角色注入的 `*:*` 超级
// 通配权限，不自造权限键。查看授予（GET 端点）不受该门限制，入口对本页可见的角色保持可用。
const canManageGrants = computed(() =>
  (userStore.userInfo?.permissions || []).some(perm => perm === '*:*')
);

const permKey = (rt: string, action: string): string => `${rt}::${action}`;

/**
 * 解析树节点主键为资源与动作。
 * 入参：`${resource_type}::${action}` 形态的键。
 * 出参：{resourceType, action}；缺分隔符或任一侧为空时返回 null，由调用方跳过该键。
 */
const splitPermKey = (key: string): { resourceType: string; action: string } | null => {
  const sep = key.indexOf('::');
  if (sep <= 0 || sep + 2 >= key.length) return null;
  return { resourceType: key.slice(0, sep), action: key.slice(sep + 2) };
};

const openPermissionDialog = (row: Role) => {
  currentRoleId.value = row.id;
  currentRoleName.value = row.name;
  // 每次打开都从空态开始：上一个角色的勾选/授予若残留，整批替换会把别的角色的授予
  // 当作本角色的现状提交（读回失败时终集合还会退化成空集，等于清空本角色权限）。
  permissionTree.value = [];
  checkedKeys.value = [];
  existingGrants.value = new Map();
  grantsLoaded.value = false;
  fetchRolePermissions(row.id);
  permissionDialogVisible.value = true;
};

const fetchRolePermissions = async (roleId: number) => {
  permissionLoading.value = true;
  try {
    const [treeRes, roleRes] = await Promise.all([getPermissionList(), getRolePermissions(roleId)]);
    permissionTree.value = buildPermissionTree(treeRes.data || []);
    const grantRows = roleRes.data;
    if (!Array.isArray(grantRows)) {
      // 后端出参是裸数组，取不到数组即契约异常，显式失败而不是当空集合继续渲染
      throw new Error(t('system.role.message.permissionReadBackInvalid'));
    }
    const map = new Map<string, boolean>();
    const keys: string[] = [];
    grantRows.forEach(p => {
      const k = permKey(p.resource_type, p.action);
      map.set(k, p.allowed);
      if (p.allowed) keys.push(k);
    });
    existingGrants.value = map;
    checkedKeys.value = keys;
    grantsLoaded.value = true;
  } catch (e) {
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
  perms.forEach(p => {
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
  checkedKeys.value = keys.map(k => String(k)).filter(k => !k.startsWith('group::'));
};

/**
 * 由勾选集合与既有角色级授予算出整批替换的目标终集合（后端批量授予项数组）。
 * 规则：勾选中的键一律以 allowed=true 入集；未勾选的既有行只在 allowed=false 时原样保留
 * （显式拒绝行不因取消勾选而消失）；未勾选且 allowed=true 的既有行不入集，由服务端按
 * 「期望集外的既有行」删除。同一键只出现一次，出参按 resource_type、action 稳定排序。
 */
const buildDesiredEntries = (
  checked: ReadonlySet<string>,
  existing: ReadonlyMap<string, boolean>
): RolePermissionBatchEntry[] => {
  const entries: RolePermissionBatchEntry[] = [];
  checked.forEach(key => {
    const parsed = splitPermKey(key);
    if (parsed)
      entries.push({ resource_type: parsed.resourceType, action: parsed.action, allowed: true });
  });
  existing.forEach((allowed, key) => {
    if (allowed || checked.has(key)) return;
    const parsed = splitPermKey(key);
    if (parsed)
      entries.push({ resource_type: parsed.resourceType, action: parsed.action, allowed: false });
  });
  return entries.sort(
    (a, b) => a.resource_type.localeCompare(b.resource_type) || a.action.localeCompare(b.action)
  );
};

/**
 * 按目标终集合找出同时持有「新建 + 审批」（或通配动作 * 与其中任一）的资源 token。
 * 与后端终集合职责分离校验同口径，只用于提交前把冲突点名给用户；服务端仍是权威：
 * 记录级授权行不在本对话框可见范围内，预检通过也必须照常提交并按读回结果提示。
 */
const findDutyConflicts = (entries: readonly RolePermissionBatchEntry[]): string[] => {
  const flags = new Map<string, { create: boolean; approve: boolean; wildcard: boolean }>();
  entries.forEach(entry => {
    if (!entry.allowed) return;
    const current = flags.get(entry.resource_type) ?? {
      create: false,
      approve: false,
      wildcard: false,
    };
    if (entry.action === 'create') current.create = true;
    if (entry.action === 'approve') current.approve = true;
    if (entry.action === '*') current.wildcard = true;
    flags.set(entry.resource_type, current);
  });
  return [...flags.entries()]
    .filter(([, f]) => (f.create && f.approve) || (f.wildcard && (f.create || f.approve)))
    .map(([resourceType]) => resourceType)
    .sort();
};

interface GrantSetDiff {
  missing: string[];
  extra: string[];
  mismatched: string[];
}

/**
 * 读回比对：把提交的目标终集合与 GET /roles/{id}/permissions 返回的授予行逐元素比对。
 * 出参：missing（期望有而库里没有）、extra（库里有而期望没有）、mismatched（allowed 不一致）
 * 三个键列表；三者皆空即两侧不多不少、逐项相同。
 */
const diffDesiredAgainstReadBack = (
  desired: readonly RolePermissionBatchEntry[],
  readBack: readonly Permission[]
): GrantSetDiff => {
  const expected = new Map<string, boolean>();
  desired.forEach(entry => expected.set(permKey(entry.resource_type, entry.action), entry.allowed));
  const actual = new Map<string, boolean>();
  readBack.forEach(row => actual.set(permKey(row.resource_type, row.action), row.allowed));
  const diff: GrantSetDiff = { missing: [], extra: [], mismatched: [] };
  expected.forEach((allowed, key) => {
    if (!actual.has(key)) diff.missing.push(key);
    else if (actual.get(key) !== allowed) diff.mismatched.push(key);
  });
  actual.forEach((_, key) => {
    if (!expected.has(key)) diff.extra.push(key);
  });
  return diff;
};

/**
 * 取后端失败信封的 message（失败体形状 `{code,message,trace_id,timestamp}`）。
 * 后端出参默认给脱敏常量、仅显式声明可外显时才是真实拒绝原因，这里原样透出，
 * 不用前端自造文案替换它。出参：字符串；响应体缺失或 message 非字符串时为空串。
 */
const backendMessageOf = (error: unknown): string => {
  const data = (error as { response?: { data?: { message?: unknown } } })?.response?.data;
  return typeof data?.message === 'string' ? data.message : '';
};

const submitPermissions = async () => {
  if (!grantsLoaded.value) {
    logger.warn(`[角色授权] 授予现状未读取成功，拒绝整批替换，role_id=${currentRoleId.value}`);
    ElMessage.error(t('system.role.message.permissionNotLoaded'));
    return;
  }
  const desired = buildDesiredEntries(new Set(checkedKeys.value), existingGrants.value);
  const conflicts = findDutyConflicts(desired);
  if (conflicts.length > 0) {
    ElMessage.warning(
      t('system.role.message.permissionDutyConflict', { resources: conflicts.join(', ') })
    );
    logger.warn(
      `[角色授权] 终集合职责预检拦截，role_id=${currentRoleId.value}: ${conflicts.join(',')}`
    );
    return;
  }
  permissionSubmitLoading.value = true;
  try {
    await replaceRolePermissions(currentRoleId.value, desired);
    const readBack = await getRolePermissions(currentRoleId.value);
    if (!Array.isArray(readBack.data)) {
      throw new Error(t('system.role.message.permissionReadBackInvalid'));
    }
    const diff = diffDesiredAgainstReadBack(desired, readBack.data);
    if (diff.missing.length > 0 || diff.extra.length > 0 || diff.mismatched.length > 0) {
      logger.error(
        `[角色授权] 读回与提交不一致，role_id=${currentRoleId.value}`,
        `missing=${diff.missing.join('|')}`,
        `extra=${diff.extra.join('|')}`,
        `mismatched=${diff.mismatched.join('|')}`
      );
      ElMessage.error(t('system.role.message.permissionVerifyFailed'));
      return;
    }
    ElMessage.success(t('system.role.message.permissionSuccess'));
    permissionDialogVisible.value = false;
  } catch (e: unknown) {
    logger.error(`[角色授权] 整批替换授予失败，role_id=${currentRoleId.value}`, e);
    const detail = backendMessageOf(e);
    ElMessage.error(
      detail
        ? `${t('system.role.message.permissionSaveFailed')}: ${detail}`
        : t('system.role.message.permissionSaveFailed')
    );
  } finally {
    permissionSubmitLoading.value = false;
  }
};

onMounted(() => {
  fetchRoles();
});
</script>
