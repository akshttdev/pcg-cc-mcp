# Unregister Sovereign Stack from Windows File Explorer Navigation Pane
# Run as: powershell -ExecutionPolicy Bypass -File scripts/unregister_sovereign_stack.ps1

$CLSID = "{B3E49587-7262-0631-A680-2FDE729BF2B2}"

Write-Host "Unregistering Sovereign Stack from File Explorer sidebar..."

# Remove CLSID
$CLSIDPath = "HKCU:\Software\Classes\CLSID\$CLSID"
if (Test-Path $CLSIDPath) {
    Remove-Item -Path $CLSIDPath -Recurse -Force
    Write-Host "  Removed CLSID: $CLSIDPath"
} else {
    Write-Host "  CLSID not found (already removed)"
}

# Remove NameSpace
$NameSpacePath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\Desktop\NameSpace\$CLSID"
if (Test-Path $NameSpacePath) {
    Remove-Item -Path $NameSpacePath -Recurse -Force
    Write-Host "  Removed NameSpace: $NameSpacePath"
} else {
    Write-Host "  NameSpace not found (already removed)"
}

# Remove HideDesktopIcons entry
$HideDesktopPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\HideDesktopIcons\NewStartPanel"
if (Test-Path $HideDesktopPath) {
    $prop = Get-ItemProperty -Path $HideDesktopPath -Name $CLSID -ErrorAction SilentlyContinue
    if ($prop) {
        Remove-ItemProperty -Path $HideDesktopPath -Name $CLSID -Force
        Write-Host "  Removed HideDesktopIcons entry"
    }
}

Write-Host ""
Write-Host "Done! Sovereign Stack has been unregistered."
Write-Host "Restart File Explorer (or log out/in) for changes to take effect."
