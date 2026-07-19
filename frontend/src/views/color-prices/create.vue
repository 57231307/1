<!--
  面料多色号定价扩展 - 新建价格页
  提交后调用 color-price.ts:createColorPrice，成功后跳转到列表页
  创建时间: 2026-06-19
  关联 spec: docs/superpowers/specs/2026-06-16-color-price-extension-design.md §3.2
-->
<template>
  <div class="color-price-create">
    <el-card>
      <template #header>
        <span>新建色号价格</span>
      </template>

      <el-form
        :model="form"
        :rules="rules"
        ref="formRef"
        label-width="120px"
        style="max-width: 800px"
        aria-label="色卡价格创建表单"
      >
        <el-form-item label="产品 ID" prop="product_id">
          <el-input v-model.number="form.product_id" placeholder="产品 ID" />
        </el-form-item>
        <el-form-item label="色号 ID" prop="color_id">
          <el-input v-model.number="form.color_id" placeholder="色号 ID" />
        </el-form-item>
        <el-form-item label="币种" prop="currency">
          <el-select v-model="form.currency" placeholder="请选择" style="width: 100%">
            <el-option label="人民币 CNY" value="CNY" />
            <el-option label="美元 USD" value="USD" />
            <el-option label="欧元 EUR" value="EUR" />
          </el-select>
        </el-form-item>
        <el-form-item label="基础价" prop="base_price">
          <el-input v-model.number="form.base_price" placeholder="基础价" />
        </el-form-item>
        <el-form-item label="生效日期" prop="effective_from">
          <el-date-picker
            v-model="form.effective_from"
            type="date"
            value-format="YYYY-MM-DD"
            placeholder="选择生效日期"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="失效日期">
          <el-date-picker
            v-model="form.effective_to"
            type="date"
            value-format="YYYY-MM-DD"
            placeholder="可选：失效日期"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="客户等级">
          <el-select v-model="form.customer_level" placeholder="可选：客户等级" clearable style="width: 100%">
            <el-option label="VIP" value="VIP" />
            <el-option label="GOLD" value="GOLD" />
            <el-option label="SILVER" value="SILVER" />
            <el-option label="NORMAL" value="NORMAL" />
          </el-select>
        </el-form-item>
        <el-form-item label="季节">
          <el-select v-model="form.season" placeholder="可选：季节" clearable style="width: 100%">
            <el-option label="春夏 SS" value="SS" />
            <el-option label="秋冬 AW" value="AW" />
            <el-option label="节日 HOLIDAY" value="HOLIDAY" />
          </el-select>
        </el-form-item>
        <el-form-item label="客户 ID">
          <el-input v-model.number="form.customer_id" placeholder="可选：客户专属价对应的客户 ID" />
        </el-form-item>
        <el-form-item label="起订量">
          <el-input v-model.number="form.min_quantity" placeholder="可选：阶梯价起订量" />
        </el-form-item>
        <el-form-item label="限订量">
          <el-input v-model.number="form.max_quantity" placeholder="可选：阶梯价上限" />
        </el-form-item>
        <el-form-item label="优先级">
          <el-input-number v-model="form.priority" :min="0" :max="1000" style="width: 100%" />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="form.notes" type="textarea" :rows="3" placeholder="可选：备注" />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :loading="submitting" @click="handleSubmit">立即创建</el-button>
          <el-button @click="$router.back()">取消</el-button>
        </el-form-item>
      </el-form>
    </el-card>
  </div>
</template>

<script setup lang="ts">
// 新建色号价格页
// 提交逻辑：调用 createColorPrice，成功后跳转回列表页
import { ref, reactive } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, FormInstance, FormRules } from 'element-plus'
import { createColorPrice, type CreateColorPriceDto } from '@/api/color-price'

const router = useRouter()
const formRef = ref<FormInstance>()
const submitting = ref(false)

// 表单状态：与 CreateColorPriceDto 对齐
const form = reactive<{
  product_id: number | undefined
  color_id: number | undefined
  currency: string
  base_price: number | undefined
  effective_from: string
  effective_to: string | null
  customer_level: string | null
  season: string | null
  customer_id: number | null
  min_quantity: number | null
  max_quantity: number | null
  priority: number
  notes: string | null
}>({
  product_id: undefined,
  color_id: undefined,
  currency: 'CNY',
  base_price: undefined,
  effective_from: '',
  effective_to: null,
  customer_level: null,
  season: null,
  customer_id: null,
  min_quantity: null,
  max_quantity: null,
  priority: 0,
  notes: null,
})

// 表单校验规则
const rules: FormRules = {
  product_id: [{ required: true, message: '请输入产品 ID', trigger: 'blur' }],
  color_id: [{ required: true, message: '请输入色号 ID', trigger: 'blur' }],
  currency: [{ required: true, message: '请选择币种', trigger: 'change' }],
  base_price: [{ required: true, message: '请输入基础价', trigger: 'blur' }],
  effective_from: [{ required: true, message: '请选择生效日期', trigger: 'change' }],
}

// 提交处理：调用创建 API，成功后跳转到列表页
const handleSubmit = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async (valid) => {
    if (!valid) return
    submitting.value = true
    try {
      const payload: CreateColorPriceDto = {
        product_id: form.product_id!,
        color_id: form.color_id!,
        currency: form.currency,
        base_price: form.base_price!,
        effective_from: form.effective_from,
        effective_to: form.effective_to,
        customer_level: form.customer_level,
        season: form.season,
        customer_id: form.customer_id,
        min_quantity: form.min_quantity,
        max_quantity: form.max_quantity,
        priority: form.priority,
        notes: form.notes,
      }
      await createColorPrice(payload)
      ElMessage.success('创建成功')
      router.push('/color-prices/list')
    } catch (e: unknown) {
      ElMessage.error('创建失败：' + (e instanceof Error ? e.message : '未知错误'))
    } finally {
      submitting.value = false
    }
  })
}
</script>

<style scoped>
.color-price-create { padding: 20px; }
</style>
