/**
 * Playwright 网络拦截 / Mock / 弱网模拟工具集
 *
 * 批次 262：针对项目 E2E 测试增强，提供以下能力：
 * - 网络拦截（route）：拦截指定 URL 的请求并自定义响应
 * - Mock 接口能力：模拟后端异常返回（500/403/超时/网络错误）
 * - 弱网环境模拟：通过延迟响应模拟慢速网络
 * - 请求观察（爬虫/RPA 类）：记录请求与响应供断言
 *
 * 设计原则：
 * - 所有 mock 函数支持 context 级别（多上下文隔离）与 page 级别
 * - 弱网模拟使用 route.fulfill 前置 delay，不依赖浏览器原生网络节流
 * - 异常模拟覆盖后端业务错误码（code:非200）与 HTTP 状态码两种场景
 */
import type { BrowserContext, Page, Route } from '@playwright/test'

/**
 * 项目统一 API 响应结构（与后端 AppError 脱敏响应一致）
 */
export interface ApiResponse<T = unknown> {
  code: number | string
  message: string
  data: T | null
  timestamp?: string
}

/**
 * Mock 异常返回配置
 */
export interface MockErrorOptions {
  /** HTTP 状态码（默认 500） */
  status?: number
  /** 业务错误码（写入响应体 code 字段） */
  errorCode?: string | number
  /** 业务错误消息（写入响应体 message 字段） */
  errorMessage?: string
  /** 响应延迟毫秒数（模拟慢速异常返回） */
  delayMs?: number
}

/**
 * Mock 成功返回配置
 */
export interface MockSuccessOptions<T = unknown> {
  /** HTTP 状态码（默认 200） */
  status?: number
  /** 业务数据（写入响应体 data 字段） */
  data: T
  /** 业务消息（默认 'success'） */
  message?: string
  /** 响应延迟毫秒数（模拟慢速成功返回） */
  delayMs?: number
}

/**
 * 构造项目统一 API 响应体
 */
function buildApiResponse<T>(options: {
  code: number | string
  message: string
  data: T | null
}): ApiResponse<T> {
  return {
    code: options.code,
    message: options.message,
    data: options.data,
    timestamp: new Date().toISOString(),
  }
}

/**
 * 应用延迟（模拟弱网 / 慢速返回）
 */
async function applyDelay(delayMs: number): Promise<void> {
  if (delayMs > 0) {
    await new Promise((resolve) => setTimeout(resolve, delayMs))
  }
}

/**
 * Mock 指定 URL 的后端异常返回
 *
 * 使用场景：
 * - 测试前端对后端 500 错误的兜底处理
 * - 测试前端对 403 权限不足的提示
 * - 测试前端对业务错误码（如 VALIDATION_ERROR）的处理
 *
 * @example
 * // 模拟销售订单接口返回 500
 * await mockApiError(context, '**/api/v1/erp/sales/orders**', {
 *   status: 500,
 *   errorCode: 'INTERNAL_ERROR',
 *   errorMessage: '服务器内部错误',
 * })
 */
export async function mockApiError(
  context: BrowserContext | Page,
  urlPattern: string,
  options: MockErrorOptions = {}
): Promise<void> {
  const {
    status = 500,
    errorCode = 'INTERNAL_ERROR',
    errorMessage = '模拟的后端异常返回',
    delayMs = 0,
  } = options

  await context.route(urlPattern, async (route: Route) => {
    await applyDelay(delayMs)
    await route.fulfill({
      status,
      contentType: 'application/json',
      body: JSON.stringify(
        buildApiResponse({ code: errorCode, message: errorMessage, data: null })
      ),
    })
  })
}

/**
 * Mock 指定 URL 的后端成功返回
 *
 * 使用场景：
 * - 测试前端对特定数据结构的渲染（无需真实后端数据）
 * - 测试前端对边界数据（空列表/超大列表）的处理
 *
 * @example
 * // Mock 销售订单列表返回空分页
 * await mockApiSuccess(context, '**/api/v1/erp/sales/orders**', {
 *   data: { items: [], total: 0, page: 1, page_size: 20 },
 * })
 */
export async function mockApiSuccess<T = unknown>(
  context: BrowserContext | Page,
  urlPattern: string,
  options: MockSuccessOptions<T>
): Promise<void> {
  const { status = 200, data, message = 'success', delayMs = 0 } = options

  await context.route(urlPattern, async (route: Route) => {
    await applyDelay(delayMs)
    await route.fulfill({
      status,
      contentType: 'application/json',
      body: JSON.stringify(buildApiResponse({ code: 200, message, data })),
    })
  })
}

/**
 * 模拟网络请求超时（不返回任何响应，让请求挂起）
 *
 * 使用场景：
 * - 测试前端对请求超时的兜底处理（loading 状态、超时提示）
 * - 测试前端对弱网下长时间未响应的处理
 *
 * 实现说明：通过 route.abort 模拟网络层错误，比挂起更可控
 * （挂起会导致测试自身超时）。如需测试"长时间未响应"，配合 delayMs。
 *
 * @example
 * // 模拟销售订单接口网络中断
 * await mockNetworkFailure(context, '**/api/v1/erp/sales/orders**')
 */
