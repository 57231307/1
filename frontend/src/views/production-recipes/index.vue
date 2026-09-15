<template>
  <div class="production-recipes-page">
    <el-card shadow="never">
      <div class="page-header">
        <h2>生产配方管理</h2>
        <div class="header-actions">
          <el-button type="primary" @click="handleCreate">新建配方</el-button>
        </div>
      </div>

      <el-table v-loading="loading" :data="list" border>
        <el-table-column prop="recipe_no" label="配方编号" min-width="160" />
        <el-table-column prop="recipe_name" label="配方名称" min-width="140">
          <template #default="{ row }">{{ row.recipe_name || '-' }}</template>
        </el-table-column>
        <el-table-column prop="product_name" label="产品" min-width="120">
          <template #default="{ row }">{{ row.product_name || '-' }}</template>
        </el-table-column>
        <el-table-column prop="color_no" label="色号" width="120">
          <template #default="{ row }">{{ row.color_no || '-' }}</template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="110" align="center">
          <template #default="{ row }">
            <el-tag :type="statusTag(row.status)">{{ statusText(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="240" fixed="right">
          <template #default="{ row }">
            <el-button
              v-if="row.status === 'draft'"
              size="small"
              link
              type="success"
              @click="handleApprove(row)"
              >审批</el-button
            >
            <el-button
              v-if="row.status === 'draft' || row.status === 'approved'"
              size="small"
              link
              type="warning"
              @click="handleClose(row)"
              >关闭</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
              size="small"
              link
              type="danger"
              @click="handleCancel(row)"
              >取消</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
              size="small"
              link
              type="danger"
              @click="handleDelete(row)"
              >删除</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="dialogVisible" title="新建生产配方" width="520px">
      <el-form :model="form" label-width="90px">
        <el-form-item label="配方名称" required>
          <el-input v-model="form.recipe_name" />
        </el-form-item>
        <el-form-item label="产品 ID" required>
          <el-input-number v-model="form.product_id" :min="1" style="width: 100%" />
        </el-form-item>
        <el-form-item label="色号">
          <el-input v-model="form.color_no" />
        </el-form-item>
        <el-form-item label="染色类型">
          <el-input v-model="form.dye_type" placeholder="如：活性染料" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="submitCreate">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  getProductionRecipeList,
  createProductionRecipe,
  deleteProductionRecipe,
  approveProductionRecipe,
  closeProductionRecipe,
  cancelProductionRecipe,
  type ProductionRecipe,
  type ProductionRecipeStatus,
} from '@/api/production-recipe';

const loading = ref(false);
const submitting = ref(false);
const dialogVisible = ref(false);
const list = ref<ProductionRecipe[]>([]);

const form = reactive({
  recipe_name: '',
  product_id: 1,
  color_no: '',
  dye_type: '',
  fabric_weight: 100,
  liquor_ratio: '1:10',
});

const statusText = (s: string) =>
  ({ draft: '草稿', approved: '已审核', closed: '已关闭', cancelled: '已取消' })[s] ?? s;
const statusTag = (s: ProductionRecipeStatus | string) =>
  (({ approved: 'success', closed: 'info', cancelled: 'danger' }) as Record<string, string>)[s] ??
  'warning';

const unwrap = (payload: unknown): ProductionRecipe[] => {
  const p = payload as unknown;
  return Array.isArray(p) ? p : ((p as { items?: ProductionRecipe[] })?.items ?? []);
};

const loadList = async () => {
  loading.value = true;
  try {
    const res = await getProductionRecipeList({ page: 1, page_size: 50 });
    list.value = unwrap(res.data);
  } catch (e) {
    ElMessage.error(`加载配方列表失败: ${(e as Error).message}`);
  } finally {
    loading.value = false;
  }
};

const handleCreate = () => {
  Object.assign(form, {
    recipe_name: '',
    product_id: 1,
    color_no: '',
    dye_type: '',
    fabric_weight: 100,
    liquor_ratio: '1:10',
  });
  dialogVisible.value = true;
};

const submitCreate = async () => {
  if (!form.recipe_name) {
    ElMessage.warning('配方名称必填');
    return;
  }
  submitting.value = true;
  try {
    await createProductionRecipe(form);
    ElMessage.success('配方已创建');
    dialogVisible.value = false;
    await loadList();
  } catch (e) {
    ElMessage.error(`创建失败: ${(e as Error).message}`);
  } finally {
    submitting.value = false;
  }
};

const runAction = async (_row: ProductionRecipe, label: string, fn: () => Promise<unknown>) => {
  try {
    await fn();
    ElMessage.success(`${label}成功`);
    await loadList();
  } catch (e) {
    ElMessage.error(`${label}失败: ${(e as Error).message}`);
  }
};

const handleApprove = (row: ProductionRecipe) =>
  runAction(row, '审批', () => approveProductionRecipe(row.id, { approved_by: 1 }));
const handleClose = (row: ProductionRecipe) =>
  runAction(row, '关闭', () => closeProductionRecipe(row.id));
const handleCancel = (row: ProductionRecipe) =>
  runAction(row, '取消', () => cancelProductionRecipe(row.id));
const handleDelete = async (row: ProductionRecipe) => {
  await ElMessageBox.confirm(`确认删除配方 ${row.recipe_no}?`, '删除确认', { type: 'warning' });
  await runAction(row, '删除', () => deleteProductionRecipe(row.id));
};

onMounted(() => {
  loadList();
});
</script>

<style scoped>
.production-recipes-page {
  padding: 20px;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}
.page-header h2 {
  margin: 0;
  font-size: 18px;
}
.header-actions {
  display: flex;
  gap: 12px;
  align-items: center;
}
</style>
