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
      // V15 P1-20-6 覆盖率门槛提升至 70%（批次 20 前端架构审计要求）
      // 未达门槛时 vitest run --coverage 退出非零码阻塞 CI
      thresholds: {
        // 全项目最低线（lines/funcs/branches/statements 均 70%）
        lines: 70,
        functions: 70,
        branches: 70,
        statements: 70,
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
