/**
 * 安卓系统打印桥。
 *
 * 调用方 打印工具在安卓壳内把排好版的整页 HTML 或当前页面交给系统打印。
 * 入参 整页 HTML 与打印任务名，或只给任务名表示打印当前 WebView 显示的页面。
 * 传给谁 经 window.__TAURI__ 调壳内 Rust 命令，最终由 Kotlin 插件调系统 PrintManager。
 * 存什么 不存储。
 */
import { invokeBridge, isAndroidClient } from '@/utils/bluetooth-print';

/** 把整页 HTML 交给安卓系统打印对话框，返回是否已交给壳侧处理。 */
export async function printHtmlOnAndroid(html: string, jobName: string): Promise<boolean> {
  if (!isAndroidClient()) return false;
  await invokeBridge('system_print_html', { html, jobName });
  return true;
}

/** 把当前页面交给安卓系统打印对话框，返回是否已交给壳侧处理。 */
export async function printCurrentPageOnAndroid(jobName: string): Promise<boolean> {
  if (!isAndroidClient()) return false;
  await invokeBridge('system_print_page', { jobName });
  return true;
}
