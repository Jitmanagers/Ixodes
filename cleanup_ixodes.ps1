# Run this script as Administrator to ensure all components are removed.

$ErrorActionPreference = "SilentlyContinue"

Write-Host "--- Ixodes Advanced Cleanup Started ---" -ForegroundColor Cyan

Write-Host "[*] Terminating known processes..."
$ProcessNames = @("ms-identity", "crypto-svc", "cld-cache", "dnt-svc", "dev-chauffeur", "ixodes")
foreach ($name in $ProcessNames) {
    Stop-Process -Name $name -Force -ErrorAction SilentlyContinue
}

Write-Host "[*] Cleaning Registry Run keys..."
$RunKeys = @(
    "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run",
    "HKLM:\Software\Microsoft\Windows\CurrentVersion\Run",
    "HKCU:\Software\Microsoft\Windows\CurrentVersion\RunOnce",
    "HKLM:\Software\Microsoft\Windows\CurrentVersion\RunOnce",
    "HKCU:\Software\Microsoft\Windows\CurrentVersion\Policies\Explorer\Run",
    "HKLM:\Software\Microsoft\Windows\CurrentVersion\Policies\Explorer\Run"
)

$SuspiciousPatterns = @("ixodes", "ms-identity", "crypto-svc", "cld-cache", "dnt-svc", "dev-chauffeur", "IdentityCRL", "Crypto\\RSA", "DeviceChauffeur")

foreach ($keyPath in $RunKeys) {
    if (Test-Path $keyPath) {
        $props = Get-ItemProperty -Path $keyPath
        foreach ($propName in $props.PSObject.Properties.Name) {
            $val = $props.$propName.ToString()
            foreach ($pattern in $SuspiciousPatterns) {
                if ($val -like "*$pattern*") {
                    Write-Host "    Removing suspicious Run entry: $propName ($val) from $keyPath" -ForegroundColor Yellow
                    Remove-ItemProperty -Path $keyPath -Name $propName -Force
                    break
                }
            }
        }
    }
}

Write-Host "[*] Cleaning Environment hijacks..."
$EnvKey = "HKCU:\Environment"
$EnvProps = @("windir", "UserInitMprLogonScript")
foreach ($prop in $EnvProps) {
    $val = (Get-ItemProperty -Path $EnvKey -Name $prop -ErrorAction SilentlyContinue).$prop
    if ($val -and ($val -like "*ixodes*" -or $val -like "*cmd /c start*")) {
        Write-Host "    Removing Environment hijack: $prop ($val)" -ForegroundColor Yellow
        Remove-ItemProperty -Path $EnvKey -Name $prop -Force
    }
}

Write-Host "[*] Removing UAC Bypass hijacks..."
$UACClasses = @(
    "HKCU:\Software\Classes\ms-settings",
    "HKCU:\Software\Classes\Launcher.SystemSettings",
    "HKCU:\Software\Microsoft\Windows\CurrentVersion\App Paths\control.exe"
)

foreach ($path in $UACClasses) {
    if (Test-Path $path) {
        Write-Host "    Removing UAC bypass path: $path" -ForegroundColor Yellow
        Remove-Item -Path $path -Recurse -Force
    }
}

Write-Host "[*] Removing Scheduled Tasks..."
$TaskNames = @("WinMgmtEngineHealth")
foreach ($name in $TaskNames) {
    if (Get-ScheduledTask -TaskName $name -ErrorAction SilentlyContinue) {
        Unregister-ScheduledTask -TaskName $name -Confirm:$false -ErrorAction SilentlyContinue
        Write-Host "    Scheduled Task '$name' removed." -ForegroundColor Green
    }
}

Write-Host "[*] Cleaning COM Hijacks..."
$CLSIDs = @(
    "{42aedc87-2188-41fd-b9a3-0c966feabec1}",
    "{BCDE0395-E52F-467C-8E3D-C4579291692E}",
    "{FBEB8A05-BEEE-4442-8594-1592C541D06F}",
    "{00021401-0000-0000-C000-000000000046}",
    "{63354731-1688-4E7B-8228-05F7CE2A1145}"
)

foreach ($clsid in $CLSIDs) {
    $path = "HKCU:\Software\Classes\CLSID\$clsid"
    if (Test-Path $path) {
        Write-Host "    Removing COM Hijack: $clsid" -ForegroundColor Yellow
        Remove-Item -Path $path -Recurse -Force
    }
}

Write-Host "[*] Removing WMI Event Subscription..."
$WmiName = "WinMgmtEngineHealth"
Get-WmiObject -Namespace root\subscription -Class __EventFilter -Filter "Name='$WmiName'" | Remove-WmiObject
Get-WmiObject -Namespace root\subscription -Class CommandLineEventConsumer -Filter "Name='$WmiName'" | Remove-WmiObject
Get-WmiObject -Namespace root\subscription -Class __FilterToConsumerBinding -Filter "Filter=""__EventFilter.Name='$WmiName'""" | Remove-WmiObject
Get-WmiObject -Namespace root\subscription -Class __FilterToConsumerBinding -Filter "Consumer=""CommandLineEventConsumer.Name='$WmiName'""" | Remove-WmiObject

Write-Host "[*] Removing AppCertDlls entry..."
Remove-ItemProperty -Path "HKLM:\System\CurrentControlSet\Control\Session Manager\AppCertDlls" -Name "WinMgmtHealthSvc"

Write-Host "[*] Deleting persistence files..."
$TargetPaths = @(
    "$env:LOCALAPPDATA\Microsoft\Windows\IdentityCRL",
    "$env:LOCALAPPDATA\Microsoft\Crypto\RSA",
    "$env:LOCALAPPDATA\Microsoft\Windows\Caches",
    "$env:LOCALAPPDATA\Microsoft\Windows\DNT",
    "$env:LOCALAPPDATA\Microsoft\Windows\DeviceChauffeur"
)

foreach ($path in $TargetPaths) {
    if (Test-Path $path) {
        Write-Host "    Deleting: $path" -ForegroundColor Green
        attrib -s -h "$path\*" /S /D
        attrib -s -h "$path"
        Remove-Item -Path $path -Recurse -Force
    }
}

if (Test-Path "$env:USERPROFILE\Desktop\ixodes.exe") {
    Write-Host "[*] Found ixodes.exe on Desktop, deleting..." -ForegroundColor Yellow
    Remove-Item -Path "$env:USERPROFILE\Desktop\ixodes.exe" -Force
}

Write-Host "--- Cleanup Complete ---" -ForegroundColor Cyan
Write-Host "It is highly recommended to restart your computer now." -ForegroundColor Yellow
