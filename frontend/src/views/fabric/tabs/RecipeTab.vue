<!--
  RecipeTab.vue - 染色配方 Tab
  来源：原 fabric/index.vue 中 染色配方 tab 内容
  拆分日期：2026-06-15 B3-4
-->
<template>
  <div class="recipe-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('fabric.recipeTab.title') }}</h2>
      <el-button type="primary" @click="openCreate">
        <el-icon><Plus /></el-icon>
        {{ t('fabric.recipeTab.buttonCreate') }}
      </el-button>
    </div>

    <el-card shadow="hover">
      <el-table
        v-loading="loading"
        :data="recipes"
        stripe
        :aria-label="t('fabric.recipeTab.tableAriaLabel')"
      >
        <el-table-column
          prop="recipe_no"
          :label="t('fabric.recipeTab.columnRecipeNo')"
          width="120"
        />
        <el-table-column prop="recipe_name" :label="t('fabric.recipeTab.columnName')" width="150" />
        <el-table-column prop="color_name" :label="t('fabric.recipeTab.columnColor')" width="120" />
        <el-table-column
          prop="fabric_type"
          :label="t('fabric.recipeTab.columnFabricType')"
          width="120"
        />
        <el-table-column prop="version" :label="t('fabric.recipeTab.columnVersion')" width="80" />
        <el-table-column
          prop="status"
          :label="t('fabric.recipeTab.columnStatus')"
          width="100"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="statusTagType(row.status)" size="small">
              {{ getStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="created_at"
          :label="t('fabric.recipeTab.columnCreatedAt')"
          width="160"
        />
        <el-table-column :label="t('fabric.recipeTab.columnAction')" width="240" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleView(row)">{{
              t('fabric.recipeTab.buttonView')
            }}</el-button>
            <el-button
              v-if="canApprove(row.status)"
              type="success"
              link
              size="small"
              @click="handleApprove(row)"
              >{{ t('fabric.recipeTab.buttonApprove') }}</el-button
            >
            <el-button
              v-if="row.status === DYE_RECIPE_STATUS.APPROVED"
              type="warning"
              link
              size="small"
              @click="handleNewVersion(row)"
              >{{ t('fabric.recipeTab.buttonNewVersion') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, defineEmits } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import {
  getDyeRecipe,
  approveDyeRecipe,
  createNewVersion as createNewVersionApi,
  DYE_RECIPE_STATUS,
  type DyeRecipe,
  type DyeRecipeStatus,
} from '@/api/dye-recipe';
import { useUserStore } from '@/store/user';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });
const userStore = useUserStore();

const emit = defineEmits<{ openDialog: [row: DyeRecipe | null] }>();

const recipes = ref<DyeRecipe[]>([]);
const loading = ref(false);

// 状态取值域与后端 models/status/quality_dyeing.rs::dye_recipe 词表同源（小写英文），
// 中文仅在此经 i18n 展示层映射。
const getStatusLabel = (status: DyeRecipeStatus): string => {
  const map: Record<DyeRecipeStatus, string> = {
    [DYE_RECIPE_STATUS.DRAFT]: t('fabric.recipeTab.statusDraft'),
    [DYE_RECIPE_STATUS.PENDING_APPROVAL]: t('fabric.recipeTab.statusPendingApproval'),
    [DYE_RECIPE_STATUS.APPROVED]: t('fabric.recipeTab.statusApproved'),
    [DYE_RECIPE_STATUS.DISABLED]: t('fabric.recipeTab.statusDisabled'),
  };
  return map[status];
};

const statusTagType = (status: DyeRecipeStatus): 'info' | 'warning' | 'success' | 'danger' => {
  const map: Record<DyeRecipeStatus, 'info' | 'warning' | 'success' | 'danger'> = {
    [DYE_RECIPE_STATUS.DRAFT]: 'info',
    [DYE_RECIPE_STATUS.PENDING_APPROVAL]: 'warning',
    [DYE_RECIPE_STATUS.APPROVED]: 'success',
    [DYE_RECIPE_STATUS.DISABLED]: 'danger',
  };
  return map[status];
};

// 审批按钮渲染条件与后端 validate_can_approve 同源：草稿或待审核均可审批。
const canApprove = (status: DyeRecipeStatus) =>
  status === DYE_RECIPE_STATUS.DRAFT || status === DYE_RECIPE_STATUS.PENDING_APPROVAL;

const fetchRecipes = async () => {
  loading.value = true;
  try {
    const { getDyeRecipeList } = await import('@/api/dye-recipe');
    const res = await getDyeRecipeList();
    // 后端 dye_recipe_handler.rs:66 返回 PaginatedResponse ⇒ data.items 是唯一形状；
    // 不做「数组或 {items}」双形状宽容，那会让契约漂移永远暴露不出来。
    recipes.value = res.data.items;
  } catch (error) {
    const err = error as Error;
    logger.error(t('fabric.recipeTab.fetchFailed'), err.message);
  } finally {
    loading.value = false;
  }
};

const openCreate = () => emit('openDialog', null);

const handleView = async (row: DyeRecipe) => {
  try {
    const res = await getDyeRecipe(row.id);
    emit('openDialog', (res.data as DyeRecipe | undefined) || null);
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('fabric.recipeTab.messageGetDetailFailed'));
  }
};

const handleApprove = async (row: DyeRecipe) => {
  try {
    await ElMessageBox.confirm(
      t('fabric.recipeTab.confirmApproveContent'),
      t('fabric.common.confirmTitle'),
      { type: 'info' }
    );
    // approved_by 取自登录用户真实 ID（参考本仓库其它审批入口，如 custom-orders/bpm）；
    // 取不到身份必须显式报错，不得用查询串/硬编码/默认值伪造。
    const approverId = userStore.userInfo?.id;
    if (!approverId) {
      ElMessage.error(t('fabric.recipeTab.messageNoUserInfo'));
      return;
    }
    await approveDyeRecipe(row.id, { approved_by: approverId });
    ElMessage.success(t('fabric.recipeTab.messageApproveSuccess'));
    fetchRecipes();
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error;
      ElMessage.error(err.message || t('fabric.recipeTab.messageApproveFailed'));
    }
  }
};

const handleNewVersion = async (row: DyeRecipe) => {
  try {
    await ElMessageBox.confirm(
      t('fabric.recipeTab.confirmNewVersionContent'),
      t('fabric.common.confirmTitle'),
      { type: 'info' }
    );
    await createNewVersionApi(row.id);
    ElMessage.success(t('fabric.recipeTab.messageNewVersionSuccess'));
    fetchRecipes();
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error;
      ElMessage.error(err.message || t('fabric.common.failed'));
    }
  }
};

onMounted(() => fetchRecipes());

defineExpose({ fetchRecipes });
</script>
