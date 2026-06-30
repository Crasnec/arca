$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$TempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("arca-smoke-" + [System.Guid]::NewGuid())
New-Item -ItemType Directory -Path $TempRoot | Out-Null

try {
    $Bin = $env:ARCA_BIN
    if (-not $Bin) {
        cargo build --locked --bin arca --manifest-path (Join-Path $Root "Cargo.toml")
        $Bin = Join-Path $Root "target\debug\arca.exe"
    }

    $SevenZip = $env:SEVEN_ZIP_BIN
    if (-not $SevenZip) {
        $Command = Get-Command 7z -ErrorAction SilentlyContinue
        if ($Command) {
            $SevenZip = $Command.Source
        } elseif (Test-Path "C:\Program Files\7-Zip\7z.exe") {
            $SevenZip = "C:\Program Files\7-Zip\7z.exe"
        }
    }
    $RequireSevenZip = $env:ARCA_REQUIRE_7ZIP -eq "1"

    function Assert-SameFiles($Expected, $Actual) {
        $ExpectedFiles = Get-ChildItem -Path $Expected -Recurse -File | Sort-Object FullName
        foreach ($ExpectedFile in $ExpectedFiles) {
            $Relative = [System.IO.Path]::GetRelativePath($Expected, $ExpectedFile.FullName)
            $ActualFile = Join-Path $Actual $Relative
            if (-not (Test-Path $ActualFile)) {
                throw "missing extracted file: $Relative"
            }
            if ((Get-FileHash $ExpectedFile.FullName).Hash -ne (Get-FileHash $ActualFile).Hash) {
                throw "file content differs: $Relative"
            }
        }
        $ActualFiles = Get-ChildItem -Path $Actual -Recurse -File | Sort-Object FullName
        if ($ExpectedFiles.Count -ne $ActualFiles.Count) {
            throw "file count differs: $Expected has $($ExpectedFiles.Count), $Actual has $($ActualFiles.Count)"
        }
    }

    $Src = Join-Path $TempRoot "src"
    New-Item -ItemType Directory -Path (Join-Path $Src "sub") | Out-Null
    Set-Content -Path (Join-Path $Src "a.txt") -Value "hello arca"
    Set-Content -Path (Join-Path $Src "sub\b.txt") -Value "nested"

    & $Bin compress $Src -o (Join-Path $TempRoot "src.zip") --jobs 4 --quiet
    & $Bin test (Join-Path $TempRoot "src.zip") --jobs 4 --quiet
    & $Bin extract (Join-Path $TempRoot "src.zip") -o (Join-Path $TempRoot "zip-out") --jobs 4 --quiet
    Assert-SameFiles $Src (Join-Path $TempRoot "zip-out")

    & $Bin compress $Src -o (Join-Path $TempRoot "src.tar.gz") --quiet
    & $Bin test (Join-Path $TempRoot "src.tar.gz") --quiet
    & $Bin extract (Join-Path $TempRoot "src.tar.gz") -o (Join-Path $TempRoot "tgz-out") --quiet
    Assert-SameFiles $Src (Join-Path $TempRoot "tgz-out")

    foreach ($Suffix in @("tar.bz2", "tar.xz")) {
        & $Bin compress $Src -o (Join-Path $TempRoot "src.$Suffix") --quiet
        & $Bin test (Join-Path $TempRoot "src.$Suffix") --quiet
        & $Bin extract (Join-Path $TempRoot "src.$Suffix") -o (Join-Path $TempRoot "$Suffix-out") --quiet
        Assert-SameFiles $Src (Join-Path $TempRoot "$Suffix-out")
    }

    "secret`n" | & $Bin compress $Src -o (Join-Path $TempRoot "aes.zip") --jobs 4 --password-stdin --quiet
    "secret`n" | & $Bin test (Join-Path $TempRoot "aes.zip") --jobs 4 --password-stdin --quiet
    "wrong`n" | & $Bin test (Join-Path $TempRoot "aes.zip") --password-stdin --quiet 2>$null
    if ($LASTEXITCODE -eq 0) {
        throw "expected AES ZIP wrong password to be rejected"
    }
    "secret`n" | & $Bin extract (Join-Path $TempRoot "aes.zip") -o (Join-Path $TempRoot "aes-out") --jobs 4 --password-stdin --quiet
    Assert-SameFiles $Src (Join-Path $TempRoot "aes-out")

    "secret`n" | & $Bin compress $Src -o (Join-Path $TempRoot "zipcrypto.zip") --password-stdin --zipcrypto --quiet
    "secret`n" | & $Bin test (Join-Path $TempRoot "zipcrypto.zip") --password-stdin --quiet
    "wrong`n" | & $Bin test (Join-Path $TempRoot "zipcrypto.zip") --password-stdin --quiet 2>$null
    if ($LASTEXITCODE -eq 0) {
        throw "expected ZipCrypto ZIP wrong password to be rejected"
    }
    "secret`n" | & $Bin extract (Join-Path $TempRoot "zipcrypto.zip") -o (Join-Path $TempRoot "zipcrypto-out") --password-stdin --quiet
    Assert-SameFiles $Src (Join-Path $TempRoot "zipcrypto-out")

    & $Bin compress (Join-Path $Src "a.txt") -o (Join-Path $TempRoot "a.txt.gz") --quiet
    & $Bin test (Join-Path $TempRoot "a.txt.gz") --quiet
    & $Bin extract (Join-Path $TempRoot "a.txt.gz") -o (Join-Path $TempRoot "a.out") --quiet
    if ((Get-FileHash (Join-Path $Src "a.txt")).Hash -ne (Get-FileHash (Join-Path $TempRoot "a.out")).Hash) {
        throw "single-stream gzip roundtrip differs"
    }

    foreach ($Suffix in @("bz2", "xz")) {
        & $Bin compress (Join-Path $Src "a.txt") -o (Join-Path $TempRoot "a.txt.$Suffix") --quiet
        & $Bin test (Join-Path $TempRoot "a.txt.$Suffix") --quiet
        & $Bin extract (Join-Path $TempRoot "a.txt.$Suffix") -o (Join-Path $TempRoot "a.$Suffix.out") --quiet
        if ((Get-FileHash (Join-Path $Src "a.txt")).Hash -ne (Get-FileHash (Join-Path $TempRoot "a.$Suffix.out")).Hash) {
            throw "single-stream $Suffix roundtrip differs"
        }
    }

    Add-Type -AssemblyName System.IO.Compression
    $EvilZip = Join-Path $TempRoot "evil.zip"
    $Stream = [System.IO.File]::Open($EvilZip, [System.IO.FileMode]::CreateNew)
    try {
        $Archive = [System.IO.Compression.ZipArchive]::new($Stream, [System.IO.Compression.ZipArchiveMode]::Create)
        try {
            $Entry = $Archive.CreateEntry("../outside.txt")
            $Writer = [System.IO.StreamWriter]::new($Entry.Open())
            try {
                $Writer.WriteLine("bad")
            } finally {
                $Writer.Dispose()
            }
        } finally {
            $Archive.Dispose()
        }
    } finally {
        $Stream.Dispose()
    }
    & $Bin test $EvilZip --quiet 2>$null
    if ($LASTEXITCODE -eq 0) {
        throw "expected traversal ZIP test to be rejected"
    }
    $EvilOut = Join-Path $TempRoot "evil-out"
    & $Bin extract $EvilZip -o $EvilOut --quiet 2>$null
    if ($LASTEXITCODE -eq 0) {
        throw "expected traversal ZIP extraction to be rejected"
    }
    if (Test-Path $EvilOut) {
        throw "failed traversal ZIP extraction published output"
    }
    if (Test-Path (Join-Path $TempRoot "outside.txt")) {
        throw "failed traversal ZIP extraction wrote outside output"
    }

    if ($SevenZip) {
        Push-Location $Src
        & $SevenZip a -tzip (Join-Path $TempRoot "by-7z.zip") . | Out-Null
        Pop-Location
        & $Bin extract (Join-Path $TempRoot "by-7z.zip") -o (Join-Path $TempRoot "from-7z") --quiet
        Assert-SameFiles $Src (Join-Path $TempRoot "from-7z")

        & $Bin compress $Src -o (Join-Path $TempRoot "arca-for-7z.zip") --jobs 4 --quiet
        New-Item -ItemType Directory -Path (Join-Path $TempRoot "7z-out") | Out-Null
        & $SevenZip x (Join-Path $TempRoot "arca-for-7z.zip") "-o$(Join-Path $TempRoot "7z-out")" | Out-Null
        Assert-SameFiles $Src (Join-Path $TempRoot "7z-out")

        New-Item -ItemType Directory -Path (Join-Path $TempRoot "7z-aes-out") | Out-Null
        & $SevenZip x (Join-Path $TempRoot "aes.zip") "-o$(Join-Path $TempRoot "7z-aes-out")" "-psecret" "-y" | Out-Null
        Assert-SameFiles $Src (Join-Path $TempRoot "7z-aes-out")

        New-Item -ItemType Directory -Path (Join-Path $TempRoot "7z-zipcrypto-out") | Out-Null
        & $SevenZip x (Join-Path $TempRoot "zipcrypto.zip") "-o$(Join-Path $TempRoot "7z-zipcrypto-out")" "-psecret" "-y" | Out-Null
        Assert-SameFiles $Src (Join-Path $TempRoot "7z-zipcrypto-out")
    } else {
        if ($RequireSevenZip) {
            throw "7-Zip not found but ARCA_REQUIRE_7ZIP=1"
        }
        Write-Host "7-Zip not found; external compatibility smoke skipped"
    }

    Write-Host "arca smoke ok"
} finally {
    Remove-Item -Path $TempRoot -Recurse -Force -ErrorAction SilentlyContinue
}
