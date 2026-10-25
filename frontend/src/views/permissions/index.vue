<!--
  权限管理页（路由 /permissions，门控 users:read）
  调用方：MainLayout 侧栏 权限治理 分组下的 权限管理 菜单项
  职责：按角色只读展示已授予的权限清单
-->
<template>
  <div class="permission-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('system.permission.title') }}</h2>
    </div>
    <el-card shadow="hover">
      <el-table
        v-loading="permissionListLoading"
        :data="permissionList"
        stripe
        :aria-label="t('system.permission.aria.list')"
      >
        <el-table-column
          prop="resource_type"
          :label="t('system.permission.column.resourceType')"
          width="150"
        >
          <template #default="{ row }">{{ resourceTypeLabel(row.resource_type) }}</template>
        </el-table-column>
        <el-table-column prop="action" :label="t('system.permission.column.action')" width="120">
          <template #default="{ row }">{{ actionLabel(row.action) }}</template>
        </el-table-column>
        <el-table-column
          prop="allowed"
          :label="t('system.permission.column.status')"
          width="100"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="row.allowed ? 'success' : 'danger'" size="small">
              {{
                row.allowed
                  ? t('system.permission.status.allowed')
                  : t('system.permission.status.denied')
              }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="resource_id"
          :label="t('system.permission.column.resourceId')"
          width="100"
        />
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { logger, logAuxLoadFailure } from '@/utils/logger';
import { getPermissionList, type Permission } from '@/api/role';
import { getResourceTypes } from '@/api/data-permission';
import { dataPermissionResourceTypeLabelKey } from '@/constants/data-permission-resource';
import { permissionActionLabelKey } from '@/constants/permission-action';

const { t } = useI18n({ useScope: 'global' });

// 行类型直接复用 api/role.ts 里对齐后端 PermissionResponse 的声明，页面不再自持一份
// 本地 interface —— 手抄字段必然与后端漂移。
const permissionList = ref<Permission[]>([]);
const permissionListLoading = ref(false);
// 后端资源词表（数据权限域端点 GET /data-permissions/resource-types 的裸数组出参），
// 用于把「后端已登记但前端无词条」的 token 对账出来；本页行本身的资源 token 来自
// GET /permissions 目录（资源注册表派生），词表取不到时不影响列表展示。
const resourceVocab = ref<string[]>([]);

/**
 * 资源 token 的展示文案：有词条走 i18n，无词条原样显示后端 token。
 * 词条来源与数据权限页同一份 constants 映射（token → 完整 i18n 键），页面内不写映射表。
 */
const resourceTypeLabel = (value: string): string => {
  const labelKey = dataPermissionResourceTypeLabelKey(value);
  return labelKey ? t(labelKey) : value;
};

/** 动作 token 的展示文案：取值域同源于 constants/permission-action，无词条时原样显示 */
const actionLabel = (value: string): string => {
  const labelKey = permissionActionLabelKey(value);
  return labelKey ? t(labelKey) : value;
};

// 同一 token 只点名一次，避免整表逐行刷屏把真实漂移淹掉
const warnedTokens = new Set<string>();

/**
 * 词表覆盖度对账：token 没有对应词条时按原码显示并在控制台留 warn。
 * 入参：待检查的资源 token；不写回任何兜底文案。
 */
const warnUnlabeledResource = (token: string): void => {
  if (dataPermissionResourceTypeLabelKey(token)) return;
  const mark = `resource:${token}`;
  if (warnedTokens.has(mark)) return;
  warnedTokens.add(mark);
  logger.warn(`${t('system.permission.unknownResourceType')}${token}`);
};

/** 动作 token 的同类对账，判据与提示口径与资源侧一致 */
const warnUnlabeledAction = (token: string): void => {
  if (permissionActionLabelKey(token)) return;
  const mark = `action:${token}`;
  if (warnedTokens.has(mark)) return;
  warnedTokens.add(mark);
  logger.warn(`${t('system.permission.unknownAction')}${token}`);
};

/**
 * 对账整张列表与后端资源词表：列表里出现的、以及词表登记的，只要没有词条都点名。
 * 出参：无（只产生日志），界面取值仍由 resourceTypeLabel/actionLabel 决定。
 */
const reportVocabCoverage = (rows: readonly Permission[]): void => {
  new Set(rows.map(row => row.resource_type)).forEach(warnUnlabeledResource);
  new Set(rows.map(row => row.action)).forEach(warnUnlabeledAction);
  resourceVocab.value.forEach(warnUnlabeledResource);
};

/**
 * 资源词表读取：它是覆盖度对账的辅助数据，取失败只让对账少一路输入并留痕，
 * 不把已取到的权限目录判成加载失败，也不兜底成一份前端自造的候选清单。
 * 出参：词表 token 数组；不可用时为空数组（空数组只用于"无词表可对"，不参与展示）。
 */
const loadResourceVocab = async (): Promise<string[]> => {
  try {
    const res = await getResourceTypes();
    if (!Array.isArray(res.data)) {
      logAuxLoadFailure(
        `${t('system.permission.loadResourceVocabFailed')}: response data is not an array`,
        res
      );
      return [];
    }
    return res.data;
  } catch (e) {
    logAuxLoadFailure(t('system.permission.loadResourceVocabFailed'), e);
    return [];
  }
};

// 后端目录端点出参是 `Json<Vec<PermissionResponse>>`（裸数组），拦截器返回 ApiResponse
// 完整信封，业务数组在 data 字段；把信封对象直接赋给 el-table :data 会整页崩溃，
// 因此非数组载荷按契约异常处理，不兜底成空表继续渲染。
const fetchPermissionList = async () => {
  permissionListLoading.value = true;
  try {
    const [rowsRes, vocab] = await Promise.all([getPermissionList(), loadResourceVocab()]);
    if (!Array.isArray(rowsRes.data)) {
      throw new Error(`${t('system.permission.loadListFailed')}: response data is not an array`);
    }
    permissionList.value = rowsRes.data;
    resourceVocab.value = vocab;
    reportVocabCoverage(permissionList.value);
  } catch (e) {
    logAuxLoadFailure(t('system.permission.loadListFailed'), e);
    permissionList.value = [];
  } finally {
    permissionListLoading.value = false;
  }
};

defineExpose({ refresh: fetchPermissionList });

onMounted(() => {
  fetchPermissionList();
});
</script>
