# Routeliner для RustGIS - события на маршрутах и пикетаж.
# Copyright (C) 2026 ООО «Информ++». Лицензия GPL 3.0 или новее.
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Сборка и пакет расширения. Сборка release, затем сценарий упаковки
# крейта rustgis-kit. В dist появляются папка routeliner для «Add Plugin
# Folder…» и пакет routeliner-<версия>.rgplugin для установки из файла.
#
#   powershell -File scripts\package.ps1
#   powershell -File scripts\package.ps1 -Kit D:\src\rustgis-kit

param(
    [string]$Kit = (Join-Path (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)) 'rustgis-kit')
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw 'cargo build --release failed' }
    & (Join-Path $Kit 'scripts\package.ps1') `
        -Manifest plugin.json `
        -Program target\release\routeliner-rustgis.exe `
        -Dist dist
} finally {
    Pop-Location
}
