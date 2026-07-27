<!--
  FieldPermissionTab.vue - 字段权限 Tab
  来源：原 system/index.vue 中 字段权限 tab 内容
  拆分日期：2026-06-15 B3-1
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
          prop="visible"
          :label="t('system.fieldPermission.column.visible')"
          width="80"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="row.visible ? 'success' : 'danger'" size="small">
              {{
                row.visible
                  ? t('system.fieldPermission.common.yes')
                  : t('system.fieldPermission.common.no')
              }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="editable"
          :label="t('system.fieldPermission.column.editable')"
          width="80"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="row.editable ? 'success' : 'info'" size="small">
              {{
                row.editable
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
import { useI18n } from 'vue-i18n';
import { request } from '@/api/request';

const { t } = useI18n({ useScope: 'global' });

interface FieldPermissionRow {
  role_name: string;
  resource_type: string;
  field_name: string;
  visible: boolean;
  editable: boolean;
}

const fieldPermissionList = ref<FieldPermissionRow[]>([]);
const fieldPermLoading = ref(false);

const fetchFieldPermissions = async () => {
  fieldPermLoading.value = true;
  try {
    const res = await request.get<{ items?: FieldPermissionRow[] } | FieldPermissionRow[]>(
      '/permissions/fields'
    );
    const d = res;
    if (d && typeof d === 'object' && 'items' in d) {
      fieldPermissionList.value = d.items || [];
    } else {
      fieldPermissionList.value = (d as FieldPermissionRow[]) || [];
    }
  } catch (_e) {
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
