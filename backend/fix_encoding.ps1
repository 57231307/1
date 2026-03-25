# 修复 department_service.rs 文件的编码问题
$filePath = "src\services\department_service.rs"
$content = Get-Content $filePath -Raw -Encoding UTF8

# 替换所有中文注释和字符串为英文
$replacements = @{
    '/// 閮ㄩ棬鏍戣妭鐐癸紙鐢ㄤ簬杩斿洖鏍戝舰缁撴瀯锛？' = '/// Department tree node'
    '/// 鑾峰彇閮ㄩ棬鍒楄〃锛堟敮鎸佸垎椤靛拰杩囨护锛？' = '/// Get department list'
    '/// 搴旂敤杩囨护鏉′欢' = '// Apply filters'
    '閮ㄩ棬鍚嶇О' = 'Department name'
    '鐖堕儴闂？' = 'Parent department'
    '閮ㄩ棬' = 'Department'
    '宸插瓨鍦？' = 'already exists'
    '涓嶅瓨鍦？' = 'does not exist'
    '.offset(' = '.skip('
}

foreach ($key in $replacements.Keys) {
    $content = $content -replace [regex]::Escape($key), $replacements[$key]
}

# 修复注释和代码连在一起的问题
$content = $content -replace '(\s锛？)(\s*pub )', "`n`$2"

$content | Set-Content $filePath -Encoding UTF8
Write-Host "Fixed department_service.rs"
