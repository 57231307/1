/**
 * 响应式设计工具函数
 */

// 断点定义
export const breakpoints = {
  xs: 0,
  sm: 576,
  md: 768,
  lg: 992,
  xl: 1200,
  xxl: 1400,
} as const

export type Breakpoint = keyof typeof breakpoints

/**
 * 获取当前断点
 */
export function getCurrentBreakpoint(): Breakpoint {
  const width = window.innerWidth
  if (width >= breakpoints.xxl) return 'xxl'
  if (width >= breakpoints.xl) return 'xl'
  if (width >= breakpoints.lg) return 'lg'
  if (width >= breakpoints.md) return 'md'
  if (width >= breakpoints.sm) return 'sm'
  return 'xs'
}

/**
 * 检查是否匹配断点
 */
export function matchesBreakpoint(breakpoint: Breakpoint): boolean {
  return window.innerWidth >= breakpoints[breakpoint]
}

/**
 * 检查是否在断点范围内
 */
export function isBetweenBreakpoints(
  min: Breakpoint,
  max: Breakpoint
): boolean {
  const width = window.innerWidth
  return width >= breakpoints[min] && width < breakpoints[max]
}

/**
 * 监听断点变化
 */
export function onBreakpointChange(
  callback: (breakpoint: Breakpoint) => void
): () => void {
  let currentBreakpoint = getCurrentBreakpoint()

  const handleResize = () => {
    const newBreakpoint = getCurrentBreakpoint()
    if (newBreakpoint !== currentBreakpoint) {
      currentBreakpoint = newBreakpoint
      callback(newBreakpoint)
    }
  }

  window.addEventListener('resize', handleResize)
  return () => window.removeEventListener('resize', handleResize)
}

/**
 * 获取响应式值
 */
export function getResponsiveValue<T>(
  values: Partial<Record<Breakpoint, T>>,
  defaultValue: T
): T {
  const currentBreakpoint = getCurrentBreakpoint()
  const breakpointOrder: Breakpoint[] = ['xxl', 'xl', 'lg', 'md', 'sm', 'xs']

  for (const bp of breakpointOrder) {
    if (breakpoints[bp] <= breakpoints[currentBreakpoint] && values[bp] !== undefined) {
      return values[bp] as T
    }
  }

  return defaultValue
}

/**
 * 创建媒体查询
 */
export function createMediaQuery(breakpoint: Breakpoint): string {
  return `(min-width: ${breakpoints[breakpoint]}px)`
}

/**
 * 检查媒体查询是否匹配
 */
export function matchesMediaQuery(breakpoint: Breakpoint): boolean {
  return window.matchMedia(createMediaQuery(breakpoint)).matches
}

/**
 * 监听媒体查询变化
 */
export function onMediaQueryChange(
  breakpoint: Breakpoint,
  callback: (matches: boolean) => void
): () => void {
  const mediaQuery = window.matchMedia(createMediaQuery(breakpoint))

  const handleChange = (e: MediaQueryListEvent) => {
    callback(e.matches)
  }

  mediaQuery.addEventListener('change', handleChange)
  return () => mediaQuery.removeEventListener('change', handleChange)
}

/**
 * 获取设备类型
 */
export function getDeviceType(): 'mobile' | 'tablet' | 'desktop' {
  const width = window.innerWidth
  if (width < breakpoints.md) return 'mobile'
  if (width < breakpoints.lg) return 'tablet'
  return 'desktop'
}

/**
 * 检查是否是移动设备
 */
export function isMobile(): boolean {
  return getDeviceType() === 'mobile'
}

/**
 * 检查是否是平板设备
 */
export function isTablet(): boolean {
  return getDeviceType() === 'tablet'
}

/**
 * 检查是否是桌面设备
 */
export function isDesktop(): boolean {
  return getDeviceType() === 'desktop'
}

/**
 * 获取屏幕方向
 */
export function getScreenOrientation(): 'portrait' | 'landscape' {
  return window.innerHeight > window.innerWidth ? 'portrait' : 'landscape'
}

/**
 * 检查是否是竖屏
 */
export function isPortrait(): boolean {
  return getScreenOrientation() === 'portrait'
}

/**
 * 检查是否是横屏
 */
export function isLandscape(): boolean {
  return getScreenOrientation() === 'landscape'
}

/**
 * 监听屏幕方向变化
 */
