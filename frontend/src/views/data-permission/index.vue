<template>
  <div class="data-permission">
    <el-card>
      <template #header>
        <div class="card-header">
          <span>{{ t('dataPermission.index.pageTitle') }}</span>
        </div>
      </template>

      <div class="layout">
        <div class="role-panel">
          <h3>{{ t('dataPermission.index.titleRoleList') }}</h3>
          <el-menu :default-active="selectedRoleId" class="role-menu" @select="handleSelectRole">
            <el-menu-item :index="ALL_ROLES_KEY">{{
              t('dataPermission.index.menuAllRoles')
            }}</el-menu-item>
            <el-menu-item v-for="role in roleList" :key="role.id" :index="String(role.id)">
              {{ role.name }}
            </el-menu-item>
          </el-menu>
        </div>

        <div class="permission-panel">
          <div class="panel-header">
            <h3 v-if="isOverview">{{ t('dataPermission.index.titleAllRoles') }}</h3>
            <h3 v-else>{{ currentRoleName }} - {{ t('dataPermission.index.titlePermission') }}</h3>
            <el-button v-if="isAdmin && !isOverview" type="primary" @click="handleAddPermission">{{
              t('dataPermission.index.buttonAddPermission')
            }}</el-button>
          </div>

          <p class="scope-note">{{ t('dataPermission.index.noteEnabledOnly') }}</p>

          <el-table
            :data="permissionList"
            border
            stripe
            :aria-label="t('dataPermission.index.ariaTable')"
          >
            <el-table-column
              v-if="isOverview"
              prop="role_name"
              :label="t('dataPermission.index.colRole')"
              width="140"
            />
            <el-table-column
              prop="resource_type"
              :label="t('dataPermission.index.colResourceType')"
            >
              <template #default="{ row }">{{ resourceTypeLabel(row.resource_type) }}</template>
            </el-table-column>
            <el-table-column prop="scope_type" :label="t('dataPermission.index.colScopeType')">
              <template #default="{ row }">
                <el-tag v-if="row.scope_type === 'ALL'" type="success">{{
                  t('dataPermission.index.scopeAll')
                }}</el-tag>
                <el-tag v-else-if="row.scope_type === 'DEPT'" type="primary">{{
                  t('dataPermission.index.scopeDept')
                }}</el-tag>
                <el-tag v-else-if="row.scope_type === 'DEPT_AND_BELOW'" type="warning">{{
                  t('dataPermission.index.scopeDeptAndBelow')
                }}</el-tag>
                <el-tag v-else-if="row.scope_type === 'SELF'" type="info">{{
                  t('dataPermission.index.scopeSelf')
                }}</el-tag>
                <el-tag v-else type="danger">{{ t('dataPermission.index.scopeCustom') }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column
              prop="is_enabled"
              :label="t('dataPermission.index.colStatus')"
              width="100"
            >
              <template #default="{ row }">
                <el-tag v-if="row.is_enabled" type="success">{{
                  t('dataPermission.index.statusEnabled')
                }}</el-tag>
                <el-tag v-else type="danger">{{ t('dataPermission.index.statusDisabled') }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column :label="t('dataPermission.index.colCreatedAt')" width="180">
              <template #default="{ row }">{{ formatDateTime(row.created_at) }}</template>
            </el-table-column>
            <el-table-column
              v-if="!isOverview"
              :label="t('dataPermission.index.colOperation')"
              width="150"
            >
              <template #default="{ row }">
                <el-button
                  v-if="isAdmin"
                  link
                  type="primary"
                  @click="handleEditPermission(row as DataPermissionRow)"
                  >{{ t('dataPermission.index.buttonEdit') }}</el-button
                >
                <el-button
                  v-if="isAdmin"
                  link
                  type="danger"
                  @click="handleDeletePermission(row as DataPermissionRow)"
                  >{{ t('dataPermission.index.buttonDelete') }}</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </div>
      </div>
    </el-card>

    <!-- 权限设置对话框 -->
    <el-dialog
      v-model="permissionDialogVisible"
      :title="isEdit ? t('dataPermission.index.titleEdit') : t('dataPermission.index.titleCreate')"
      width="600px"
      :aria-label="
        isEdit
          ? t('dataPermission.index.ariaEditDialog')
          : t('dataPermission.index.ariaCreateDialog')
      "
    >
      <el-form
        ref="permissionFormRef"
        :model="permissionForm"
        :rules="permissionRules"
        label-width="120px"
        :aria-label="t('dataPermission.index.ariaForm')"
      >
        <el-form-item :label="t('dataPermission.index.colResourceType')" prop="resource_type">
          <el-select
            v-model="permissionForm.resource_type"
            :placeholder="t('dataPermission.index.placeholderSelect')"
            style="width: 100%"
          >
            <el-option
              v-for="resourceType in resourceTypeOptions"
              :key="resourceType.value"
              :label="resourceType.label"
              :value="resourceType.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('dataPermission.index.colScopeType')" prop="scope_type">
          <el-select
            v-model="permissionForm.scope_type"
            :placeholder="t('dataPermission.index.placeholderSelect')"
            style="width: 100%"
          >
            <el-option
              v-for="scope in scopeTypeList"
              :key="scope.value"
              :label="scopeLabel(scope.value)"
              :value="scope.value"
            >
              <span>{{ scopeLabel(scope.value) }}</span>
              <span class="scope-option-desc">{{ scopeDescription(scope.value) }}</span>
            </el-option>
          </el-select>
        </el-form-item>
        <el-form-item
          v-if="permissionForm.scope_type === 'CUSTOM'"
          :label="t('dataPermission.index.labelCustomCondition')"
          prop="custom_condition"
        >
          <el-input
            v-model="permissionForm.custom_condition"
            type="textarea"
            :rows="4"
            :placeholder="t('dataPermission.index.placeholderCustomCondition')"
          />
        </el-form-item>
        <el-form-item :label="t('dataPermission.index.labelAllowedFields')" prop="allowed_fields">
          <el-input
            v-model="permissionForm.allowed_fields"
            type="textarea"
            :rows="2"
            :placeholder="t('dataPermission.index.placeholderAllowedFields')"
          />
        </el-form-item>
        <el-form-item :label="t('dataPermission.index.labelHiddenFields')" prop="hidden_fields">
          <el-input
            v-model="permissionForm.hidden_fields"
            type="textarea"
            :rows="2"
            :placeholder="t('dataPermission.index.placeholderHiddenFields')"
          />
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="permissionDialogVisible = false">{{
          t('dataPermission.index.buttonCancel')
        }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSavePermission">{{
          t('dataPermission.index.buttonSave')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { isDialogDismissal } from '@/utils/monitor';
import { ref, reactive, onMounted, computed } from 'vue';
import { logger } from '@/utils/logger';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import { useUserStore } from '@/store/user';
import {
  getDataPermissionList,
  getRoleDataPermissionList,
  getDataPermission,
  setDataPermission,
  deleteDataPermission,
  getResourceTypes,
  getScopeTypeList,
  DEFAULT_SCOPE_TYPES,
  type DataPermissionRole,
  type ScopeType,
  type CustomCondition,
  type AllowedFields,
  type HiddenFields,
  type DataPermissionRow,
} from '@/api/data-permission';
import {
  FALLBACK_DATA_PERMISSION_RESOURCE_TYPES,
  dataPermissionResourceTypeLabelKey,
} from '@/constants/data-permission-resource';
import { getRoleList } from '@/api/role';

const { t } = useI18n({ useScope: 'global' });
const userStore = useUserStore();

// 数据权限写操作（列表/编辑/删除/新增）在后端 require_admin_role 门控下仅管理员可用。
// 前端据后端注入的 `*:*` 超级通配权限（管理员角色专属）判定 admin，与 admin-only 现实对齐，
// 不再用按角色的死权限键假装可授权；非 admin 隐藏入口，后端 fail-closed 403 时也不再误报为缺陷。
const isAdmin = computed(() =>
  (userStore.userInfo?.permissions || []).some(perm => perm === '*:*')
);

const roleList = ref<Array<{ id: number; name: string }>>([]);
// 左侧菜单首项为跨角色只读概览，用非数字哨兵与真实角色主键区分
const ALL_ROLES_KEY = 'all';
const selectedRoleId = ref('1');
const isOverview = computed(() => selectedRoleId.value === ALL_ROLES_KEY);
const permissionList = ref<DataPermissionRole[]>([]);
const scopeTypeList = ref<ScopeType[]>([]);
// resource_type 下拉候选，来自后端资源类型接口（失败时以兜底常量填充）
const resourceTypes = ref<string[]>([]);

const resourceTypeLabel = (value: string): string => {
  const labelKey = dataPermissionResourceTypeLabelKey(value);
  return labelKey ? t(labelKey) : value;
};

// 范围类型下拉的展示文案取前端 i18n（词表=后端 GET /data-permissions/scope-types 的
// value 集合，后端 label/description 为中文硬编码，不参与展示，避免英文界面露中文）。
// 后端词表若新增未映射取值，原样显示 token 并 warn 点名，不猜含义。
const SCOPE_LABEL_KEYS: Record<string, string> = {
  ALL: 'dataPermission.index.scopeAll',
  DEPT: 'dataPermission.index.scopeDept',
  DEPT_AND_BELOW: 'dataPermission.index.scopeDeptAndBelow',
  SELF: 'dataPermission.index.scopeSelf',
  CUSTOM: 'dataPermission.index.scopeCustom',
};
const SCOPE_DESC_KEYS: Record<string, string> = {
  ALL: 'dataPermission.index.descAll',
  DEPT: 'dataPermission.index.descDept',
  DEPT_AND_BELOW: 'dataPermission.index.descDeptAndBelow',
  SELF: 'dataPermission.index.descSelf',
  CUSTOM: 'dataPermission.index.descCustom',
};
const warnedScopeTokens = new Set<string>();
const scopeLabel = (value: string): string => {
  const key = SCOPE_LABEL_KEYS[value];
  if (!key && !warnedScopeTokens.has(value)) {
    warnedScopeTokens.add(value);
    logger.warn(`${t('dataPermission.index.messageUnknownScopeType')}${value}`);
  }
  return key ? t(key) : value;
};
const scopeDescription = (value: string): string => {
  const key = SCOPE_DESC_KEYS[value];
  return key ? t(key) : '';
};

const resourceTypeOptions = computed(() =>
  resourceTypes.value.map(value => ({ value, label: resourceTypeLabel(value) }))
);

// 后端把自定义条件/字段列表以 JSON（对象或数组）落库，编辑回显需转成缩进 JSON 文本填入 textarea，
// 空值转空串；提交时再解析回对象/数组类型。
const fieldTextFromValue = (value: unknown): string => {
  if (value === null || value === undefined || value === '') return '';
  return typeof value === 'string' ? value : JSON.stringify(value, null, 2);
};

// 动态加载角色列表，避免硬编码
const fetchRoles = async () => {
  try {
    const res = await getRoleList();
    // 后端 list_roles 返回 RoleListResponse { roles, total }
    roleList.value = res.data.roles.map(r => ({ id: r.id, name: r.name }));
    if (roleList.value.length > 0) {
      selectedRoleId.value = String(roleList.value[0].id);
    }
  } catch (e) {
    const err = e as Error;
    ElMessage.error(
      `${t('dataPermission.index.messageLoadRolesFailed')}${err.message || t('dataPermission.index.messageUnknownError')}`
    );
  }
};

const permissionDialogVisible = ref(false);
const isEdit = ref(false);
const submitLoading = ref(false);
const permissionFormRef = ref();

const permissionForm = reactive({
  id: undefined as number | undefined,
  role_id: undefined as number | undefined,
  resource_type: '',
  scope_type: '',
  custom_condition: '',
  allowed_fields: '',
  hidden_fields: '',
});

const permissionRules = {
  resource_type: [
    {
      required: true,
      message: t('dataPermission.index.ruleResourceTypeRequired'),
      trigger: 'change',
    },
  ],
  scope_type: [
    { required: true, message: t('dataPermission.index.ruleScopeTypeRequired'), trigger: 'change' },
  ],
};

const currentRoleName = computed(() => {
  const role = roleList.value.find(r => String(r.id) === selectedRoleId.value);
  return role ? role.name : '';
});

const fetchPermissions = async () => {
  try {
    // 两个视图的后端出参都只含启用记录：概览走 GET /data-permissions（按 role_id LEFT JOIN 富化
    // role_name），单角色走 GET /data-permissions/roles/{id}（该端点不富化，role_name 恒为 null，
    // 归属由左侧选中项表达）。两端点都先过 require_admin_role，非管理员拿到的是 403。
    const res = isOverview.value
      ? await getDataPermissionList()
      : await getRoleDataPermissionList(parseInt(selectedRoleId.value));
    if (res.data) {
      // 两端点均返回裸数组载荷
      permissionList.value = res.data;
    }
  } catch {
    ElMessage.error(t('dataPermission.index.messageLoadPermissionsFailed'));
  }
};

// 后端 created_at 为 UTC RFC3339，概览按浏览器本地时区展示
const formatDateTime = (value: string): string => new Date(value).toLocaleString();

const fetchScopeTypes = async () => {
  try {
    const res = await getScopeTypeList();
    if (res.data && Array.isArray(res.data) && res.data.length > 0) {
      scopeTypeList.value = res.data;
    } else {
      scopeTypeList.value = DEFAULT_SCOPE_TYPES;
    }
  } catch (e) {
    // 接口不可用时用兜底常量，并告知用户
    scopeTypeList.value = DEFAULT_SCOPE_TYPES;
    const err = e as Error;
    ElMessage.warning(
      `${t('dataPermission.index.messageLoadScopeTypesFailed')}${err.message || t('dataPermission.index.messageUnknownError')}`
    );
  }
};

const fetchResourceTypes = async () => {
  try {
    const res = await getResourceTypes();
    if (Array.isArray(res.data) && res.data.length > 0) {
      resourceTypes.value = res.data;
    } else {
      // 接口返回空或非数组属契约异常，不静默当空下拉，用兜底常量并留痕
      resourceTypes.value = FALLBACK_DATA_PERMISSION_RESOURCE_TYPES;
      logger.warn(t('dataPermission.index.messageResourceTypesEmpty'));
    }
  } catch (e) {
    // 接口不可用时用兜底常量，避免下拉候选变空导致无法新增权限
    resourceTypes.value = FALLBACK_DATA_PERMISSION_RESOURCE_TYPES;
    const err = e as Error;
    logger.warn(
      `${t('dataPermission.index.messageLoadResourceTypesFailed')}${err.message || t('dataPermission.index.messageUnknownError')}`
    );
  }
  // 校验词表漂移：接口返回的 token 若不在文案映射内，原样显示并告警，不猜含义
  for (const value of resourceTypes.value) {
    if (!dataPermissionResourceTypeLabelKey(value)) {
      logger.warn(`${t('dataPermission.index.messageUnknownResourceType')}${value}`);
    }
  }
};

const handleSelectRole = (roleId: string) => {
  selectedRoleId.value = roleId;
  fetchPermissions();
};

const handleAddPermission = () => {
  isEdit.value = false;
  // 新增复位为 snake_case（与 permissionForm 声明一致），并清空主键避免先编辑后新增时残留上次记录
  Object.assign(permissionForm, {
    id: undefined,
    role_id: parseInt(selectedRoleId.value),
    resource_type: '',
    scope_type: '',
    custom_condition: '',
    allowed_fields: '',
    hidden_fields: '',
  });
  permissionDialogVisible.value = true;
};

const handleEditPermission = async (row: DataPermissionRow) => {
  isEdit.value = true;
  let source: DataPermissionRole = row;
  try {
    // 编辑前按主键回源最新权限配置（by-id 端点，可见性过滤后可能为 null，保留行数据兜显）
    const res = await getDataPermission(row.id);
    if (res.data) source = res.data;
  } catch (error) {
    logger.error(t('dataPermission.index.messageDetailFailed'), error);
  }
  Object.assign(permissionForm, {
    id: source.id,
    role_id: source.role_id,
    resource_type: source.resource_type,
    scope_type: source.scope_type,
    custom_condition: fieldTextFromValue(source.custom_condition),
    allowed_fields: fieldTextFromValue(source.allowed_fields),
    hidden_fields: fieldTextFromValue(source.hidden_fields),
  });
  permissionDialogVisible.value = true;
};

const handleSavePermission = async () => {
  if (!permissionFormRef.value || submitLoading.value) return;

  // 锁必须先于异步校验：校验本身要等一拍，连点时第二次调用会绕过回调内的状态位
  submitLoading.value = true;
  try {
    const valid = await permissionFormRef.value.validate().catch(() => false);
    if (!valid) return;

    // role_id 无对话框内选择器（角色取自左侧菜单），在提交边界强制校验，杜绝空值上送
    const roleId = Number(selectedRoleId.value);
    if (!Number.isInteger(roleId) || roleId <= 0) {
      logger.error(t('dataPermission.index.messageRoleNotSelected'));
      ElMessage.error(t('dataPermission.index.messageRoleNotSelected'));
      return;
    }

    // 表单字段为 JSON 文本，提交前解析为 API 契约类型；仅在解析或类型真不符时报错并留痕。
    // parseJsonLiteral 把 JSON.parse 的原生 SyntaxError（英文实现细节）收敛为具名
    // i18n 文案，用户看到"该改成什么形态"，而不是解析器内部报错。
    const parseJsonLiteral = (text: string, failMessage: string): unknown => {
      try {
        return JSON.parse(text) as unknown;
      } catch (e) {
        logger.warn('JSON 字面量解析失败', e);
        throw new Error(failMessage);
      }
    };
    const parseJsonObject = (text: string): CustomCondition | undefined => {
      if (!text.trim()) return undefined;
      const parsed = parseJsonLiteral(
        text,
        t('dataPermission.index.messageCustomConditionNotObject')
      );
      if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) {
        throw new Error(t('dataPermission.index.messageCustomConditionNotObject'));
      }
      return parsed as CustomCondition;
    };
    const parseJsonArray = (text: string): AllowedFields | undefined => {
      const trimmed = text.trim();
      if (!trimmed) return undefined;
      // 兼容两种真实输入形态：JSON 数组字面量，或逗号分隔的字段名清单
      //（"允许字段/隐藏字段"本质是名字列表，行业通行做法是直接列名；
      // 只收 JSON 数组会让用户按直觉填 cost_price,profit 却被判"JSON 非法"）
      let parsed: unknown;
      if (trimmed.startsWith('[')) {
        parsed = parseJsonLiteral(trimmed, t('dataPermission.index.messageFieldListNotArray'));
        if (!Array.isArray(parsed)) {
          throw new Error(t('dataPermission.index.messageFieldListNotArray'));
        }
        if (!parsed.every(item => typeof item === 'string')) {
          throw new Error(t('dataPermission.index.messageFieldListItemsNotString'));
        }
        return parsed as AllowedFields;
      }
      const names = trimmed
        .split(',')
        .map(item => item.trim())
        .filter(item => item.length > 0);
      if (names.length === 0) {
        throw new Error(t('dataPermission.index.messageFieldListEmpty'));
      }
      return names;
    };
    let customCondition: CustomCondition | undefined;
    let allowedFields: AllowedFields | undefined;
    let hiddenFields: HiddenFields | undefined;
    try {
      customCondition = parseJsonObject(permissionForm.custom_condition);
      allowedFields = parseJsonArray(permissionForm.allowed_fields);
      hiddenFields = parseJsonArray(permissionForm.hidden_fields);
    } catch (error) {
      const err = error as Error;
      logger.error(t('dataPermission.index.messageJsonInvalid'), err);
      // 解析器抛出的都是具名 i18n 消息，直接点名；仅在消息缺失时退回泛化文案
      ElMessage.error(err.message || t('dataPermission.index.messageJsonInvalid'));
      return;
    }

    // set_data_permission 为 upsert（按 role_id + resource_type 幂等），编辑与新增同走 POST
    await setDataPermission({
      role_id: roleId,
      resource_type: permissionForm.resource_type,
      scope_type: permissionForm.scope_type,
      custom_condition: customCondition,
      allowed_fields: allowedFields,
      hidden_fields: hiddenFields,
    });
    ElMessage.success(
      isEdit.value
        ? t('dataPermission.index.messageUpdateSuccess')
        : t('dataPermission.index.messageSaveSuccess')
    );
    permissionDialogVisible.value = false;
    fetchPermissions();
  } catch (e: unknown) {
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) ||
        (isEdit.value
          ? t('dataPermission.index.messageUpdateFailed')
          : t('dataPermission.index.messageSaveFailed'))
    );
  } finally {
    submitLoading.value = false;
  }
};

