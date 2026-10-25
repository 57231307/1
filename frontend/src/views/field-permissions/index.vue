<!--
  字段权限页（路由 /field-permissions，门控 users:read）
  调用方：MainLayout 侧栏 权限治理 分组下的 字段权限 菜单项
  职责：按角色配置资源字段的读写与脱敏（后端 /field-permissions CRUD，写键
        field-permissions:create/update/delete 由 v-permission 控制入口可见性）
-->
<template>
  <div class="field-permission-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('system.fieldPermission.title') }}</h2>
      <el-button
        v-permission="PERMISSIONS.FIELD_PERMISSION_CREATE"
        type="primary"
        @click="openCreateDialog"
      >
        <el-icon><Plus /></el-icon> {{ t('system.fieldPermission.button.create') }}
      </el-button>
    </div>
    <el-card shadow="hover">
      <div class="field-permission-toolbar">
        <el-select
          v-model="filterResourceType"
          :placeholder="t('system.fieldPermission.filterResourcePlaceholder')"
          clearable
          class="field-permission-filter"
          @change="fetchFieldPermissions"
        >
          <el-option
            v-for="rt in resourceTypeOptions"
            :key="rt"
            :label="resourceTypeLabel(rt)"
            :value="rt"
          />
        </el-select>
      </div>
      <el-table
        v-loading="fieldPermLoading"
        :data="fieldPermissionList"
        stripe
        :aria-label="t('system.fieldPermission.aria.list')"
      >
        <el-table-column
          prop="role_name"
          :label="t('system.fieldPermission.column.role')"
          width="120"
        >
          <template #default="{ row }">{{ row.role_name || `#${row.role_id}` }}</template>
        </el-table-column>
        <el-table-column
          prop="resource_type"
          :label="t('system.fieldPermission.column.resource')"
          width="140"
        >
          <template #default="{ row }">{{ resourceTypeLabel(row.resource_type) }}</template>
        </el-table-column>
        <el-table-column
          prop="field_name"
          :label="t('system.fieldPermission.column.fieldName')"
          width="160"
          show-overflow-tooltip
        />
        <el-table-column
          prop="can_read"
          :label="t('system.fieldPermission.column.canRead')"
          width="80"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="row.can_read ? 'success' : 'danger'" size="small">
              {{
                row.can_read
                  ? t('system.fieldPermission.common.yes')
                  : t('system.fieldPermission.common.no')
              }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="can_write"
          :label="t('system.fieldPermission.column.canWrite')"
          width="80"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="row.can_write ? 'success' : 'info'" size="small">
              {{
                row.can_write
                  ? t('system.fieldPermission.common.yes')
                  : t('system.fieldPermission.common.no')
              }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="mask_strategy"
          :label="t('system.fieldPermission.column.maskStrategy')"
          width="110"
          align="center"
        >
          <template #default="{ row }">{{ maskStrategyLabel(row.mask_strategy) }}</template>
        </el-table-column>
        <el-table-column
          prop="is_enabled"
          :label="t('system.fieldPermission.column.enabled')"
          width="90"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="row.is_enabled ? 'success' : 'info'" size="small">
              {{
                row.is_enabled
                  ? t('system.fieldPermission.status.enabled')
                  : t('system.fieldPermission.status.disabled')
              }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('system.fieldPermission.column.action')" width="150" fixed="right">
          <template #default="{ row }">
            <el-button
              v-permission="PERMISSIONS.FIELD_PERMISSION_UPDATE"
              size="small"
              link
              type="primary"
              @click="openEditDialog(row as FieldPermissionRow)"
              >{{ t('system.fieldPermission.button.edit') }}</el-button
            >
            <el-button
              v-permission="PERMISSIONS.FIELD_PERMISSION_DELETE"
              size="small"
              link
              type="danger"
              @click="removeFieldPermission(row as FieldPermissionRow)"
              >{{ t('system.fieldPermission.button.delete') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 字段权限编辑对话框：新增可改全部定位键；编辑时 role/resource/field 是唯一定位
         键（后端 PUT 载荷不含这三键），只读展示 -->
    <el-dialog
      v-model="dialogVisible"
      :title="
        isEdit ? t('system.fieldPermission.dialog.editTitle') : t('system.fieldPermission.dialog.createTitle')
      "
      width="560px"
      :aria-label="t('system.fieldPermission.dialog.aria')"
    >
      <el-form ref="formRef" :model="form" :rules="formRules" label-width="120px">
        <el-form-item :label="t('system.fieldPermission.dialog.labelRole')" prop="role_id">
          <el-select
            v-model="form.role_id"
            filterable
            :disabled="isEdit"
            :placeholder="t('system.fieldPermission.dialog.placeholderRole')"
            style="width: 100%"
          >
            <el-option v-for="role in roleOptions" :key="role.id" :label="role.name" :value="role.id">
              <span>{{ role.name }}</span>
              <span class="role-option-code">{{ role.code }}</span>
            </el-option>
          </el-select>
        </el-form-item>
        <el-form-item :label="t('system.fieldPermission.dialog.labelResource')" prop="resource_type">
          <el-select
            v-model="form.resource_type"
            filterable
            :disabled="isEdit"
            :placeholder="t('system.fieldPermission.dialog.placeholderResource')"
            style="width: 100%"
          >
            <el-option
              v-for="rt in resourceTypeOptions"
              :key="rt"
              :label="resourceTypeLabel(rt)"
              :value="rt"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('system.fieldPermission.dialog.labelFieldName')" prop="field_name">
          <el-input
            v-model="form.field_name"
            :disabled="isEdit"
            :placeholder="t('system.fieldPermission.dialog.placeholderFieldName')"
          />
        </el-form-item>
        <el-form-item :label="t('system.fieldPermission.dialog.labelCanRead')">
          <el-switch v-model="form.can_read" :aria-label="t('system.fieldPermission.dialog.labelCanRead')" />
        </el-form-item>
        <el-form-item :label="t('system.fieldPermission.dialog.labelCanWrite')">
          <el-switch v-model="form.can_write" :aria-label="t('system.fieldPermission.dialog.labelCanWrite')" />
        </el-form-item>
        <el-form-item :label="t('system.fieldPermission.dialog.labelMaskStrategy')">
          <el-select v-model="form.mask_strategy" style="width: 100%">
            <el-option
              v-for="opt in MASK_STRATEGY_OPTIONS"
              :key="opt.value"
              :label="t(opt.labelKey)"
              :value="opt.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item v-if="isEdit" :label="t('system.fieldPermission.dialog.labelEnabled')">
          <el-switch
            v-model="form.is_enabled"
            :aria-label="t('system.fieldPermission.dialog.labelEnabled')"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ t('system.fieldPermission.button.cancel') }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="submitForm">{{
          t('system.fieldPermission.button.save')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import type { FormInstance, FormRules } from 'element-plus';
import { isDialogDismissal } from '@/utils/monitor';
import { logger, logAuxLoadFailure } from '@/utils/logger';
import { PERMISSIONS } from '@/constants/permissions';
import {
  getFieldPermissionList,
  createFieldPermission,
  updateFieldPermission,
  deleteFieldPermission,
  type FieldPermissionRow,
} from '@/api/field-permission';
import { getRoleList, type Role } from '@/api/role';
import { getResourceTypes } from '@/api/data-permission';
import {
  FALLBACK_DATA_PERMISSION_RESOURCE_TYPES,
  dataPermissionResourceTypeLabelKey,
} from '@/constants/data-permission-resource';

const { t } = useI18n({ useScope: 'global' });

const fieldPermissionList = ref<FieldPermissionRow[]>([]);
const fieldPermLoading = ref(false);
// 资源下拉候选的真相源是 GET /data-permissions/resource-types；取数失败回退到
// 与后端词表逐字一致的兜底常量（与数据权限页同一策略），下拉候选为空时
// 明确提示而不是静默空列表
const resourceTypeOptions = ref<string[]>([]);
const filterResourceType = ref('');

// 掩码策略取值域与后端消费侧同源：field_permission_service 判 "MASK" 置 "***"，
// 其余按不脱敏处理；候选不新增取值
const MASK_STRATEGY_OPTIONS: ReadonlyArray<{ value: string; labelKey: string }> = [
  { value: 'NONE', labelKey: 'system.fieldPermission.mask.none' },
  { value: 'MASK', labelKey: 'system.fieldPermission.mask.mask' },
];

/** 资源 token 展示文案：有词条走 i18n，无词条原样显示并 warn 一次（与权限管理页同口径） */
const warnedResourceTokens = new Set<string>();
const resourceTypeLabel = (value: string): string => {
  const labelKey = dataPermissionResourceTypeLabelKey(value);
  if (!labelKey && !warnedResourceTokens.has(value)) {
    warnedResourceTokens.add(value);
    logger.warn(`${t('system.fieldPermission.unknownResourceType')}${value}`);
  }
  return labelKey ? t(labelKey) : value;
};

/** 掩码策略展示：取值域外表原样显示并 warn，不猜含义 */
const warnedMaskTokens = new Set<string>();
const maskStrategyLabel = (value: string): string => {
  const opt = MASK_STRATEGY_OPTIONS.find(o => o.value === value);
  if (!opt) {
    if (!warnedMaskTokens.has(value)) {
      warnedMaskTokens.add(value);
      logger.warn(`${t('system.fieldPermission.unknownMaskStrategy')}${value}`);
    }
    return value;
  }
  return t(opt.labelKey);
};

const loadResourceTypes = async () => {
  try {
    const res = await getResourceTypes();
    if (Array.isArray(res.data) && res.data.length > 0) {
      resourceTypeOptions.value = res.data;
      return;
    }
    resourceTypeOptions.value = FALLBACK_DATA_PERMISSION_RESOURCE_TYPES;
    logger.warn(t('system.fieldPermission.messageResourceTypesEmpty'));
  } catch (e) {
    resourceTypeOptions.value = FALLBACK_DATA_PERMISSION_RESOURCE_TYPES;
    logAuxLoadFailure(t('system.fieldPermission.loadResourceTypesFailed'), e);
  }
};

const fetchFieldPermissions = async () => {
  fieldPermLoading.value = true;
  try {
    const res = await getFieldPermissionList(
      filterResourceType.value ? { resource_type: filterResourceType.value } : undefined
    );
    // 后端出参是裸数组载荷；非数组按契约异常显式失败，不兜底成空表继续渲染
    if (!Array.isArray(res.data)) {
      throw new Error(t('system.fieldPermission.loadListContractBroken'));
    }
    fieldPermissionList.value = res.data;
  } catch (e) {
    logAuxLoadFailure(t('system.fieldPermission.loadListFailed'), e);
    fieldPermissionList.value = [];
  } finally {
    fieldPermLoading.value = false;
  }
};

defineExpose({ refresh: fetchFieldPermissions });

// 对话框状态。isEdit 时定位三键（role/resource/field）只读——后端 PUT 载荷不含它们，
// 改定位键的语义是"换一条规则"，走 删除 + 新建，不提供隐式 upsert 入口。
const dialogVisible = ref(false);
const isEdit = ref(false);
const editId = ref(0);
const submitLoading = ref(false);
const formRef = ref<FormInstance>();
const roleOptions = ref<Role[]>([]);

const form = reactive({
  role_id: undefined as number | undefined,
  resource_type: '',
  field_name: '',
  can_read: true,
  can_write: false,
  mask_strategy: 'NONE',
  is_enabled: true,
});

// 字段名规则与库列 VARCHAR(100) 及"数据库列名风格"消费侧（field_mask 按键名精确匹配）对齐
const FIELD_NAME_PATTERN = /^[A-Za-z_][A-Za-z0-9_]{0,99}$/;
const formRules: FormRules = {
  role_id: [
    { required: true, message: t('system.fieldPermission.dialog.ruleRoleRequired'), trigger: 'change' },
  ],
  resource_type: [
    {
      required: true,
      message: t('system.fieldPermission.dialog.ruleResourceRequired'),
      trigger: 'change',
    },
  ],
  field_name: [
    {
      required: true,
      message: t('system.fieldPermission.dialog.ruleFieldNameRequired'),
      trigger: 'blur',
    },
    {
      pattern: FIELD_NAME_PATTERN,
      message: t('system.fieldPermission.dialog.ruleFieldNamePattern'),
      trigger: 'blur',
    },
  ],
};

const loadRoles = async () => {
  try {
    const res = await getRoleList();
    roleOptions.value = res.data.roles;
  } catch (e) {
    logAuxLoadFailure(t('system.fieldPermission.loadRolesFailed'), e);
    roleOptions.value = [];
  }
};

const resetForm = () => {
  Object.assign(form, {
    role_id: undefined,
    resource_type: '',
    field_name: '',
    can_read: true,
    can_write: false,
    mask_strategy: 'NONE',
    is_enabled: true,
  });
  formRef.value?.clearValidate();
};

const openCreateDialog = () => {
  isEdit.value = false;
  editId.value = 0;
  resetForm();
  dialogVisible.value = true;
};

const openEditDialog = (row: FieldPermissionRow) => {
  isEdit.value = true;
  editId.value = row.id;
  Object.assign(form, {
    role_id: row.role_id,
    resource_type: row.resource_type,
    field_name: row.field_name,
    can_read: row.can_read,
    can_write: row.can_write,
    mask_strategy: row.mask_strategy,
    is_enabled: row.is_enabled,
  });
  formRef.value?.clearValidate();
  dialogVisible.value = true;
};

/**
 * 提交：新增走全量 POST；编辑只 PUT 后端可变更的四键（read/write/mask/enabled）。
 * 成功判定以**读回**为准（与角色授权页同一"提示与落库真值一致"口径）：
 * 写请求 200 后重新拉列表，按主键核对提交值确实生效，读不回就点名失败。
 */
const submitForm = async () => {
  if (!formRef.value || submitLoading.value) return;
  submitLoading.value = true;
  try {
    const valid = await formRef.value.validate().catch(() => false);
    if (!valid) return;
    if (isEdit.value) {
      await updateFieldPermission(editId.value, {
        can_read: form.can_read,
        can_write: form.can_write,
        mask_strategy: form.mask_strategy,
        is_enabled: form.is_enabled,
      });
    } else {
      await createFieldPermission({
        role_id: form.role_id as number,
        resource_type: form.resource_type,
        field_name: form.field_name,
        can_read: form.can_read,
        can_write: form.can_write,
        mask_strategy: form.mask_strategy,
      });
    }
    await fetchFieldPermissions();
    const readBack = fieldPermissionList.value.find(row =>
      isEdit.value ? row.id === editId.value : matchesDraft(row)
    );
    if (!readBack || !valuesApplied(readBack)) {
      logger.error(
        `[字段权限] 写后读回不一致，${isEdit.value ? `id=${editId.value}` : form.field_name}`,
        fieldPermissionList.value
      );
      ElMessage.error(t('system.fieldPermission.messageVerifyFailed'));
      return;
    }
    ElMessage.success(t('system.fieldPermission.messageSaved'));
    dialogVisible.value = false;
  } catch (e: unknown) {
    const detail = backendMessageOf(e);
    ElMessage.error(
      detail
        ? `${t('system.fieldPermission.messageSaveFailed')}: ${detail}`
        : t('system.fieldPermission.messageSaveFailed')
    );
  } finally {
    submitLoading.value = false;
  }
};

/** 新增读回定位：列表按角色+资源+字段名匹配刚提交的唯一键组合 */
const matchesDraft = (row: FieldPermissionRow): boolean =>
  row.role_id === form.role_id &&
  row.resource_type === form.resource_type &&
  row.field_name === form.field_name;

/** 读回行与提交值逐键核对（四键全部一致才算生效） */
const valuesApplied = (row: FieldPermissionRow): boolean =>
  row.can_read === form.can_read &&
  row.can_write === form.can_write &&
  row.mask_strategy === form.mask_strategy &&
  row.is_enabled === form.is_enabled;

/** 取后端失败信封的 message（失败体 `{code,message,trace_id,timestamp}`），缺省空串 */
const backendMessageOf = (error: unknown): string => {
  const data = (error as { response?: { data?: { message?: unknown } } })?.response?.data;
  return typeof data?.message === 'string' ? data.message : '';
};

const removeFieldPermission = async (row: FieldPermissionRow) => {
  try {
    await ElMessageBox.confirm(
      t('system.fieldPermission.messageDeleteConfirm', {
        role: row.role_name || `#${row.role_id}`,
        field: `${resourceTypeLabel(row.resource_type)}.${row.field_name}`,
      }),
      t('system.fieldPermission.messageDeleteTitle'),
      { type: 'warning' }
    );
  } catch (error: unknown) {
    if (isDialogDismissal(error)) return;
    throw error;
  }
  try {
    await deleteFieldPermission(row.id);
    await fetchFieldPermissions();
    // 后端为软删（置 is_enabled=false）：读回确认该行已转停用或不在列表，
    // 仍存在且仍启用则点名失败，不空口报成功
    const stillActive = fieldPermissionList.value.find(r => r.id === row.id && r.is_enabled);
    if (stillActive) {
      ElMessage.error(t('system.fieldPermission.messageVerifyFailed'));
      return;
    }
    ElMessage.success(t('system.fieldPermission.messageDeleted'));
  } catch (e: unknown) {
    const detail = backendMessageOf(e);
    ElMessage.error(
      detail
        ? `${t('system.fieldPermission.messageDeleteFailed')}: ${detail}`
        : t('system.fieldPermission.messageDeleteFailed')
    );
  }
};

onMounted(() => {
  fetchFieldPermissions();
  loadRoles();
  loadResourceTypes();
});
</script>

<style scoped>
.field-permission-toolbar {
  margin-bottom: 12px;
}
.field-permission-filter {
  width: 220px;
}
.role-option-code {
  float: right;
  color: var(--el-text-color-secondary);
  font-size: 12px;
}
</style>
