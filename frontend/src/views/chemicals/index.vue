<template>
  <div class="page">
    <el-tabs v-model="activeTab">
      <el-tab-pane label="染化料" name="chemicals">
        <el-card shadow="never">
          <template #header>
            <div class="card-header">
              <span>染化料管理</span>
              <el-button type="primary" @click="dialogVisible = true">新建染化料</el-button>
            </div>
          </template>
          <el-table v-loading="loading" :data="chemicals" border>
            <el-table-column prop="chemical_code" label="编码" width="130" />
            <el-table-column
              prop="chemical_name"
              label="名称"
              min-width="150"
              show-overflow-tooltip
            />
            <el-table-column prop="chemical_type" label="类型" width="110" />
            <el-table-column prop="cas_number" label="CAS 号" width="130" />
            <template v-for="col in extraCols" :key="col">
              <el-table-column
                :prop="col"
                :label="col.replace(/_/g, ' ')"
                min-width="130"
                show-overflow-tooltip
              />
            </template>
            <el-table-column label="操作" width="150" fixed="right">
              <template #default="{ row }">
                <el-button size="small" @click="onEdit(row)">编辑</el-button>
                <el-button size="small" type="danger" plain @click="onDelete(row)">删除</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="批次管理" name="lots">
        <div class="card-header" style="margin-bottom: 12px">
          <el-button type="primary" @click="lotDialogVisible = true">新建批次</el-button>
        </div>
        <el-table v-loading="lotLoading" :data="chemicalLots" border>
          <el-table-column prop="id" label="ID" width="70" />
          <el-table-column prop="lot_no" label="批次号" min-width="140" />
          <el-table-column prop="chemical_code" label="染化料编码" width="140" />
          <el-table-column prop="lot_date" label="批次日期" width="120" />
          <el-table-column prop="quantity" label="数量" width="110" align="right">
            <template #default="{ row }">{{ Number(row.quantity ?? 0).toLocaleString() }}</template>
          </el-table-column>
          <el-table-column prop="status" label="状态" width="100" />
          <el-table-column label="操作" width="150" fixed="right">
            <template #default="{ row }">
              <el-button size="small" @click="onEditLot(row)">编辑</el-button>
              <el-button size="small" type="danger" plain @click="onDeleteLot(row)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <el-tab-pane label="分类管理" name="categories">
        <div class="card-header" style="margin-bottom: 12px">
          <el-button type="primary" @click="categoryDialogVisible = true">新建分类</el-button>
        </div>
        <el-table v-loading="categoryLoading" :data="chemicalCategories" border>
          <el-table-column prop="id" label="ID" width="70" />
          <el-table-column prop="name" label="分类名称" min-width="160" />
          <el-table-column prop="parent_id" label="父分类 ID" width="110" />
          <el-table-column label="操作" width="150" fixed="right">
            <template #default="{ row }">
              <el-button size="small" @click="onEditCategory(row)">编辑</el-button>
              <el-button size="small" type="danger" plain @click="onDeleteCategory(row)"
                >删除</el-button
              >
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>
    </el-tabs>

    <el-dialog v-model="dialogVisible" :title="editingId ? '编辑染化料' : '新建染化料'" width="520">
      <el-form :model="form" label-width="110px">
        <el-form-item label="编码" required>
          <el-input v-model="form.chemical_code" :disabled="!!editingId" />
        </el-form-item>
        <el-form-item label="名称" required>
          <el-input v-model="form.chemical_name" />
        </el-form-item>
        <el-form-item label="英文名">
          <el-input v-model="form.chemical_name_en" />
        </el-form-item>
        <el-form-item label="类型" required>
          <el-select v-model="form.chemical_type" class="w-full">
            <el-option label="染料" value="dye" />
            <el-option label="助剂" value="auxiliary" />
            <el-option label="其他" value="other" />
          </el-select>
        </el-form-item>
        <el-form-item label="CAS 号">
          <el-input v-model="form.cas_number" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="onSave">保存</el-button>
      </template>
    </el-dialog>

    <!-- 新建/编辑批次 -->
    <el-dialog
      v-model="lotDialogVisible"
      :title="editingLotId ? '编辑批次' : '新建批次'"
      width="480"
    >
      <el-form :model="lotForm" label-width="110px">
        <el-form-item label="批次号" required>
          <el-input v-model="lotForm.lot_no" />
        </el-form-item>
        <el-form-item label="染化料编码" required>
          <el-input v-model="lotForm.chemical_code" />
        </el-form-item>
        <el-form-item label="批次日期">
          <el-date-picker v-model="lotForm.lot_date" type="date" value-format="YYYY-MM-DD" />
        </el-form-item>
        <el-form-item label="数量">
          <el-input-number v-model="lotForm.quantity" :min="0" :precision="2" />
        </el-form-item>
        <el-form-item label="状态">
          <el-input v-model="lotForm.status" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="lotDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="lotSaving" @click="onSaveLot">保存</el-button>
      </template>
    </el-dialog>

    <!-- 新建/编辑分类 -->
    <el-dialog
      v-model="categoryDialogVisible"
      :title="editingCategoryId ? '编辑分类' : '新建分类'"
      width="440"
    >
      <el-form :model="categoryForm" label-width="110px">
        <el-form-item label="分类名称" required>
          <el-input v-model="categoryForm.name" />
        </el-form-item>
        <el-form-item label="父分类 ID">
          <el-input-number v-model="categoryForm.parent_id" :min="0" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="categoryDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="categorySaving" @click="onSaveCategory">
          保存
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  createChemical,
  deleteChemical,
  getChemicalList,
  updateChemical,
  getChemicalLotList,
  createChemicalLot,
  updateChemicalLot,
  deleteChemicalLot,
  getChemicalCategoryList,
  createChemicalCategory,
  updateChemicalCategory,
  deleteChemicalCategory,
  type Chemical,
} from '@/api/chemical';