export async function mockNetworkFailure(
  context: BrowserContext | Page,
  urlPattern: string
): Promise<void> {
  await context.route(urlPattern, (route: Route) => {
    route.abort('failed')
  })
}

/**
 * 模拟弱网环境（对所有匹配 URL 的请求增加延迟）
 *
 * 使用场景：
 * - 测试前端 loading 状态是否正确显示
 * - 测试前端防重复提交（慢速返回下用户多次点击）
 * - 测试前端超时降级策略
 *
 * 注意：此函数会放行请求到真实后端，仅增加延迟。
 * 如需 mock 数据 + 延迟，请使用 mockApiSuccess/mockApiError 的 delayMs 参数。
 *
 * @example
 * // 模拟所有 API 请求延迟 2 秒（弱网）
 * await simulateSlowNetwork(context, '**/api/v1/erp/**', 2000)
 */
export async function simulateSlowNetwork(
  context: BrowserContext | Page,
  urlPattern: string,
  delayMs: number
): Promise<void> {
  await context.route(urlPattern, async (route: Route) => {
    await applyDelay(delayMs)
    await route.continue()
  })
}

/**
 * 网络请求观察器（爬虫/RPA 类：记录请求与响应供断言）
 *
 * 使用场景：
 * - 验证前端是否发起了预期的 API 请求（如点击按钮后触发的请求）
 * - 验证请求参数是否正确（如分页参数、过滤条件）
 * - 爬虫类场景：批量收集接口响应数据
 *
 * @example
 * const observer = observeRequests(page, '**/api/v1/erp/sales/orders**')
 * await page.click('button:has-text("查询")')
 * const requests = await observer.collect()
 * expect(requests.length).toBeGreaterThan(0)
 */
export class RequestObserver {
  private readonly requests: Array<{
    url: string
    method: string
    status: number
    body: string | null
  }> = []

  constructor(
    private readonly context: BrowserContext | Page,
    private readonly urlPattern: string
  ) {}

  /**
   * 启动观察（注册 route handler，放行请求但记录响应）
   */
  async start(): Promise<void> {
    await this.context.route(this.urlPattern, async (route: Route) => {
      const request = route.request()
      const response = await route.fetch()
      let body: string | null = null
      try {
        body = await response.text()
      } catch {
        body = null
      }
      this.requests.push({
        url: request.url(),
        method: request.method(),
        status: response.status(),
        body,
      })
      await route.fulfill({ response })
    })
  }

  /**
   * 收集已观察到的请求记录
   */
  async collect(): Promise<typeof this.requests> {
    // 等待一个微任务周期，确保所有 pending route handler 完成
    await new Promise((resolve) => setTimeout(resolve, 100))
    return [...this.requests]
  }

  /**
   * 停止观察（取消 route handler）
   */
  async stop(): Promise<void> {
    await this.context.unroute(this.urlPattern)
  }
}

/**
 * 创建网络请求观察器（便捷工厂函数）
 */
export function observeRequests(
  context: BrowserContext | Page,
  urlPattern: string
): RequestObserver {
  return new RequestObserver(context, urlPattern)
}

/**
 * 等待指定 API 调用完成（等待响应）
 *
 * 使用场景：
 * - 点击按钮后等待对应的 API 请求完成
 * - 表单提交后等待创建接口返回
 *
 * @example
 * await page.click('button:has-text("保存")')
 * await waitForApiCall(page, '**/api/v1/erp/sales/orders**')
 */
export async function waitForApiCall(
  page: Page,
  urlPattern: string,
  timeout = 10_000
): Promise<void> {
  await page.waitForResponse(urlPattern, { timeout })
}

/**
 * 一次性 Mock：仅拦截下一次匹配的请求，后续放行
 *
 * 使用场景：
 * - 测试单次提交场景（第一次提交返回错误，重试返回成功）
 *
 * @example
 * // 第一次创建返回 500，重试放行到真实后端
 * await mockOnce(context, '**/api/v1/erp/sales/orders**', {
 *   status: 500,
 *   errorCode: 'INTERNAL_ERROR',
 * })
 */
export async function mockOnce(
  context: BrowserContext | Page,
  urlPattern: string,
  options: MockErrorOptions
): Promise<void> {
  let consumed = false
  await context.route(urlPattern, async (route: Route) => {
    if (consumed) {
      await route.continue()
      return
    }
    consumed = true
    await applyDelay(options.delayMs ?? 0)
    await route.fulfill({
      status: options.status ?? 500,
      contentType: 'application/json',
      body: JSON.stringify(
        buildApiResponse({
          code: options.errorCode ?? 'INTERNAL_ERROR',
          message: options.errorMessage ?? '模拟的一次性异常',
          data: null,
        })
      ),
    })
  })
}
