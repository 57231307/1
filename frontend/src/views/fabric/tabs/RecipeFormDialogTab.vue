<!--
  RecipeFormDialogTab.vue - 染色配方编辑对话框
  来源：原 fabric/index.vue 中 染色配方编辑对话框
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    :title="
      formData.id
        ? t('fabric.recipeFormDialog.titleEdit')
        : t('fabric.recipeFormDialog.titleCreate')
    "
    width="700px"
    :aria-label="
      formData.id
        ? t('fabric.recipeFormDialog.titleEdit')
        : t('fabric.recipeFormDialog.titleCreate')
    "
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-form
      ref="formRef"
      :model="formData"
      label-width="100px"
      :aria-label="t('fabric.recipeFormDialog.formAriaLabel')"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('fabric.recipeFormDialog.labelRecipeNo')" prop="recipe_no">
            <el-input v-model="formData.recipe_no" readonly />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('fabric.recipeFormDialog.labelName')" prop="recipe_name">
            <el-input v-model="formData.recipe_name" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('fabric.recipeFormDialog.labelColor')" prop="color_name">
            <el-input v-model="formData.color_name" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('fabric.recipeFormDialog.labelFabricType')" prop="fabric_type">
            <el-input v-model="formData.fabric_type" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('fabric.recipeFormDialog.labelContent')" prop="chemical_formula">
        <el-input
          v-model="formData.chemical_formula"
          type="textarea"
          :rows="6"
          :placeholder="t('fabric.recipeFormDialog.placeholderContent')"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:modelValue', false)">{{
        t('fabric.common.cancel')
      }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
        t('fabric.common.confirm')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import type { FormInstance } from 'element-plus';
import { createDyeRecipe, updateDyeRecipe, type DyeRecipe } from '@/api/dye-recipe';
import { generateUniqueDocNo } from '@/utils/document-no';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

interface Props {
  modelValue: boolean;
  currentRow: DyeRecipe | null;
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void;
  (e: 'submitted'): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();

const formRef = ref<FormInstance>();
const submitLoading = ref(false);

const formData = reactive({
  id: 0,
  recipe_no: '',
  recipe_name: '',
  color_name: '',
  fabric_type: '',
  // 后端 dye_recipe_handler.rs::CreateDyeRecipeRequest.version 为 Option<i32>；
  // 此前写成字符串 '1.0' → serde 反序列化整数失败 → 422，新建被拒（03-02 红根因）。
  // DyeRecipe.version 契约亦为 number，初值须与后端整数版本口径一致。
  version: 1 as number,
  chemical_formula: '',
  status: 'draft' as 'draft' | 'pending_approval' | 'approved' | 'disabled',
});

const resetForm = () => {
  formData.id = 0;
  formData.recipe_no = '';
  formData.recipe_name = '';
  formData.color_name = '';
  formData.fabric_type = '';
  formData.version = 1;
  formData.chemical_formula = '';
  formData.status = 'draft';
};

watch(
  () => props.modelValue,
  val => {
    if (val) {
      if (props.currentRow) {
        Object.assign(formData, props.currentRow);
      } else {
        resetForm();
        // 新建时预生成配方号（查重唯一后只读展示，防手动输入重复）；
        // fail-visible：取号抛错（查重接口异常或重试耗尽）必须用户可见，禁止静默留空
        initRecipeNo();
      }
    }
  }
);

/** 生成唯一配方号；失败即显式报错（号码留空时 handleSubmit 阻止提交） */
const initRecipeNo = async () => {
  try {
    formData.recipe_no = await generateUniqueDocNo('DR', 'dye_recipe');
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('fabric.recipeFormDialog.docNoGenerateFailed'));
    logger.error(t('fabric.recipeFormDialog.docNoGenerateFailed'), err.message);
  }
};

const handleSubmit = async () => {
  // 取号失败时配方号为空：阻止提交（后端 recipe_no NOT NULL，空号提交只会得到库级报错）
  if (!formData.id && !formData.recipe_no) {
    ElMessage.error(t('fabric.recipeFormDialog.docNoGenerateFailed'));
    return;
  }
  submitLoading.value = true;
  try {
    if (formData.id) {
      await updateDyeRecipe(formData.id, formData as unknown as Partial<DyeRecipe>);
    } else {
      await createDyeRecipe(formData as unknown as Partial<DyeRecipe>);
    }
    ElMessage.success(t('fabric.common.success'));
    emit('update:modelValue', false);
    emit('submitted');
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('fabric.common.failed'));
    logger.error(t('fabric.recipeFormDialog.saveFailed'), err.message);
  } finally {
    submitLoading.value = false;
  }
};
</script>
