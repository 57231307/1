<template>
  <div class="page">
    <el-card shadow="never">
      <template #header>
        <div class="card-header">
          <span>验布管理</span>
          <el-button type="primary" @click="dialogVisible = true">新建验布单</el-button>
        </div>
      </template>
      <el-table v-loading="loading" :data="inspections" border>
        <el-table-column prop="id" label="ID" width="70" />
        <el-table-column prop="inspection_no" label="验布单号" width="150" />
        <el-table-column label="状态" width="110">
          <template #default="{ row }">
            <el-tag :type="statusTag(row.status)">{{
              INSPECTION_STATUS_LABEL[row.status] ?? row.status
            }}</el-tag>
          </template>
        </el-table-column>
        <template v-for="col in extraCols" :key="col">
          <el-table-column
            :prop="col"
            :label="col.replace(/_/g, ' ')"
            min-width="130"
            show-overflow-tooltip
          />
        </template>
        <el-table-column label="操作" width="230" fixed="right">
          <template #default="{ row }">
            <el-button
              v-if="row.status !== 'closed'"
              size="small"
              type="primary"
              @click="onGrade(row)"
              >定级</el-button
            >
            <el-button
              v-if="row.status === 'graded'"
              size="small"
              type="success"
              @click="onClose(row)"
              >关闭</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
              size="small"
              type="danger"
              plain
              @click="onDelete(row)"
              >删除</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="dialogVisible" title="新建验布单" width="480">
      <el-form :model="form" label-width="100px">
        <el-form-item label="坯布批次">
          <el-input v-model="form.fabric_batch_no" />
        </el-form-item>
        <el-form-item label="米数">
          <el-input-number v-model="form.total_length_m" :min="0" :precision="1" class="w-full" />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="form.remarks" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="onCreate">保存</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="gradeVisible" title="验布定级" width="440">
      <el-form :model="gradeForm" label-width="100px">
        <el-form-item label="等级" required>
          <el-select v-model="gradeForm.grade" class="w-full">
            <el-option v-for="g in ['A', 'B', 'C', 'D']" :key="g" :label="g" :value="g" />
          </el-select>
        </el-form-item>
        <el-form-item label="评分">
          <el-input-number
            v-model="gradeForm.score"
            :min="0"
            :max="100"
            :precision="1"
            class="w-full"
          />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="gradeForm.remarks" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="gradeVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="onGradeSubmit">提交</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  closeFabricInspection,
  createFabricInspection,
  deleteFabricInspection,
  getFabricInspectionList,
  gradeFabricInspection,
  INSPECTION_STATUS_LABEL,
  type FabricInspection,
} from '@/api/fabric-inspection';

const inspections = ref<FabricInspection[]>([]);
const loading = ref(false);
const saving = ref(false);
const dialogVisible = ref(false);
const gradeVisible = ref(false);
const gradingId = ref<number | null>(null);

const form = reactive({
  fabric_batch_no: '',
  total_length_m: undefined as number | undefined,
  remarks: '',
});
const gradeForm = reactive({ grade: 'A', score: 90, remarks: '' });

const unwrapList = (p: unknown): FabricInspection[] =>
  Array.isArray(p) ? p : ((p as { items?: FabricInspection[] })?.items ?? []);

const statusTag = (s: string) =>
  ({ draft: 'info', inspecting: 'warning', graded: 'success', closed: 'info' })[s] ?? 'info';

const skipCols = new Set(['id', 'inspection_no', 'status', 'created_at', 'updated_at']);
const extraCols = computed(() => {
  const sample = inspections.value[0] ?? {};
  return Object.keys(sample)
    .filter(k => !skipCols.has(k) && typeof sample[k] !== 'object' && sample[k] !== null)
    .slice(0, 5);
});

async function load() {
  loading.value = true;
  try {
    inspections.value = unwrapList(await getFabricInspectionList());
  } finally {
    loading.value = false;
  }
}

async function onCreate() {
  saving.value = true;
  try {
    await createFabricInspection({
      fabric_batch_no: form.fabric_batch_no || undefined,
      total_length_m: form.total_length_m ?? undefined,
      remarks: form.remarks || undefined,
    });
    ElMessage.success('验布单已创建');
    dialogVisible.value = false;
    await load();
  } finally {
    saving.value = false;
  }
}

function onGrade(row: FabricInspection) {
  gradingId.value = row.id;
  gradeForm.grade = 'A';
  gradeForm.score = 90;
  gradeForm.remarks = '';
  gradeVisible.value = true;
}

async function onGradeSubmit() {
  if (gradingId.value === null) return;
  saving.value = true;
  try {
    await gradeFabricInspection(gradingId.value, {
      grade: gradeForm.grade,
      score: gradeForm.score,
      remarks: gradeForm.remarks || undefined,
    });
    ElMessage.success('定级完成');
    gradeVisible.value = false;
    await load();
  } finally {
    saving.value = false;
  }
}

async function onClose(row: FabricInspection) {
  await ElMessageBox.confirm('确认关闭该验布单？', '关闭确认');
  await closeFabricInspection(row.id);
  ElMessage.success('已关闭');
  await load();
}

async function onDelete(row: FabricInspection) {
  await ElMessageBox.confirm('确认删除该验布单？', '删除确认');
  await deleteFabricInspection(row.id);
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