const chemicals = ref<Chemical[]>([]);
const loading = ref(false);
const saving = ref(false);
const dialogVisible = ref(false);
const editingId = ref<number | null>(null);

const form = reactive({
  chemical_code: '',
  chemical_name: '',
  chemical_name_en: '',
  chemical_type: 'dye',
  cas_number: '',
});

const unwrapList = (p: unknown): Chemical[] => (p as { data: { items: Chemical[] } }).data.items;

const skipCols = new Set([
  'id',
  'chemical_code',
  'chemical_name',
  'chemical_type',
  'cas_number',
  'created_at',
  'updated_at',
]);
const extraCols = computed(() => {
  const sample = chemicals.value[0] ?? {};
  return Object.keys(sample)
    .filter(k => !skipCols.has(k) && typeof sample[k] !== 'object' && sample[k] !== null)
    .slice(0, 4);
});

async function load() {
  loading.value = true;
  try {
    chemicals.value = unwrapList(await getChemicalList());
  } finally {
    loading.value = false;
  }
}

async function onSave() {
  if (!form.chemical_code || !form.chemical_name || !form.chemical_type) {
    ElMessage.warning('请填写编码/名称/类型');
    return;
  }
  saving.value = true;
  try {
    const payload = {
      chemical_code: form.chemical_code,
      chemical_name: form.chemical_name,
      chemical_name_en: form.chemical_name_en || undefined,
      chemical_type: form.chemical_type,
      cas_number: form.cas_number || undefined,
    };
    if (editingId.value) {
      await updateChemical(editingId.value, payload);
      ElMessage.success('已更新');
    } else {
      await createChemical(payload);
      ElMessage.success('已创建');
    }
    dialogVisible.value = false;
    editingId.value = null;
    await load();
  } finally {
    saving.value = false;
  }
}

function onEdit(row: Chemical) {
  editingId.value = row.id;
  form.chemical_code = row.chemical_code;
  form.chemical_name = row.chemical_name;
  form.chemical_name_en = (row.chemical_name_en as string) ?? '';
  form.chemical_type = row.chemical_type;
  form.cas_number = (row.cas_number as string) ?? '';
  dialogVisible.value = true;
}

async function onDelete(row: Chemical) {
  await ElMessageBox.confirm(`确认删除 ${row.chemical_name}？`, '删除确认');
  await deleteChemical(row.id);
  ElMessage.success('已删除');
  await load();
}

onMounted(load);

