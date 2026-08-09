/**
 * 性能监控工具 - batch-20 P3: FCP/LCP/TTI 监控
 * 使用 PerformanceObserver 监控核心 Web 指标
 */

/** 性能指标数据 */
export interface PerformanceMetric {
  name: string;
  value: number;
  rating: 'good' | 'needs-improvement' | 'poor';
}

/** 获取性能评级 */
function getRating(name: string, value: number): 'good' | 'needs-improvement' | 'poor' {
  const thresholds: Record<string, [number, number]> = {
    FCP: [1800, 3000],
    LCP: [2500, 4000],
    TTI: [3800, 7300],
    CLS: [0.1, 0.25],
    INP: [200, 500],
  };
  const [good, poor] = thresholds[name] || [0, 0];
  if (value <= good) return 'good';
  if (value <= poor) return 'needs-improvement';
  return 'poor';
}

/** 监控 FCP (First Contentful Paint) */
export function observeFCP(callback: (metric: PerformanceMetric) => void): void {
  try {
    const observer = new PerformanceObserver(list => {
      const entries = list.getEntriesByName('first-contentful-paint');
      if (entries.length > 0) {
        const value = entries[0].startTime;
        callback({
          name: 'FCP',
          value,
          rating: getRating('FCP', value),
        });
        observer.disconnect();
      }
    });
    observer.observe({ type: 'paint', buffered: true });
  } catch {
    // PerformanceObserver not supported
  }
}

/** 监控 LCP (Largest Contentful Paint) */
export function observeLCP(callback: (metric: PerformanceMetric) => void): void {
  try {
    const observer = new PerformanceObserver(list => {
      const entries = list.getEntries();
      if (entries.length > 0) {
        const lastEntry = entries[entries.length - 1];
        callback({
          name: 'LCP',
          value: lastEntry.startTime,
          rating: getRating('LCP', lastEntry.startTime),
        });
      }
    });
    observer.observe({ type: 'largest-contentful-paint', buffered: true });
  } catch {
    // PerformanceObserver not supported
  }
}

/** 监控 TTI (Time to Interactive) - 近似值 */
export function observeTTI(callback: (metric: PerformanceMetric) => void): void {
  try {
    // 使用 Navigation Timing API 近似计算 TTI
    const observer = new PerformanceObserver(list => {
      const entries = list.getEntries();
      if (entries.length > 0) {
        const navEntry = entries[0] as PerformanceNavigationTiming;
        const tti = navEntry.domInteractive - navEntry.startTime;
        callback({
          name: 'TTI',
          value: tti,
          rating: getRating('TTI', tti),
        });
        observer.disconnect();
      }
    });
    observer.observe({ type: 'navigation', buffered: true });
  } catch {
    // PerformanceObserver not supported
  }
}

/** 收集所有性能指标 */
export function collectMetrics(): Promise<PerformanceMetric[]> {
  return new Promise(resolve => {
    const metrics: PerformanceMetric[] = [];
    let resolved = false;

    const timeout = setTimeout(() => {
      if (!resolved) {
        resolved = true;
        resolve(metrics);
      }
    }, 10000);

    const checkDone = () => {
      if (metrics.length >= 3 && !resolved) {
        resolved = true;
        clearTimeout(timeout);
        resolve(metrics);
      }
    };

    observeFCP(metric => {
      metrics.push(metric);
      checkDone();
    });

    observeLCP(metric => {
      metrics.push(metric);
      checkDone();
    });

    observeTTI(metric => {
      metrics.push(metric);
      checkDone();
    });
  });
}

/** 上报性能指标到后端 */
export async function reportMetrics(metrics: PerformanceMetric[]): Promise<void> {
  try {
    const { request } = await import('@/api/request');
    await request.post('/api/v1/erp/performance/metrics', {
      metrics,
      url: window.location.href,
      timestamp: Date.now(),
    });
  } catch {
    // 静默失败：性能指标上报不应影响用户体验
  }
}
