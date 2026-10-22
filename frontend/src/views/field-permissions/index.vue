<!--
  字段权限页（路由 /field-permissions，门控 users:read）
  调用方：MainLayout 侧栏 权限治理 分组下的 字段权限 菜单项
  职责：只读展示各角色的字段级可读与可写配置
-->
<template>
  <div class="field-permission-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('system.fieldPermission.title') }}</h2>
    </div>
    <el-card shadow="hover">
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
        />
        <el-table-column
          prop="resource_type"
          :label="t('system.fieldPermission.column.resource')"
          width="120"
        />
        <el-table-column
          prop="field_name"
          :label="t('system.fieldPermission.column.fieldName')"
          width="150"
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
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { logAuxLoadFailure } from '@/utils/logger';
import { useI18n } from 'vue-i18n';
import { request } from '@/api/request';
import type { ApiResponse } from '@/types/api';

const { t } = useI18n({ useScope: 'global' });

interface FieldPermissionRow {
  // 后端富化：role_name 来自 LEFT JOIN role（无匹配角色时为 null）
  role_name: string | null;
  resource_type: string;
  field_name: string;
  can_read: boolean;
  can_write: boolean;
}

const fieldPermissionList = ref<FieldPermissionRow[]>([]);
const fieldPermLoading = ref(false);

const fetchFieldPermissions = async () => {
  fieldPermLoading.value = true;
  try {
    const res = await request.get<ApiResponse<unknown[]>>('/field-permissions');
    // 拦截器返回 ApiResponse 信封，业务数组在 data 字段（信封对象直赋表格会触发 rows not iterable 崩溃）
    fieldPermissionList.value = (res.data as FieldPermissionRow[]) ?? [];
  } catch (_e) {
    logAuxLoadFailure(t('system.fieldPermission.loadListFailed'), _e);
    fieldPermissionList.value = [];
  } finally {
    fieldPermLoading.value = false;
  }
};

defineExpose({ refresh: fetchFieldPermissions });

onMounted(() => {
  fetchFieldPermissions();
});
</script>