const handleDeletePermission = async (row: DataPermissionRow) => {
  if (!row.id) {
    logger.error(t('dataPermission.index.messageDeleteNoId'));
    ElMessage.error(t('dataPermission.index.messageDeleteNoId'));
    return;
  }

  try {
    await ElMessageBox.confirm(
      t('dataPermission.index.messageConfirmDelete'),
      t('dataPermission.index.titleConfirm'),
      {
        confirmButtonText: t('dataPermission.index.buttonConfirm'),
        cancelButtonText: t('dataPermission.index.buttonCancel'),
        type: 'warning',
      }
    );

    // 按主键删除（by-id 端点）
    await deleteDataPermission(row.id);
    ElMessage.success(t('dataPermission.index.messageDeleteSuccess'));
    fetchPermissions();
  } catch (e: unknown) {
    if (!isDialogDismissal(e)) {
      logger.error(t('dataPermission.index.messageDeleteFailed'), e);
      ElMessage.error(
        (e instanceof Error ? e.message : String(e)) ||
          t('dataPermission.index.messageDeleteFailed')
      );
    }
  }
};

const hasLoaded = createLazyLoader();

onMounted(async () => {
  // 先加载角色列表（fetchRoles 会更新 selectedRoleId），再加载权限、范围类型与资源类型
  await fetchRoles();
  fetchPermissions();
  loadIfNot('scopeTypes', fetchScopeTypes, hasLoaded);
  loadIfNot('resourceTypes', fetchResourceTypes, hasLoaded);
});
</script>

<style scoped>
.data-permission .card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.data-permission .layout {
  display: flex;
  gap: 20px;
}

.data-permission .layout .role-panel {
  width: 240px;
  flex-shrink: 0;
}

.data-permission .layout .role-panel h3 {
  margin-bottom: 12px;
  font-size: 16px;
}

.data-permission .layout .role-panel .role-menu {
  border: 1px solid #ebeef5;
  border-radius: 4px;
}

.data-permission .layout .permission-panel {
  flex: 1;
}

.data-permission .layout .permission-panel .panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.data-permission .layout .permission-panel .panel-header h3 {
  font-size: 16px;
  margin: 0;
}

.data-permission .layout .permission-panel .scope-note {
  margin: 0 0 12px;
  font-size: 12px;
  color: #909399;
}

@media (max-width: 768px) {
  /* 固定 240 像素的角色栏在竖屏把权限表格压到无法阅读，改为上下堆叠 */
  .data-permission .layout {
    flex-direction: column;
    gap: 12px;
  }

  .data-permission .layout .role-panel {
    width: auto;
  }

  .data-permission .layout .permission-panel .panel-header {
    flex-wrap: wrap;
    gap: 8px;
  }
}
</style>
