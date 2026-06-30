$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$Archive = $env:ARCA_ARCHIVE
if (-not $Archive) {
    $Archive = "arca-windows-x86_64.zip"
}

if ([System.IO.Path]::IsPathRooted($Archive)) {
    $ArchivePath = $Archive
} else {
    $ArchivePath = Join-Path $Root $Archive
}

$Bin = $env:ARCA_BIN
if (-not $Bin) {
    $Bin = Join-Path $Root "target\release\arca.exe"
    cargo build --release --locked --bin arca --manifest-path (Join-Path $Root "Cargo.toml")
}

if (-not (Test-Path $Bin -PathType Leaf)) {
    throw "release binary not found: $Bin"
}
$Node = Get-Command node -ErrorAction SilentlyContinue
if (-not $Node) {
    throw "node is required to verify package binary version"
}
$ExpectedVersion = (& node (Join-Path $Root "scripts\version-check.mjs") --print-version).Trim()

$Stage = Join-Path $Root "dist\arca"
Remove-Item -Recurse -Force $Stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $Stage | Out-Null
Copy-Item $Bin (Join-Path $Stage "arca.exe")
Copy-Item (Join-Path $Root "docs\package-readme.md") (Join-Path $Stage "README.md")
Copy-Item (Join-Path $Root "LICENSE") $Stage
& node (Join-Path $Root "scripts\license-check.mjs") --write (Join-Path $Stage "THIRD_PARTY_LICENSES.md")

Remove-Item -Force $ArchivePath -ErrorAction SilentlyContinue
Remove-Item -Force "$ArchivePath.sha256" -ErrorAction SilentlyContinue
Compress-Archive -Path $Stage -DestinationPath $ArchivePath
$PackageCheck = Join-Path $Root "dist\package-check"
Remove-Item -Recurse -Force $PackageCheck -ErrorAction SilentlyContinue
Expand-Archive -Path $ArchivePath -DestinationPath $PackageCheck
$PackedRoot = Join-Path $PackageCheck "arca"
$RequiredFiles = @("arca.exe", "README.md", "LICENSE", "THIRD_PARTY_LICENSES.md")
foreach ($Name in $RequiredFiles) {
    $RequiredPath = Join-Path $PackedRoot $Name
    if (-not (Test-Path $RequiredPath)) {
        throw "packaged file missing: $Name"
    }
}
$ActualVersion = (& (Join-Path $PackedRoot "arca.exe") --version).Trim()
if ($ActualVersion -ne "arca $ExpectedVersion") {
    throw "packaged binary version mismatch: expected arca $ExpectedVersion, got $ActualVersion"
}

$Hash = (Get-FileHash $ArchivePath -Algorithm SHA256).Hash.ToLowerInvariant()
$ArchiveName = Split-Path $ArchivePath -Leaf
"$Hash  $ArchiveName" | Out-File -FilePath "$ArchivePath.sha256" -Encoding ascii

Write-Host $ArchivePath
