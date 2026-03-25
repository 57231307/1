# 手动下载 MinGW-w64 说明

## ⚠️ 自动下载失败

由于网络连接问题，自动下载失败。请手动下载 winlibs MinGW。

## 📥 手动下载步骤

### 步骤 1：下载 winlibs MinGW

**下载地址**（选择一个）：

1. **GitHub 官方**（推荐）：
   - 访问：https://github.com/brechtsanders/winlibs_mingw/releases
   - 下载：`winlibs-x86_64-posix-seh-gcc-14.2.0-mingw-w64ucrt-12.0.0-r2.7z`
   - 大小：约 150 MB

2. **备用地址**：
   - 访问：https://winlibs.com/
   - 下载对应版本

### 步骤 2：解压文件

1. 使用 7-Zip 或 WinRAR 解压下载的 `.7z` 文件
2. 解压到：`C:\Program Files\mingw64`

### 步骤 3：配置环境变量

以**管理员身份**运行 PowerShell，执行：

```powershell
$mingwPath = "C:\Program Files\mingw64\bin"
[Environment]::SetEnvironmentVariable("Path", $env:Path + ";$mingwPath", [System.EnvironmentVariableTarget]::Machine)
```

### 步骤 4：验证安装

打开新的 PowerShell 窗口：

```powershell
gcc --version
```

应该显示 GCC 版本信息。

### 步骤 5：安装 Rust GNU 工具链

```powershell
rustup install stable-x86_64-pc-windows-gnu
rustup default stable-x86_64-pc-windows-gnu
```

### 步骤 6：配置项目

编辑项目根目录的 `.cargo/config.toml`：

```toml
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc.exe"
ar = "x86_64-w64-mingw32-ar.exe"
```

### 步骤 7：测试编译

```powershell
cd e:\1\10\bingxi-rust\backend
cargo clean
cargo check
```

---

## 📞 需要帮助？

下载完成后，告诉我"已下载"，我会帮你完成后续配置。
