# Register APN Cloud in Windows File Explorer Navigation Pane
# Run as: powershell -ExecutionPolicy Bypass -File scripts/register_sovereign_stack.ps1

$CLSID = "{B3E49587-7262-0631-A680-2FDE729BF2B2}"
$TargetFolder = "E:\topos\sovereign_stack"
$DisplayName = "APN Cloud"
$IconPath = "%SystemRoot%\system32\shell32.dll,275"

# Check if custom icon exists, use it if available
$CustomIcon = "E:\topos\sovereign_stack\.sovereign\apn-cloud.ico"
if (Test-Path $CustomIcon) {
    $IconPath = $CustomIcon
}

Write-Host "Registering '$DisplayName' in File Explorer sidebar..."
Write-Host "  CLSID:  $CLSID"
Write-Host "  Target: $TargetFolder"
Write-Host "  Icon:   $IconPath"

# Ensure target directory exists
if (-not (Test-Path $TargetFolder)) {
    New-Item -Path $TargetFolder -ItemType Directory -Force | Out-Null
    Write-Host "  Created target directory: $TargetFolder"
}

# ── CLSID registration ──────────────────────────────────────────────────────
$CLSIDPath = "HKCU:\Software\Classes\CLSID\$CLSID"

# Create CLSID key
New-Item -Path $CLSIDPath -Force | Out-Null
Set-ItemProperty -Path $CLSIDPath -Name "(Default)" -Value $DisplayName
New-ItemProperty -Path $CLSIDPath -Name "System.IsPinnedToNameSpaceTree" -PropertyType DWord -Value 1 -Force | Out-Null
New-ItemProperty -Path $CLSIDPath -Name "SortOrderIndex" -PropertyType DWord -Value 0x42 -Force | Out-Null

# DefaultIcon
$IconKeyPath = "$CLSIDPath\DefaultIcon"
New-Item -Path $IconKeyPath -Force | Out-Null
Set-ItemProperty -Path $IconKeyPath -Name "(Default)" -Value $IconPath

# InProcServer32 (empty — tells Explorer this is a shell namespace extension)
$InProcPath = "$CLSIDPath\InProcServer32"
New-Item -Path $InProcPath -Force | Out-Null
Set-ItemProperty -Path $InProcPath -Name "(Default)" -Value ""

# Instance
$InstancePath = "$CLSIDPath\Instance"
New-Item -Path $InstancePath -Force | Out-Null
Set-ItemProperty -Path $InstancePath -Name "CLSID" -Value "{0E5AAE11-A475-4c5b-AB00-C66DE400274E}"

# Instance\InitPropertyBag
$PropBagPath = "$InstancePath\InitPropertyBag"
New-Item -Path $PropBagPath -Force | Out-Null
New-ItemProperty -Path $PropBagPath -Name "Attributes" -PropertyType DWord -Value 0x11 -Force | Out-Null
Set-ItemProperty -Path $PropBagPath -Name "TargetFolderPath" -Value $TargetFolder

# ShellFolder
$ShellFolderPath = "$CLSIDPath\ShellFolder"
New-Item -Path $ShellFolderPath -Force | Out-Null
New-ItemProperty -Path $ShellFolderPath -Name "FolderValueFlags" -PropertyType DWord -Value 0x28 -Force | Out-Null
New-ItemProperty -Path $ShellFolderPath -Name "Attributes" -PropertyType DWord -Value 0xF080004D -Force | Out-Null

# ── NameSpace registration (makes it appear in sidebar) ─────────────────────
$NameSpacePath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Desktop\NameSpace\$CLSID"
New-Item -Path $NameSpacePath -Force | Out-Null
Set-ItemProperty -Path $NameSpacePath -Name "(Default)" -Value $DisplayName

# ── Hide from Desktop ───────────────────────────────────────────────────────
$HideDesktopPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\HideDesktopIcons\NewStartPanel"
if (-not (Test-Path $HideDesktopPath)) {
    New-Item -Path $HideDesktopPath -Force | Out-Null
}
New-ItemProperty -Path $HideDesktopPath -Name $CLSID -PropertyType DWord -Value 1 -Force | Out-Null

Write-Host ""
Write-Host "Done! '$DisplayName' has been registered."
Write-Host "Restart File Explorer (or log out/in) to see it in the sidebar."
Write-Host ""
Write-Host "To unregister, run: scripts/unregister_sovereign_stack.ps1"
Write-Host ""
Write-Host "Note: If you had a previous 'Sovereign Stack' entry, run unregister first."