// ==================== 批次管理（chemical-lots） ====================
const activeTab = ref('chemicals');
const chemicalLots = ref<Array<Record<string, unknown>>>([]);
const lotLoading = ref(false);
const lotDialogVisible = ref(false);
const lotSaving = ref(false);
const editingLotId = ref<number | null>(null);
const lotForm = reactive({
  lot_no: '',
  chemical_code: '',
  lot_date: new Date().toISOString().split('T')[0],
  quantity: 0,
  status: 'available',
});

const loadLots = async () => {
  lotLoading.value = true;
  try {
    const res = await getChemicalLotList();
    chemicalLots.value = res.data.items;
  } catch (e) {
    ElMessage.error((e as Error).message || '加载批次失败');
  } finally {
    lotLoading.value = false;
  }
};

const onSaveLot = async () => {
  if (!lotForm.lot_no || !lotForm.chemical_code) {
    ElMessage.warning('批次号与染化料编码必填');
    return;
  }
  lotSaving.value = true;
  try {
    if (editingLotId.value) {
      await updateChemicalLot(editingLotId.value, lotForm);
    } else {
      await createChemicalLot(lotForm);
    }
    ElMessage.success('保存成功');
    lotDialogVisible.value = false;
    loadLots();
  } catch (e) {
    ElMessage.error((e as Error).message || '保存失败');
  } finally {
    lotSaving.value = false;
  }
};

const onEditLot = (row: Record<string, unknown>) => {
  editingLotId.value = Number(row.id);
  lotForm.lot_no = String(row.lot_no || '');
  lotForm.chemical_code = String(row.chemical_code || '');
  lotForm.lot_date = String(row.lot_date || new Date().toISOString().split('T')[0]);
  lotForm.quantity = Number(row.quantity ?? 0);
  lotForm.status = String(row.status || 'available');
  lotDialogVisible.value = true;
};

const onDeleteLot = async (row: Record<string, unknown>) => {
  try {
    await ElMessageBox.confirm(`确认删除批次 ${row.lot_no}？`, '删除确认', { type: 'warning' });
  } catch {
    return;
  }
  try {
    await deleteChemicalLot(Number(row.id));
    ElMessage.success('删除成功');
    loadLots();
  } catch (e) {
    ElMessage.error((e as Error).message || '删除失败');
  }
};

// ==================== 分类管理（chemical-categories） ====================
const chemicalCategories = ref<Array<Record<string, unknown>>>([]);
const categoryLoading = ref(false);
const categoryDialogVisible = ref(false);
const categorySaving = ref(false);
const editingCategoryId = ref<number | null>(null);
const categoryForm = reactive({ name: '', parent_id: 0 });

const loadCategories = async () => {
  categoryLoading.value = true;
  try {
    const res = await getChemicalCategoryList();
    chemicalCategories.value = res.data.items;
  } catch (e) {
    ElMessage.error((e as Error).message || '加载分类失败');
  } finally {
    categoryLoading.value = false;
  }
};

const onSaveCategory = async () => {
  if (!categoryForm.name) {
    ElMessage.warning('分类名称必填');
    return;
  }
  categorySaving.value = true;
  try {
    if (editingCategoryId.value) {
      await updateChemicalCategory(editingCategoryId.value, categoryForm);
    } else {
      await createChemicalCategory(categoryForm);
    }
    ElMessage.success('保存成功');
    categoryDialogVisible.value = false;
    loadCategories();
  } catch (e) {
    ElMessage.error((e as Error).message || '保存失败');
  } finally {
    categorySaving.value = false;
  }
};

const onEditCategory = (row: Record<string, unknown>) => {
  editingCategoryId.value = Number(row.id);
  categoryForm.name = String(row.name || '');
  categoryForm.parent_id = Number(row.parent_id ?? 0);
  categoryDialogVisible.value = true;
};

const onDeleteCategory = async (row: Record<string, unknown>) => {
  try {
    await ElMessageBox.confirm(`确认删除分类「${row.name}」？`, '删除确认', { type: 'warning' });
  } catch {
    return;
  }
  try {
    await deleteChemicalCategory(Number(row.id));
    ElMessage.success('删除成功');
    loadCategories();
  } catch (e) {
    ElMessage.error((e as Error).message || '删除失败');
  }
};
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.w-full {
  width: 100%;
}
</style>
