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
      // V15 P1-20-6 覆盖率门槛（当前 1.78%，逐步提升至 70%）
      // 2026-08-11: 提升至 2%，防止覆盖率回退
      thresholds: {
        lines: 2,
        functions: 2,
        branches: 2,
        statements: 2,
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
