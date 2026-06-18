<!--
  密码强度可视化组件
  - 输入：password（双向绑定）
  - 输出：strength（0-4 强度等级）、feedback（改进建议）、color（颜色）
  - 算法：长度+大写+小写+数字+特殊字符 各加 1 分，封顶 4 分
  - 复用：修改密码页面、注册/重置密码页面
-->
<template>
  <div v-if="password.length > 0" class="pwd-strength">
    <div class="pwd-strength-bar">
      <el-progress
        :percentage="strengthPct"
        :color="color"
        :stroke-width="8"
        :show-text="false"
      />
    </div>
    <div class="pwd-strength-label" :style="{ color }">
      强度：{{ strengthText }}
    </div>
    <ul v-if="feedback.length > 0" class="pwd-strength-feedback">
      <li v-for="(tip, idx) in feedback" :key="idx">{{ tip }}</li>
    </ul>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  /** 双向绑定的密码字符串 */
  password: string
}

const props = defineProps<Props>()
const emit = defineEmits<{
  (e: 'update:password', value: string): void
}>()

/**
 * 简化版密码强度计算（不调后端）
 * - 长度 < 8：直接 0 分（极弱，过短无法补救）
 * - 长度 ≥ 8：进入评分
 *   - 长度 ≥ 12：+1
 *   - 含大写字母：+1
 *   - 含小写字母：+1
 *   - 含数字：+1
 *   - 含特殊字符：+1
 *   - 累计 0-4 分（封顶 4）
 */
const strengthScore = computed<number>(() => {
  const pwd = props.password || ''
  if (!pwd) return 0
  // 长度 < 8 视为极弱（即使字符类齐全也救不回来）
  if (pwd.length < 8) return 0
  let score = 0
  if (pwd.length >= 12) score += 1
  if (/[A-Z]/.test(pwd)) score += 1
  if (/[a-z]/.test(pwd)) score += 1
  if (/[0-9]/.test(pwd)) score += 1
  if (/[^A-Za-z0-9]/.test(pwd)) score += 1
  return Math.min(score, 4)
})

/** 强度等级对应的颜色：极弱红、弱橙、中黄、强青、极强绿 */
const color = computed<string>(() => {
  switch (strengthScore.value) {
    case 0:
    case 1:
      return '#f56c6c' // 极弱/弱：红
    case 2:
      return '#e6a23c' // 中：橙
    case 3:
      return '#67c23a' // 强：绿
    case 4:
      return '#409eff' // 极强：蓝
    default:
      return '#909399'
  }
})

/** 强度等级文字标签 */
const strengthText = computed<string>(() => {
  switch (strengthScore.value) {
    case 0:
      return '极弱'
    case 1:
      return '弱'
    case 2:
      return '中'
    case 3:
      return '强'
    case 4:
      return '极强'
    default:
      return ''
  }
})

/** 进度条百分比：score 0-4 映射到 0-100 */
const strengthPct = computed<number>(() => {
  return Math.min(strengthScore.value * 25, 100)
})

/** 改进建议：列出未满足的维度 */
const feedback = computed<string[]>(() => {
  const pwd = props.password || ''
  const tips: string[] = []
  if (pwd.length < 8) {
    tips.push('建议密码长度至少 8 个字符（推荐 12 位以上）')
  } else if (pwd.length < 12) {
    tips.push('建议密码长度提升到 12 位以上以获得更高强度')
  }
  if (!/[A-Z]/.test(pwd)) tips.push('加入大写字母')
  if (!/[a-z]/.test(pwd)) tips.push('加入小写字母')
  if (!/[0-9]/.test(pwd)) tips.push('加入数字')
  if (!/[^A-Za-z0-9]/.test(pwd)) tips.push('加入特殊字符（如 !@#$%）')
  return tips
})

/** 保持 v-model 双向绑定（预留 emit 以便父组件 v-model） */
const _emit = emit
void _emit
</script>

<style scoped>
.pwd-strength {
  margin-top: 4px;
}

.pwd-strength-bar {
  width: 100%;
}

.pwd-strength-label {
  margin-top: 4px;
  font-size: 12px;
  font-weight: 500;
}

.pwd-strength-feedback {
  margin: 6px 0 0 0;
  padding-left: 18px;
  font-size: 12px;
  color: #909399;
  line-height: 1.6;
}

.pwd-strength-feedback li {
  list-style: disc;
}
</style>
