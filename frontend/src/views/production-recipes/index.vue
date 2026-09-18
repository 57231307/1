<template>
  <div class="production-recipes-page">
    <el-card shadow="never">
      <div class="page-header">
        <h2>生产配方管理</h2>
        <div class="header-actions">
          <el-button type="primary" @click="handleCreate">新建配方</el-button>
          <el-button plain @click="calcVisible = true">配方试算</el-button>
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
        <el-table-column label="操作" width="280" fixed="right">
          <template #default="{ row }">
            <el-button size="small" link type="primary" @click="openDetail(row)">详情</el-button>
            <el-button
              v-if="row.status === 'draft'"
              size="small"
              link
              type="primary"
              @click="openEdit(row)"
              >编辑</el-button
            >
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

    <el-dialog
      v-model="dialogVisible"
      :title="editingId ? '编辑生产配方' : '新建生产配方'"
      width="520px"
    >
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
        <el-button type="primary" :loading="submitting" @click="submitForm">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="detailVisible" title="配方详情" width="760px">
      <el-descriptions v-if="detailRow" :column="2" border>
        <el-descriptions-item label="配方编号">{{ detailRow.recipe_no }}</el-descriptions-item>
        <el-descriptions-item label="配方名称">{{
          detailRow.recipe_name || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="产品">{{
          detailRow.product_name || detailRow.product_id
        }}</el-descriptions-item>
        <el-descriptions-item label="色号">{{ detailRow.color_no || '-' }}</el-descriptions-item>
        <el-descriptions-item label="染色类型">{{
          detailRow.dye_type || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="状态">{{ statusText(detailRow.status) }}</el-descriptions-item>
      </el-descriptions>

      <h4 class="section-title">加料处方</h4>
      <el-table v-loading="additionLoading" :data="additions" border size="small" max-height="260">
        <el-table-column prop="id" label="ID" width="70" />
        <el-table-column prop="addition_reason" label="加料原因" min-width="140">
          <template #default="{ row }">{{ row.addition_reason || '-' }}</template>
        </el-table-column>
        <el-table-column prop="total_cost" label="总成本" width="100">
          <template #default="{ row }">{{ row.total_cost ?? '-' }}</template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="100">
          <template #default="{ row }">{{ row.status || '-' }}</template>
        </el-table-column>
        <el-table-column label="操作" width="150" fixed="right">
          <template #default="{ row }">
            <el-button
              v-if="row.status === 'pending'"
              link
              type="success"
              size="small"
              @click="handleApproveAddition(row)"
              >审批</el-button
            >
            <el-button
              v-if="row.status === 'approved'"
              link
              type="warning"
              size="small"
              @click="handleCloseAddition(row)"
              >关闭</el-button
            >
          </template>
        </el-table-column>
      </el-table>
      <div class="toolbar" style="margin-top: 8px">
        <el-button type="primary" plain size="small" @click="additionDialogVisible = true">
          新增加料
        </el-button>
      </div>
      <template #footer>
        <el-button @click="detailVisible = false">关闭</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="additionDialogVisible" title="新增加料处方" width="560px">
      <el-form :model="additionForm" label-width="100px">
        <el-form-item label="加料原因">
          <el-input v-model="additionForm.addition_reason" placeholder="如：色光偏浅补料" />
        </el-form-item>
        <el-form-item label="物料明细" required>
          <el-input
            v-model="additionForm.detailText"
            type="textarea"
            :rows="4"
            placeholder='JSON 数组，如 [{"material_code":"M01","material_name":"元明粉","amount":120,"unit":"kg","category":"salt"}]'
          />
        </el-form-item>
        <el-form-item label="总成本">
          <el-input-number v-model="additionForm.total_cost" :min="0" :precision="2" />
        </el-form-item>
        <el-form-item label="备注"><el-input v-model="additionForm.remarks" /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="additionDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="additionSaving" @click="handleSaveAddition">
          保存
        </el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="calcVisible" title="配方试算（按备布重量/浴比）" width="600px">
      <el-form :model="calcForm" label-width="110px">
        <el-form-item label="备布重量 kg" required>
          <el-input-number v-model="calcForm.fabric_weight" :min="0.01" :precision="2" />
        </el-form-item>
        <el-form-item label="浴比" required>
          <el-input v-model="calcForm.liquor_ratio" placeholder="如 1:8" />
        </el-form-item>
        <el-form-item label="调整系数">
          <el-input-number v-model="calcForm.adjustment_factor" :min="0" :precision="2" />
        </el-form-item>
        <el-form-item label="物料明细" required>
          <el-input
            v-model="calcForm.itemsText"
            type="textarea"
            :rows="4"
            placeholder='JSON 数组，物料需含 concentration，如 [{"material_code":"M01","material_name":"活性红","concentration":2.5,"unit":"g/L","category":"dye"}]'
          />
        </el-form-item>
      </el-form>
      <el-table v-if="calcResult.length" :data="calcResult" border size="small" max-height="240">
        <el-table-column prop="material_code" label="物料编码" min-width="110" />
        <el-table-column prop="material_name" label="物料名称" min-width="120" />
        <el-table-column prop="amount" label="用量" width="100" />
        <el-table-column prop="unit" label="单位" width="80" />
      </el-table>
      <template #footer>
        <el-button @click="calcVisible = false">取消</el-button>
        <el-button type="primary" :loading="calcLoading" @click="handleCalculate"
          >开始试算</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue';
import { useUserStore } from '@/store/user';
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

// ===== 编辑（draft 态，updateProductionRecipe） =====
const editingId = ref<number | null>(null);

const openEdit = (row: ProductionRecipe) => {
  editingId.value = row.id;
  Object.assign(form, {
    recipe_name: row.recipe_name || '',
    product_id: row.product_id || 1,
    color_no: row.color_no || '',
    dye_type: row.dye_type || '',
    fabric_weight: 100,
    liquor_ratio: '1:10',
  });
  dialogVisible.value = true;
};

const submitForm = async () => {
  if (editingId.value) {
    submitting.value = true;
    try {
      await updateProductionRecipe(editingId.value, {
        recipe_name: form.recipe_name,
        product_id: form.product_id,
        color_no: form.color_no || undefined,
        dye_type: form.dye_type || undefined,
      });
      ElMessage.success('配方已更新');
      dialogVisible.value = false;
      editingId.value = null;
      await loadList();
    } catch (e) {
      ElMessage.error(`更新失败: ${(e as Error).message}`);
    } finally {
      submitting.value = false;
    }
  } else {
    await submitCreate();
  }
};

// ===== 详情回源 + 加料处方（getProductionRecipe/listRecipeAdditions/...） =====
const detailVisible = ref(false);
const detailRow = ref<ProductionRecipe | null>(null);
const additions = ref<Array<Record<string, unknown>>>([]);
const additionLoading = ref(false);
const additionDialogVisible = ref(false);
const additionSaving = ref(false);
const additionForm = reactive({
  addition_reason: '',
  detailText: '',
  total_cost: 0,
  remarks: '',
});

const openDetail = async (row: ProductionRecipe) => {
  detailRow.value = row;
  detailVisible.value = true;
  additionLoading.value = true;
  try {
    const res = await getProductionRecipe(row.id);
    if (res.data) detailRow.value = res.data;
  } catch {
    /* 回源失败保留行数据 */
  }
  await loadAdditions();
};

const loadAdditions = async () => {
  if (!detailRow.value) return;
  additionLoading.value = true;
  try {
    const res = await listRecipeAdditions(detailRow.value.id);
    const d = res.data as unknown;
    additions.value = Array.isArray(d)
      ? d
      : ((d as { items?: Array<Record<string, unknown>> })?.items ?? []);
  } finally {
    additionLoading.value = false;
  }
};

const handleSaveAddition = async () => {
  if (!detailRow.value) return;
  let detail: unknown;
  try {
    detail = JSON.parse(additionForm.detailText || '[]');
  } catch {
    ElMessage.warning('物料明细 JSON 格式有误');
    return;
  }
  if (!Array.isArray(detail) || detail.length === 0) {
    ElMessage.warning('请填写物料明细 JSON 数组');
    return;
  }
  additionSaving.value = true;
  try {
    await createRecipeAddition(detailRow.value.id, {
      production_recipe_id: detailRow.value.id,
      addition_reason: additionForm.addition_reason || undefined,
      addition_detail: detail as Array<{
        material_code: string;
        material_name: string;
        amount: number;
        unit: string;
        category: string;
      }>,
      total_cost: additionForm.total_cost || undefined,
      remarks: additionForm.remarks || undefined,
    });
    ElMessage.success('加料处方已创建');
    additionDialogVisible.value = false;
    await loadAdditions();
  } catch (e) {
    ElMessage.error(`创建失败: ${(e as Error).message}`);
  } finally {
    additionSaving.value = false;
  }
};

const handleApproveAddition = async (row: Record<string, unknown>) => {
  const userStore = useUserStore();
  const approvedBy = userStore.userInfo?.id;
  if (!approvedBy) {
    ElMessage.warning('当前登录用户信息缺失，无法登记审批人');
    return;
  }
  try {
    await approveRecipeAddition(row.id as number, { approved_by: approvedBy });
    ElMessage.success('加料处方已审批');
    await loadAdditions();
  } catch (e) {
    ElMessage.error(`审批失败: ${(e as Error).message}`);
  }
};

const handleCloseAddition = async (row: Record<string, unknown>) => {
  try {
    await closeRecipeAddition(row.id as number);
    ElMessage.success('加料处方已关闭');
    await loadAdditions();
  } catch (e) {
    ElMessage.error(`关闭失败: ${(e as Error).message}`);
  }
};

// ===== 配方试算（calculateRecipeAmounts） =====
const calcVisible = ref(false);
const calcLoading = ref(false);
const calcResult = ref<Array<Record<string, unknown>>>([]);
const calcForm = reactive({
  fabric_weight: 100,
  liquor_ratio: '1:10',
  adjustment_factor: 1,
  itemsText: '',
});

const handleCalculate = async () => {
  let items: unknown;
  try {
    items = JSON.parse(calcForm.itemsText || '[]');
  } catch {
    ElMessage.warning('物料明细 JSON 格式有误');
    return;
  }
  if (!Array.isArray(items) || items.length === 0) {
    ElMessage.warning('请填写物料明细 JSON 数组');
    return;
  }
  calcLoading.value = true;
  try {
    const res = await calculateRecipeAmounts({
      fabric_weight: calcForm.fabric_weight,
      liquor_ratio: calcForm.liquor_ratio,
      adjustment_factor: calcForm.adjustment_factor || undefined,
      items: items as import('@/api/production-recipe').RecipeMaterialItem[],
    });
    const d = res.data as unknown;
    calcResult.value = Array.isArray(d)
      ? d
      : ((d as { items?: Array<Record<string, unknown>> })?.items ?? []);
    ElMessage.success('试算完成');
  } catch (e) {
    ElMessage.error(`试算失败: ${(e as Error).message}`);
  } finally {
    calcLoading.value = false;
  }
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
.section-title {
  margin: 16px 0 8px;
  font-size: 14px;
}
.toolbar {
  display: flex;
  gap: 8px;
  align-items: center;
}
</style>
