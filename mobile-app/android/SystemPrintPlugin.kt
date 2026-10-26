package com.bingxi.erp

import android.app.Activity
import android.content.Context
import android.os.Bundle
import android.os.CancellationSignal
import android.os.ParcelFileDescriptor
import android.print.PageRange
import android.print.PrintAttributes
import android.print.PrintDocumentAdapter
import android.print.PrintManager
import android.view.View
import android.view.ViewGroup
import android.webkit.WebView
import android.webkit.WebViewClient
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

/** 整页打印入参，html 为已排好版的完整页面，jobName 为系统打印任务名。 */
@InvokeArg
class SystemPrintHtmlArgs {
  var html: String = ""
  var jobName: String? = null
}

/** 当前页面打印入参，jobName 为系统打印任务名。 */
@InvokeArg
class SystemPrintJobArgs {
  var jobName: String? = null
}

/**
 * 安卓系统打印桥。
 *
 * 调用方 壳内远端 ERP 页面经 window.__TAURI__.core.invoke 调 Rust 命令，
 * Rust 再经 run_mobile_plugin_async 派发到本插件的 printHtml 与 printCurrentPage。
 * 入参 printHtml 收整页 HTML 与任务名，printCurrentPage 只收任务名。
 * 传给谁 交给系统 PrintManager 弹出打印对话框，打印机与份数由用户在对话框里选择。
 * 存什么 打印中的离屏 WebView 只留在内存，任务结束即销毁；不落库、不写文件。
 */
@TauriPlugin(permissions = [])
class SystemPrintPlugin(private val activity: Activity) : Plugin(activity) {
  // 离屏 WebView 必须在整个打印周期内强引用，否则打印框架取页时内容已被回收
  private var jobWebView: WebView? = null

  @Command
  fun printHtml(invoke: Invoke) {
    val args = invoke.parseArgs(SystemPrintHtmlArgs::class.java)
    if (args.html.isBlank()) {
      invoke.reject("打印内容为空")
      return
    }
    val jobName = args.jobName?.trim()?.takeIf { it.isNotEmpty() } ?: "打印"
    activity.runOnUiThread {
      try {
        startHtmlPrint(args.html, jobName, invoke)
      } catch (e: Exception) {
        releaseJobWebView()
        invoke.reject("唤起系统打印失败：" + (e.message ?: e.javaClass.simpleName))
      }
    }
  }

  @Command
  fun printCurrentPage(invoke: Invoke) {
    val jobName = invoke.parseArgs(SystemPrintJobArgs::class.java).jobName?.trim()?.takeIf { it.isNotEmpty() }
      ?: "打印"
    activity.runOnUiThread {
      val web = findMainWebView(activity.window.decorView)
      if (web == null) {
        invoke.reject("未找到可打印的页面")
        return@runOnUiThread
      }
      try {
        printManager().print(
          jobName,
          web.createPrintDocumentAdapter(jobName),
          PrintAttributes.Builder().build()
        )
        resolveJob(invoke, jobName)
      } catch (e: Exception) {
        invoke.reject("唤起系统打印失败：" + (e.message ?: e.javaClass.simpleName))
      }
    }
  }

  /** 用离屏 WebView 渲染整页 HTML 后交给系统打印，排版与桌面端窗内打印一致。 */
  private fun startHtmlPrint(html: String, jobName: String, invoke: Invoke) {
    releaseJobWebView()
    var settled = false
    // 结果只回一次，打印对话框弹出即算成功，失败分支也必须回一次，前端才不会一直等
    val settle: (String?) -> Unit = { error ->
      if (!settled) {
        settled = true
        if (error == null) {
          resolveJob(invoke, jobName)
        } else {
          releaseJobWebView()
          invoke.reject(error)
        }
      }
    }
    val metrics = activity.resources.displayMetrics
    val width = metrics.widthPixels.coerceAtLeast(1)
    val height = metrics.heightPixels.coerceAtLeast(1)
    val web = WebView(activity)
    web.settings.javaScriptEnabled = false
    web.settings.loadsImagesAutomatically = true
    web.isVerticalScrollBarEnabled = false
    web.layoutParams = ViewGroup.LayoutParams(width, height)
    // 页面要参与布局与绘制才会出内容，这里整体移出可视区而不是隐藏或透明
    web.translationX = -width * 2f
    jobWebView = web
    val root = activity.findViewById<ViewGroup>(android.R.id.content)
    root.addView(web)
    root.postDelayed({ settle("系统打印没有响应") }, 10000L)
    web.webViewClient = object : WebViewClient() {
      override fun onPageFinished(view: WebView, url: String) {
        // onPageFinished 早于首帧合成，稍等一拍再取打印适配器，避免打出空白页
        view.postDelayed({
          if (settled) return@postDelayed
          try {
            printManager().print(
              jobName,
              wrapAdapter(view.createPrintDocumentAdapter(jobName)),
              PrintAttributes.Builder().build()
            )
            settle(null)
          } catch (e: Exception) {
            settle("唤起系统打印失败：" + (e.message ?: e.javaClass.simpleName))
          }
        }, 300L)
      }
    }
    web.loadDataWithBaseURL(null, html, "text/html", "utf-8", null)
  }

  /** 包一层只为本插件补回收动作，打印框架结束任务时连离屏 WebView 一起销毁。 */
  private fun wrapAdapter(inner: PrintDocumentAdapter): PrintDocumentAdapter {
    return object : PrintDocumentAdapter() {
      override fun onLayout(
        oldAttributes: PrintAttributes?,
        newAttributes: PrintAttributes?,
        cancellationSignal: CancellationSignal?,
        callback: LayoutResultCallback?,
        extras: Bundle?
      ) {
        inner.onLayout(oldAttributes, newAttributes, cancellationSignal, callback, extras)
      }

      override fun onWrite(
        pages: MutableList<PageRange>?,
        destination: ParcelFileDescriptor?,
        cancellationSignal: CancellationSignal?,
        callback: WriteResultCallback?
      ) {
        inner.onWrite(pages, destination, cancellationSignal, callback)
      }

      override fun onFinish() {
        inner.onFinish()
        activity.runOnUiThread { releaseJobWebView() }
      }
    }
  }

  private fun releaseJobWebView() {
    val web = jobWebView ?: return
    jobWebView = null
    (web.parent as? ViewGroup)?.removeView(web)
    web.destroy()
  }

  private fun findMainWebView(view: View): WebView? {
    if (view is WebView) {
      return if (view === jobWebView) null else view
    }
    if (view is ViewGroup) {
      for (index in 0 until view.childCount) {
        val found = findMainWebView(view.getChildAt(index))
        if (found != null) return found
      }
    }
    return null
  }

  private fun printManager(): PrintManager {
    return activity.getSystemService(Context.PRINT_SERVICE) as PrintManager
  }

  private fun resolveJob(invoke: Invoke, jobName: String) {
    val result = JSObject()
    result.put("jobName", jobName)
    invoke.resolve(result)
  }

  override fun onDestroy(activity: androidx.appcompat.app.AppCompatActivity) {
    activity.runOnUiThread { releaseJobWebView() }
  }
}
