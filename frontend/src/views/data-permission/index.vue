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
            <el-menu-item v-for="role in roleList" :key="role.id" :index="String(role.id)">
              {{ role.name }}
            </el-menu-item>
          </el-menu>
        </div>

        <div class="permission-panel">
          <div class="panel-header">
            <h3>{{ currentRoleName }} - {{ t('dataPermission.index.titlePermission') }}</h3>
            <el-button type="primary" @click="handleAddPermission">{{
              t('dataPermission.index.buttonAddPermission')
            }}</el-button>
          </div>

          <el-table
            :data="permissionList"
            border
            stripe
            :aria-label="t('dataPermission.index.ariaTable')"
          >
            <el-table-column
              prop="resource_type"
              :label="t('dataPermission.index.colResourceType')"
            />
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
            <el-table-column :label="t('dataPermission.index.colOperation')" width="150">
              <template #default="{ row }">
                <el-button
                  v-permission="'data_permission:update'"
                  link
                  type="primary"
                  @click="handleEditPermission(row as DataPermissionRow)"
                  >{{ t('dataPermission.index.buttonEdit') }}</el-button
                >
                <el-button
                  v-permission="'data_permission:delete'"
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
            <el-option :label="t('dataPermission.index.optionCustomer')" value="customer" />
            <el-option :label="t('dataPermission.index.optionSupplier')" value="supplier" />
            <el-option :label="t('dataPermission.index.optionSalesOrder')" value="sales_order" />
            <el-option
              :label="t('dataPermission.index.optionPurchaseOrder')"
              value="purchase_order"
            />
            <el-option :label="t('dataPermission.index.optionInventory')" value="inventory" />
            <el-option :label="t('dataPermission.index.optionFinance')" value="finance" />
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
              :label="scope.label"
              :value="scope.value"
            >
              <span>{{ scope.label }}</span>
              <span style="color: #909399; font-size: 12px; margin-left: 8px">{{
                scope.description
              }}</span>
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
import { ref, reactive, onMounted, computed } from 'vue';
import { logger } from '@/utils/logger';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import {
  getRoleDataPermissionList,
  getDataPermissionByRole,
  setDataPermission,
  deleteDataPermissionByRole,
  getScopeTypeList,
  DEFAULT_SCOPE_TYPES,
  type DataPermissionRole,
  type ScopeType,
  type CustomCondition,
  type AllowedFields,
  type HiddenFields,
  type DataPermissionRow,
} from '@/api/data-permission';
import { getRoleList } from '@/api/role';

const { t } = useI18n({ useScope: 'global' });

const roleList = ref<Array<{ id: number; name: string }>>([]);
const selectedRoleId = ref('1');
const permissionList = ref<DataPermissionRole[]>([]);
const scopeTypeList = ref<ScopeType[]>([]);

