<template>
  <el-dropdown trigger="click" @command="onCommand">
    <span class="language-switcher">
      <el-icon><Compass /></el-icon>
      <span class="lang-text">{{ currentName }}</span>
      <el-icon><ArrowDown /></el-icon>
    </span>
    <template #dropdown>
      <el-dropdown-menu>
        <el-dropdown-item
          v-for="loc in locales"
          :key="loc.code"
          :command="loc.code"
          :disabled="loc.code === current"
        >
          <span class="lang-item">
            <span class="lang-native">{{ loc.nativeName }}</span>
            <el-icon v-if="loc.code === current" class="check-icon"><Check /></el-icon>
          </span>
        </el-dropdown-item>
      </el-dropdown-menu>
    </template>
  </el-dropdown>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { ArrowDown, Check, Compass } from '@element-plus/icons-vue'
import { getCurrentLocale, setLocale, SUPPORTED_LOCALES, type LocaleCode } from '../i18n'

/* 支持的语言列表 */
const locales = SUPPORTED_LOCALES

/* 当前语言 */
const current = computed<LocaleCode>(() => getCurrentLocale())

/* 当前语言显示名 */
const currentName = computed(() => {
  const loc = locales.find((l) => l.code === current.value)
  return loc ? loc.nativeName : current.value
})

/* 切换语言 */
function onCommand(code: string) {
  setLocale(code as LocaleCode)
}
</script>

<style scoped>
.language-switcher {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 12px;
  cursor: pointer;
  border-radius: 4px;
  transition: background-color 0.2s;
  font-size: 14px;
  color: var(--el-text-color-primary);
}

.language-switcher:hover {
  background-color: var(--el-fill-color-light);
}

.lang-text {
  font-weight: 500;
}

.lang-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-width: 120px;
}

.lang-native {
  font-size: 14px;
}

.check-icon {
  color: var(--el-color-primary);
  margin-left: 8px;
}
</style>
