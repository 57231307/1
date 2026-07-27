<!--
  UserTab.vue - 用户管理 Tab
  来源：原 system/index.vue 第 10-84 行（template）+ 732-869 行（script）
  拆分日期：2026-06-05
  说明：本文件由 system/index.vue 拆分而来，逻辑完整可独立运行
-->
<template>
  <div class="user-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('system.user.title') }}</h2>
      <el-button type="primary" @click="openUserDialog()">
        <el-icon><Plus /></el-icon> {{ t('system.user.button.create') }}
      </el-button>
    </div>
    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="userQuery" :aria-label="t('system.user.filter.ariaLabel')">
        <el-form-item :label="t('system.user.filter.keyword')">
          <el-input
            v-model="userQuery.keyword"
            :placeholder="t('system.user.filter.keywordPlaceholder')"
            clearable
            @keyup.enter="fetchUsers"
          />
        </el-form-item>
        <el-form-item :label="t('system.user.filter.status')">
          <el-select
            v-model="userQuery.status"
            :placeholder="t('system.user.filter.statusPlaceholder')"
            clearable
          >
            <el-option :label="t('system.user.status.active')" :value="1" />
            <el-option :label="t('system.user.status.inactive')" :value="0" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">{{
            t('system.user.button.query')
          }}</el-button>
          <el-button @click="resetUserQuery">{{ t('system.user.button.reset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>
    <el-card shadow="hover">
      <el-table
        v-loading="loading"
        :data="users"
        stripe
        :aria-label="t('system.user.table.ariaLabel')"
      >
        <el-table-column prop="username" :label="t('system.user.table.username')" width="120" />
        <el-table-column prop="real_name" :label="t('system.user.table.realName')" width="100" />
        <el-table-column prop="phone" :label="t('system.user.table.phone')" width="130" />
        <el-table-column prop="email" :label="t('system.user.table.email')" min-width="180" />
        <el-table-column
          prop="department_name"
          :label="t('system.user.table.department')"
          width="120"
        />
        <el-table-column :label="t('system.user.table.role')" width="150">
          <template #default="{ row }">
            <template v-if="row.role_names?.length">
              <el-tag v-for="r in row.role_names" :key="r" size="small" class="mr-1">{{
                r
              }}</el-tag>
            </template>
            <span v-else class="text-gray">{{ t('system.user.role.unassigned') }}</span>
          </template>
        </el-table-column>
        <el-table-column
          prop="status"
          :label="t('system.user.table.status')"
          width="80"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="row.status === 1 ? 'success' : 'info'" size="small">
              {{
                row.status === 1 ? t('system.user.status.active') : t('system.user.status.inactive')
              }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_at" :label="t('system.user.table.createdAt')" width="160" />
        <el-table-column :label="t('system.user.table.operation')" width="150" fixed="right">
          <template #default="{ row }">
            <!-- P2-17 修复（批次 86 v2 复审）：编辑/删除按钮补齐 v-permission -->
            <!-- v11 批次 166 P2-1 修复：row as any 改为 row as User -->
            <el-button
              v-permission="PERMISSIONS.USER_UPDATE"
              size="small"
              link
              @click="openUserDialog(row as User)"
              >{{ t('system.user.button.edit') }}</el-button
            >
            <el-button
              v-permission="PERMISSIONS.USER_DELETE"
              size="small"
              link
              type="danger"
              @click="deleteUser(row as User)"
              >{{ t('system.user.button.delete') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
      <el-pagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        style="margin-top: 16px; justify-content: flex-end"
        :aria-label="t('system.user.table.paginationAriaLabel')"
        @current-change="handlePageChange"
        @size-change="handleSizeChange"
      />
    </el-card>

    <!-- 用户编辑对话框 -->
    <el-dialog
      v-model="userDialogVisible"
      :title="userForm.id ? t('system.user.dialog.editTitle') : t('system.user.dialog.createTitle')"
      width="600px"
      :aria-label="t('system.user.dialog.ariaLabel')"
    >
      <el-form
        ref="userFormRef"
        :model="userForm"
        :rules="userRules"
        label-width="100px"
        :aria-label="t('system.user.dialog.formAriaLabel')"
      >
        <el-form-item :label="t('system.user.dialog.username')" prop="username">
          <el-input v-model="userForm.username" :disabled="!!userForm.id" />
        </el-form-item>
        <el-form-item v-if="!userForm.id" :label="t('system.user.dialog.password')" prop="password">
          <el-input v-model="userForm.password" type="password" show-password />
        </el-form-item>
        <el-form-item :label="t('system.user.dialog.realName')" prop="real_name">
          <el-input v-model="userForm.real_name" />
        </el-form-item>
        <el-form-item :label="t('system.user.dialog.phone')" prop="phone">
          <el-input v-model="userForm.phone" />
        </el-form-item>
        <el-form-item :label="t('system.user.dialog.email')" prop="email">
          <el-input v-model="userForm.email" />
        </el-form-item>
        <el-form-item v-if="userForm.id" :label="t('system.user.dialog.status')">
          <el-switch v-model="userForm.status" :active-value="1" :inactive-value="0" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="userDialogVisible = false">{{
          t('system.user.button.cancel')
        }}</el-button>
        <el-button type="primary" :loading="userSubmitLoading" @click="submitUser">{{
          t('system.user.button.confirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import type { FormInstance, FormRules } from 'element-plus';
import { createUser, updateUser, deleteUser as deleteUserApi, type User } from '@/api/user';
import { useTableApi } from '@/composables/useTableApi';
// Batch 462 P0-S24：引入权限码常量，与后端 users 资源对齐
import { PERMISSIONS } from '@/constants/permissions';

// 批次 32 v7 P0-2：接入 i18n，替换硬编码中文 ElMessage
const { t } = useI18n({ useScope: 'global' });

const userQuery = reactive({
  keyword: '',
  status: undefined as number | undefined,
});

// 批次 276：接入 useTableApi，消除手写 users/userTotal/userLoading/fetchUsers 重复
// useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
const {
  data: users,
  loading,
  page,
  pageSize,
  total,
  refresh: fetchUsers,
  setQueryParam,
} = useTableApi<User>({
  url: '/users',
  defaultPageSize: 10,
  onError: (err: unknown) =>
    ElMessage.error(
      (err instanceof Error ? err.message : String(err)) || t('system.user.message.loadListFailed')
    ),
});

// 批次 276：同步筛选条件到 useTableApi.queryParams 并刷新
const syncQueryParams = () => {
  setQueryParam('keyword', userQuery.keyword || undefined);
  setQueryParam('status', userQuery.status);
};

const handleQuery = () => {
  syncQueryParams();
  page.value = 1;
  fetchUsers();
};

const resetUserQuery = () => {
  userQuery.keyword = '';
  userQuery.status = undefined;
  syncQueryParams();
  page.value = 1;
  fetchUsers();
};

// 分页（useTableApi 自动 watch page/pageSize 变化触发重载）
const handlePageChange = (p: number) => {
  page.value = p;
};

const handleSizeChange = (s: number) => {
  pageSize.value = s;
  page.value = 1;
};

defineExpose({ refresh: fetchUsers });

const userDialogVisible = ref(false);
const userFormRef = ref<FormInstance>();
const userSubmitLoading = ref(false);
const userForm = reactive({
  id: 0,
  username: '',
  password: '',
  real_name: '',
  phone: '',
  email: '',
  department_id: undefined as number | undefined,
  status: 1,
});

// v11 批次 166 P2-1 修复：validator 参数类型化（FormItemRule validator 签名）
const validateEmail = (_rule: unknown, v: string, cb: (error?: Error) => void) => {
  v && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(v)
    ? cb(new Error(t('system.user.validation.emailFormat')))
    : cb();
};
const validatePhone = (_rule: unknown, v: string, cb: (error?: Error) => void) => {
  v && !/^1[3-9]\d{9}$/.test(v) ? cb(new Error(t('system.user.validation.phoneFormat'))) : cb();
};
const validatePassword = (_rule: unknown, v: string, cb: (error?: Error) => void) => {
  if (userForm.id && !v) {
    cb();
    return;
  }
  v && v.length < 8
    ? cb(new Error(t('system.user.validation.passwordMinLength')))
    : v && !/^(?=.*[a-z])(?=.*[A-Z])(?=.*\d).+$/.test(v)
      ? cb(new Error(t('system.user.validation.passwordComplexity')))
      : cb();
};

const userRules: FormRules = {
  username: [
    { required: true, message: t('system.user.validation.usernameRequired'), trigger: 'blur' },
    { min: 3, max: 20, message: t('system.user.validation.usernameLength'), trigger: 'blur' },
  ],
  password: [{ required: true, validator: validatePassword, trigger: 'blur' }],
  real_name: [
    { required: true, message: t('system.user.validation.realNameRequired'), trigger: 'blur' },
  ],
  email: [{ validator: validateEmail, trigger: 'blur' }],
  phone: [{ validator: validatePhone, trigger: 'blur' }],
};

const openUserDialog = (row?: User) => {
  userFormRef.value?.resetFields();
  if (row) {
    Object.assign(userForm, {
      id: row.id,
      username: row.username,
      real_name: row.real_name,
      phone: row.phone || '',
      email: row.email || '',
      department_id: row.department_id,
      status: row.status,
    });
  } else {
    Object.assign(userForm, {
      id: 0,
      username: '',
      password: '',
      real_name: '',
      phone: '',
      email: '',
      department_id: undefined,
      status: 1,
    });
  }
  userDialogVisible.value = true;
};

const submitUser = async () => {
  const valid = await userFormRef.value?.validate();
  if (!valid) return;
  userSubmitLoading.value = true;
  try {
    if (userForm.id) {
      await updateUser(userForm.id, {
        real_name: userForm.real_name,
        phone: userForm.phone,
        email: userForm.email,
        department_id: userForm.department_id,
        status: userForm.status,
      });
      ElMessage.success(t('settings.user.updateSuccess'));
    } else {
      await createUser({
        username: userForm.username,
        password: userForm.password,
        real_name: userForm.real_name,
        phone: userForm.phone,
        email: userForm.email,
        department_id: userForm.department_id,
      });
      ElMessage.success(t('settings.user.createSuccess'));
    }
    userDialogVisible.value = false;
    fetchUsers();
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) || t('system.user.message.operationFailed')
    );
  } finally {
    userSubmitLoading.value = false;
  }
};

const deleteUser = async (row: User) => {
  try {
    await ElMessageBox.confirm(
      t('system.user.message.deleteConfirm', { name: row.username }),
      t('system.user.message.deleteConfirmTitle'),
      { type: 'warning' }
    );
    await deleteUserApi(row.id);
    ElMessage.success(t('settings.user.deleteSuccess'));
    fetchUsers();
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    if (e !== 'cancel')
      ElMessage.error(
        (e instanceof Error ? e.message : String(e)) || t('system.user.message.deleteFailed')
      );
  }
};
</script>
