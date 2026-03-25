@echo off
cd /d "E:\1\10\bingxi-rust\backend"
echo [INFO] Running cargo check...
cargo check --message-format=short 2>&1 | findstr /V "Blocking waiting for file lock" > build_check.log 2>&1
echo [INFO] Cargo check completed. Exit code: %errorlevel%
echo [INFO] Results:
type build_check.log | more
echo [INFO] Done.
