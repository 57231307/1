<template>
  <div class="page">
    <el-card shadow="never">
      <template #header>
        <div class="card-header">
          <span>染化料管理</span>
          <el-button type="primary" @click="dialogVisible = true">新建染化料</el-button>
        </div>
      </template>
      <el-table v-loading="loading" :data="chemicals" border>
        <el-table-column prop="chemical_code" label="编码" width="130" />
        <el-table-column prop="chemical_name" label="名称" min-width="150" show-overflow-tooltip />
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

const unwrapList = (p: unknown): Chemical[] =>
  Array.isArray(p) ? p : ((p as { items?: Chemical[] })?.items ?? []);

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
