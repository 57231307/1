package com.bingxi.erp

import android.Manifest
import android.app.Activity
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothSocket
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.provider.Settings
import androidx.appcompat.app.AppCompatActivity
import androidx.core.app.ActivityCompat
import app.tauri.PermissionState
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSArray
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.nio.charset.Charset
import java.util.UUID
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors

/** 连接命令入参，address 为系统蓝牙设置里配对后打印机的 MAC 地址。 */
@InvokeArg
class BluetoothConnectArgs {
  var address: String? = null
}

/**
 * 打印命令入参。text 为已排好版的纯文本小票；cut 控制是否在结尾发切纸指令，
 * feedLines 控制切纸前走纸行数；encoding 缺省 GBK，热敏打印机中文需 GBK 或 GB18030。
 */
@InvokeArg
class BluetoothPrintTextArgs {
  var text: String = ""
  var cut: Boolean = true
  var feedLines: Int = 3
  var encoding: String = "GBK"
}

/**
 * 经典蓝牙 SPP 热敏打印桥。
 *
 * 调用方 安卓壳内的远端 ERP 页面经 window.__TAURI__.core.invoke 调 Rust 命令，
 * Rust 再经 run_mobile_plugin_async 派发到本插件的权限命令 listPrinters / connect / printText / disconnect。
 * 入参见 BluetoothConnectArgs 与 BluetoothPrintTextArgs。
 * 传给谁 扫描结果与连接状态回给 Rust 命令调用方；打印字节直接写 SPP socket。
 * 存什么 只在内存存当前 socket 与已连接地址，进程退出即失效；打印机地址由前端存本地偏好。
 * 存哪里 不落库、不写文件。
 */
@TauriPlugin(
  permissions = [
    Permission(strings = [Manifest.permission.BLUETOOTH_CONNECT], alias = "bluetoothConnect"),
    Permission(strings = [Manifest.permission.CAMERA], alias = "camera")
  ]
)
class BluetoothPrintPlugin(private val activity: Activity) : Plugin(activity) {
  private val sppUuid: UUID = UUID.fromString("00001101-0000-1000-8000-00805F9B34FB")
  private val io: ExecutorService = Executors.newSingleThreadExecutor()

  // 主线程读、IO 线程写，须 volatile 保证连接状态可见
  @Volatile private var socket: BluetoothSocket? = null
  @Volatile private var connectedAddress: String? = null