// v11 P1-5 修复：动态加载角色列表，避免硬编码
const fetchRoles = async () => {
  try {
    const res = await getRoleList();
    if (res.data && Array.isArray(res.data)) {
      roleList.value = res.data.map(r => ({ id: r.id, name: r.name }));
      if (roleList.value.length > 0) {
        selectedRoleId.value = String(roleList.value[0].id);
      }
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
    const res = await getRoleDataPermissionList(parseInt(selectedRoleId.value));
    if (res.data) {
      // 安全检查：防止后端返回 data 为 null 时崩溃
      permissionList.value = res.data || [];
    }
  } catch (e) {
    ElMessage.error(t('dataPermission.index.messageLoadPermissionsFailed'));
  }
};

const fetchScopeTypes = async () => {
  try {
    const res = await getScopeTypeList();
    if (res.data && Array.isArray(res.data) && res.data.length > 0) {
      scopeTypeList.value = res.data;
    } else {
      scopeTypeList.value = DEFAULT_SCOPE_TYPES;
    }
  } catch (e) {
    // v11 P1-5 修复：API 失败时使用 API 层常量兜底，并告知用户
    scopeTypeList.value = DEFAULT_SCOPE_TYPES;
    const err = e as Error;
    ElMessage.warning(
      `${t('dataPermission.index.messageLoadScopeTypesFailed')}${err.message || t('dataPermission.index.messageUnknownError')}`
    );
  }
};

const handleSelectRole = (roleId: string) => {
  selectedRoleId.value = roleId;
  fetchPermissions();
};

const handleAddPermission = () => {
  isEdit.value = false;
  Object.assign(permissionForm, {
    roleId: parseInt(selectedRoleId.value),
    resourceType: '',
    scopeType: '',
    customCondition: '',
    allowedFields: '',
    hiddenFields: '',
  });
  permissionDialogVisible.value = true;
};

const handleEditPermission = async (row: DataPermissionRow) => {
  isEdit.value = true;
  let source: DataPermissionRole = row;
  try {
    // 编辑前按角色+资源类型回源最新权限配置
    const res = await getDataPermissionByRole(
      row.role_id ?? parseInt(selectedRoleId.value),
      row.resource_type ?? ''
    );
    if (res.data) source = res.data as DataPermissionRole;
  } catch (error) {
    logger.error(t('dataPermission.index.messageDetailFailed'), error);
  }
  Object.assign(permissionForm, {
    id: source.id,
    role_id: source.role_id,
    resource_type: source.resource_type,
    scope_type: source.scope_type,
    custom_condition: (source.custom_condition as unknown as string) || '',
    allowed_fields: (source.allowed_fields as unknown as string) || '',
    hidden_fields: (source.hidden_fields as unknown as string) || '',
  });
  permissionDialogVisible.value = true;
};

const handleSavePermission = async () => {
  if (!permissionFormRef.value) return;

  await permissionFormRef.value.validate(async (valid: boolean) => {
    if (!valid) return;

    // 表单字段为 JSON 文本，提交前解析为 API 契约对象类型（解析失败直接暴露错误）
    const parseCondition = (text: string): CustomCondition | undefined => {
      if (!text.trim()) return undefined;
      return JSON.parse(text) as CustomCondition;
    };
    const parseFieldList = (text: string): AllowedFields | undefined => {
      if (!text.trim()) return undefined;
      const arr = JSON.parse(text) as string[];
      return Array.isArray(arr) ? arr : undefined;
    };
    let customCondition: CustomCondition | undefined;
    let allowedFields: AllowedFields | undefined;
    let hiddenFields: HiddenFields | undefined;
    try {
      customCondition = parseCondition(permissionForm.custom_condition);
      allowedFields = parseFieldList(permissionForm.allowed_fields);
      hiddenFields = parseFieldList(permissionForm.hidden_fields);
    } catch {
      ElMessage.error(t('dataPermission.index.messageJsonInvalid'));
      return;
    }

    submitLoading.value = true;
    try {
      // set_data_permission 为 upsert（按 role_id + resource_type 幂等），编辑与新增同走 POST
      await setDataPermission({
        role_id: permissionForm.role_id!,
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
      // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
      ElMessage.error(
        (e instanceof Error ? e.message : String(e)) ||
          (isEdit.value
            ? t('dataPermission.index.messageUpdateFailed')
            : t('dataPermission.index.messageSaveFailed'))
      );
    } finally {
      submitLoading.value = false;
    }
  });
};

const handleDeletePermission = async (row: DataPermissionRow) => {
  const roleId = row.role_id ?? parseInt(selectedRoleId.value);
  const resourceType = row.resource_type ?? '';
  if (!roleId || !resourceType) return;

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

    // 按角色+资源类型删除
    await deleteDataPermissionByRole(roleId, resourceType);
    ElMessage.success(t('dataPermission.index.messageDeleteSuccess'));
    fetchPermissions();
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    if (e !== 'cancel') {
      ElMessage.error(
        (e instanceof Error ? e.message : String(e)) ||
          t('dataPermission.index.messageDeleteFailed')
      );
    }
  }
};

const hasLoaded = createLazyLoader();

onMounted(async () => {
  // 先加载角色列表（fetchRoles 会更新 selectedRoleId），再加载权限和范围类型
  await fetchRoles();
  fetchPermissions();
  loadIfNot('scopeTypes', fetchScopeTypes, hasLoaded);
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
</style>
