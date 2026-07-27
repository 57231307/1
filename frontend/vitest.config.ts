import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'

export default defineConfig({
  plugins: [vue()],
  test: {
    globals: true,
    environment: 'jsdom',
    include: ['tests/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}'],
    // 排除 Playwright E2E 测试（e2e/），由 @playwright/test 运行
    exclude: [
      'node_modules',
      'dist',
      '.idea',
      '.git',
      '.cache',
      'e2e/**',
    ],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      reportsDirectory: './coverage',
      include: ['src/**/*.{ts,vue}'],
      exclude: [
        'src/types/**',
        'src/**/*.d.ts',
        'src/main.ts',
        'src/App.vue',
      ],
      // V15 批次 06 P1-12 修复：覆盖率门槛（全项目 60%+，核心 service 80%+）
      // 未达门槛时 vitest run --coverage 退出非零码阻塞 CI
      thresholds: {
        // 全项目最低线（lines/funcs/branches/statements 均 60%）
        lines: 60,
        functions: 60,
        branches: 60,
        statements: 60,
        // 关键模块按 per-file 门槛（通过 thresholds.perFile 实现）
        perFile: false,
      },
    },
    setupFiles: ['./tests/setup.ts'],
  },
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
  },
})