export function onOrientationChange(
  callback: (orientation: 'portrait' | 'landscape') => void
): () => void {
  let currentOrientation = getScreenOrientation()

  const handleResize = () => {
    const newOrientation = getScreenOrientation()
    if (newOrientation !== currentOrientation) {
      currentOrientation = newOrientation
      callback(newOrientation)
    }
  }

  window.addEventListener('resize', handleResize)
  return () => window.removeEventListener('resize', handleResize)
}

/**
 * 获取像素密度
 */
export function getDevicePixelRatio(): number {
  return window.devicePixelRatio || 1
}

/**
 * 检查是否是高分辨率屏幕
 */
export function isHighDPI(): boolean {
  return getDevicePixelRatio() > 1
}

/**
 * 获取视口尺寸
 */
export function getViewportSize(): { width: number; height: number } {
  return {
    width: window.innerWidth,
    height: window.innerHeight,
  }
}

/**
 * 获取文档尺寸
 */
export function getDocumentSize(): { width: number; height: number } {
  return {
    width: document.documentElement.scrollWidth,
    height: document.documentElement.scrollHeight,
  }
}

/**
 * 获取滚动位置
 */
export function getScrollPosition(): { x: number; y: number } {
  return {
    x: window.scrollX,
    y: window.scrollY,
  }
}

/**
 * 监听滚动位置变化
 */
export function onScrollChange(
  callback: (position: { x: number; y: number }) => void
): () => void {
  let ticking = false

  const handleScroll = () => {
    if (!ticking) {
      window.requestAnimationFrame(() => {
        callback(getScrollPosition())
        ticking = false
      })
      ticking = true
    }
  }

  window.addEventListener('scroll', handleScroll, { passive: true })
  return () => window.removeEventListener('scroll', handleScroll)
}

/**
 * 平滑滚动到指定位置
 */
export function scrollTo(
  x: number,
  y: number,
  behavior: ScrollBehavior = 'smooth'
): void {
  window.scrollTo({
    left: x,
    top: y,
    behavior,
  })
}

/**
 * 平滑滚动到元素
 */
export function scrollToElement(
  element: HTMLElement,
  behavior: ScrollBehavior = 'smooth'
): void {
  element.scrollIntoView({
    behavior,
    block: 'start',
    inline: 'nearest',
  })
}

/**
 * 检查元素是否在视口中
 */
export function isElementInViewport(element: HTMLElement): boolean {
  const rect = element.getBoundingClientRect()
  return (
    rect.top >= 0 &&
    rect.left >= 0 &&
    rect.bottom <= (window.innerHeight || document.documentElement.clientHeight) &&
    rect.right <= (window.innerWidth || document.documentElement.clientWidth)
  )
}

/**
 * 监听元素是否在视口中
 */
export function onElementInViewport(
  element: HTMLElement,
  callback: (inViewport: boolean) => void
): () => void {
  const observer = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        callback(entry.isIntersecting)
      })
    },
    {
      threshold: 0.1,
    }
  )

  observer.observe(element)
  return () => observer.disconnect()
}

/**
 * 获取元素相对于视口的位置
 */
export function getElementViewportPosition(element: HTMLElement): {
  top: number
  right: number
  bottom: number
  left: number
} {
  const rect = element.getBoundingClientRect()
  return {
    top: rect.top,
    right: rect.right,
    bottom: rect.bottom,
    left: rect.left,
  }
}

/**
 * 获取元素相对于文档的位置
 */
export function getElementDocumentPosition(element: HTMLElement): {
  top: number
  left: number
} {
  const rect = element.getBoundingClientRect()
  const scrollPosition = getScrollPosition()
  return {
    top: rect.top + scrollPosition.y,
    left: rect.left + scrollPosition.x,
  }
}

/**
 * 获取元素尺寸
 */
export function getElementSize(element: HTMLElement): {
  width: number
  height: number
} {
  return {
    width: element.offsetWidth,
    height: element.offsetHeight,
  }
}

/**
 * 监听元素尺寸变化
 */
export function onElementResize(
  element: HTMLElement,
  callback: (size: { width: number; height: number }) => void
): () => void {
  const observer = new ResizeObserver((entries) => {
    entries.forEach((entry) => {
      callback({
        width: entry.contentRect.width,
        height: entry.contentRect.height,
      })
    })
  })

  observer.observe(element)
  return () => observer.disconnect()
}