  @Command
  fun requestPermission(invoke: Invoke) {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.S) {
      resolvePermission(invoke, true)
      return
    }
    if (hasConnectPermission()) {
      resolvePermission(invoke, true)
      return
    }
    requestPermissionForAlias("bluetoothConnect", invoke, "onConnectPermissionResult")
  }

  @PermissionCallback
  fun onConnectPermissionResult(invoke: Invoke) {
    resolvePermission(invoke, getPermissionState("bluetoothConnect") == PermissionState.GRANTED)
  }

  /** 扫码前显式申请相机权限，避免 getUserMedia 与系统授权对话框并发启动而报启动失败。 */
  @Command
  fun requestCameraPermission(invoke: Invoke) {
    if (hasPermission(Manifest.permission.CAMERA)) {
      resolvePermission(invoke, true)
      return
    }
    requestPermissionForAlias("camera", invoke, "onCameraPermissionResult")
  }

  @PermissionCallback
  fun onCameraPermissionResult(invoke: Invoke) {
    resolvePermission(invoke, hasPermission(Manifest.permission.CAMERA))
  }

  @Command
  fun openSettings(invoke: Invoke) {
    activity.startActivity(Intent(Settings.ACTION_BLUETOOTH_SETTINGS))
    invoke.resolve()
  }

  @Command
  fun listPrinters(invoke: Invoke) {
    val granted = hasConnectPermission()
    val result = JSObject()
    result.put("sdkInt", Build.VERSION.SDK_INT)
    result.put("granted", granted)
    val adapter = bluetoothAdapter()
    result.put("supported", adapter != null)
    // Android 12 起 BluetoothAdapter.isEnabled 需要 BLUETOOTH_CONNECT，
    // 未授权时调用会抛 SecurityException，只能在已授权分支读取。
    result.put("enabled", if (granted && adapter != null) adapter.isEnabled else false)
    connectedAddress?.let { result.put("connectedAddress", it) }

    val printers = JSArray()
    if (adapter != null && hasConnectPermission()) {
      try {
        for (device in adapter.bondedDevices.orEmpty()) {
          val item = JSObject()
          item.put("name", device.name ?: "")
          item.put("address", device.address)
          printers.put(item)
        }
      } catch (e: SecurityException) {
        invoke.reject("读取已配对设备被系统拒绝：" + (e.message ?: "缺少蓝牙权限"))
        return
      }
    }
    result.put("printers", printers)
    invoke.resolve(result)
  }

  @Command
  fun connect(invoke: Invoke) {
    val address = invoke.parseArgs(BluetoothConnectArgs::class.java).address
    if (address.isNullOrBlank()) {
      invoke.reject("未选择打印机")
      return
    }
    if (!hasConnectPermission()) {
      invoke.reject("缺少蓝牙权限，请先授权")
      return
    }
    val adapter = bluetoothAdapter()
    if (adapter == null) {
      invoke.reject("本机不支持蓝牙")
      return
    }
    if (!adapter.isEnabled) {
      invoke.reject("蓝牙未开启，请先开启蓝牙")
      return
    }
    val device = try {
      adapter.getRemoteDevice(address)
    } catch (e: IllegalArgumentException) {
      invoke.reject("打印机地址非法：" + address)
      return
    }
    if (device.bondState != BluetoothDevice.BOND_BONDED) {
      invoke.reject("打印机尚未配对，请先在系统蓝牙设置中完成配对")
      return
    }

    io.execute {
      try {
        adapter.cancelDiscovery()
        closeSocket()
        val candidate = device.createRfcommSocketToServiceRecord(sppUuid)
        candidate.connect()
        socket = candidate
        connectedAddress = address
        val result = JSObject()
        result.put("address", address)
        result.put("name", device.name ?: "")
        invoke.resolve(result)
      } catch (e: Exception) {
        closeSocket()
        invoke.reject("连接打印机失败：" + (e.message ?: e.javaClass.simpleName))
      }
    }
  }

  @Command
  fun printText(invoke: Invoke) {
    val args = invoke.parseArgs(BluetoothPrintTextArgs::class.java)
    if (args.text.isEmpty()) {
      invoke.reject("打印内容为空")
      return
    }
    val charset = try {
      Charset.forName(args.encoding.ifBlank { "GBK" })
    } catch (e: Exception) {
      invoke.reject("不支持的字符集：" + args.encoding)
      return
    }
    val current = socket
    if (current == null || !current.isConnected) {
      invoke.reject("打印机未连接")
      return
    }

    io.execute {
      try {
        val out = current.outputStream
        out.write(byteArrayOf(0x1B, 0x40))
        val body = args.text.toByteArray(charset)
        out.write(body)
        repeat(args.feedLines.coerceIn(0, 10)) { out.write(0x0A) }
        if (args.cut) {
          out.write(byteArrayOf(0x1D, 0x56, 0x42, 0x00))
        }
        out.flush()
        val result = JSObject()
        result.put("bytes", body.size)
        invoke.resolve(result)
      } catch (e: Exception) {
        closeSocket()
        invoke.reject("打印失败：" + (e.message ?: e.javaClass.simpleName))
      }
    }
  }

  @Command
  fun disconnect(invoke: Invoke) {
    io.execute {
      closeSocket()
      invoke.resolve()
    }
  }

  override fun onDestroy(activity: AppCompatActivity) {
    io.execute { closeSocket() }
    io.shutdown()
  }

  private fun resolvePermission(invoke: Invoke, granted: Boolean) {
    val result = JSObject()
    result.put("granted", granted)
    result.put("sdkInt", Build.VERSION.SDK_INT)
    invoke.resolve(result)
  }

  private fun hasConnectPermission(): Boolean {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.S) {
      return true
    }
    return hasPermission(Manifest.permission.BLUETOOTH_CONNECT)
  }

  private fun hasPermission(permission: String): Boolean {
    return ActivityCompat.checkSelfPermission(
      activity,
      permission
    ) == PackageManager.PERMISSION_GRANTED
  }

  private fun bluetoothAdapter(): BluetoothAdapter? {
    return (activity.getSystemService(Context.BLUETOOTH_SERVICE) as? BluetoothManager)?.adapter
  }

  private fun closeSocket() {
    try {
      socket?.close()
    } catch (e: Exception) {
      // 关闭失败不影响后续重连，socket 已不可用
    }
    socket = null
    connectedAddress = null
  }
}
