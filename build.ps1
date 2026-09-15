[CmdletBinding()]
param(
    [ValidateSet('Clean', 'Restore', 'Build', 'Test', 'Verify', 'Publish')]
    [string] $Task = 'Verify',

    [ValidateSet('Debug', 'Release')]
    [string] $Configuration = 'Debug'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$cargoCommand = Get-Command cargo -CommandType Application -ErrorAction SilentlyContinue
if ($null -eq $cargoCommand) {
    throw 'The cargo CLI was not found on PATH. Install Rust with rustup and try again.'
}
$cargoPath = $cargoCommand.Source

function Invoke-Cargo {
    param(
        [Parameter(Mandatory = $true)]
        [string[]] $Arguments
    )

    Write-Host ("> cargo " + ($Arguments -join ' '))
    Push-Location $repositoryRoot
    try {
        & $cargoPath @Arguments
        if ($LASTEXITCODE -ne 0) {
            throw "cargo command failed with exit code $LASTEXITCODE."
        }
    }
    finally {
        Pop-Location
    }
}

function Get-ProfileArguments {
    param(
        [Parameter(Mandatory = $true)]
        [string] $BuildConfiguration
    )

    if ($BuildConfiguration -eq 'Release') {
        return @('--release')
    }
    return @()
}

function Get-ExecutablePath {
    param(
        [Parameter(Mandatory = $true)]
        [string] $BuildConfiguration
    )

    $profile = if ($BuildConfiguration -eq 'Release') { 'release' } else { 'debug' }
    return Join-Path $repositoryRoot (Join-Path "target\$profile" 'FrigoTab.exe')
}

function Assert-RepositoryChildPath {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Path
    )

    $resolvedRoot = [System.IO.Path]::GetFullPath($repositoryRoot).TrimEnd('\') + '\'
    $resolvedTarget = [System.IO.Path]::GetFullPath($Path)
    if (-not $resolvedTarget.StartsWith($resolvedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to remove a path outside the repository: $resolvedTarget"
    }
}

function Invoke-Restore {
    Invoke-Cargo @('fetch', '--locked')
}

function Invoke-Build {
    param(
        [Parameter(Mandatory = $true)]
        [string] $BuildConfiguration
    )

    $arguments = @('build', '--workspace', '--locked') +
        (Get-ProfileArguments -BuildConfiguration $BuildConfiguration)
    Invoke-Cargo $arguments
}

function Invoke-Tests {
    param(
        [Parameter(Mandatory = $true)]
        [string] $BuildConfiguration
    )

    Invoke-Build -BuildConfiguration $BuildConfiguration
    $executablePath = Get-ExecutablePath -BuildConfiguration $BuildConfiguration
    if (-not (Test-Path -LiteralPath $executablePath -PathType Leaf)) {
        throw "Cargo did not produce the expected executable: $executablePath"
    }

    $previousExecutable = $env:FRIGOTAB_EXE
    try {
        $env:FRIGOTAB_EXE = $executablePath
        $arguments = @('test', '-p', 'frigotab-acceptance', '--locked') +
            (Get-ProfileArguments -BuildConfiguration $BuildConfiguration) +
            @('--', '--test-threads=1')
        Invoke-Cargo $arguments
    }
    finally {
        if ($null -eq $previousExecutable) {
            Remove-Item Env:FRIGOTAB_EXE -ErrorAction SilentlyContinue
        }
        else {
            $env:FRIGOTAB_EXE = $previousExecutable
        }
    }
}

function Invoke-Verify {
    param(
        [Parameter(Mandatory = $true)]
        [string] $BuildConfiguration
    )

    Invoke-Cargo @('fmt', '--all', '--', '--check')
    $arguments = @('check', '--workspace', '--all-targets', '--locked') +
        (Get-ProfileArguments -BuildConfiguration $BuildConfiguration)
    Invoke-Cargo $arguments
    Invoke-Tests -BuildConfiguration $BuildConfiguration
}

function Invoke-Clean {
    Invoke-Cargo @('clean')
    $artifactsPath = Join-Path $repositoryRoot 'artifacts'
    if (Test-Path -LiteralPath $artifactsPath) {
        Assert-RepositoryChildPath $artifactsPath
        Remove-Item -LiteralPath $artifactsPath -Recurse -Force
    }
}

function Invoke-Publish {
    Invoke-Verify -BuildConfiguration 'Release'

    $source = Get-ExecutablePath -BuildConfiguration 'Release'
    $publishDirectory = Join-Path $repositoryRoot 'artifacts\publish\win-x64'
    if (Test-Path -LiteralPath $publishDirectory) {
        Assert-RepositoryChildPath $publishDirectory
        Remove-Item -LiteralPath $publishDirectory -Recurse -Force
    }
    New-Item -ItemType Directory -Path $publishDirectory -Force | Out-Null
    Copy-Item -LiteralPath $source -Destination (Join-Path $publishDirectory 'FrigoTab.exe')
}

switch ($Task) {
    'Clean' {
        Invoke-Clean
        break
    }
    'Restore' {
        Invoke-Restore
        break
    }
    'Build' {
        Invoke-Build -BuildConfiguration $Configuration
        break
    }
    'Test' {
        Invoke-Tests -BuildConfiguration $Configuration
        break
    }
    'Verify' {
        Invoke-Verify -BuildConfiguration $Configuration
        break
    }
    'Publish' {
        Invoke-Publish
        break
    }
}
