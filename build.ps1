[CmdletBinding()]
param(
    [ValidateSet('Clean', 'Restore', 'Build', 'Test', 'Verify', 'Publish', 'PublishPortable')]
    [string] $Task = 'Verify',

    [ValidateSet('Debug', 'Release')]
    [string] $Configuration = 'Debug'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$solutionPath = Join-Path $repositoryRoot 'FrigoTab.sln'
$applicationProjectPath = Join-Path $repositoryRoot 'FrigoTab\FrigoTab.csproj'
$acceptanceProjectPath = Join-Path $repositoryRoot 'FrigoTab.AcceptanceTests\FrigoTab.AcceptanceTests.csproj'
$leanPublishPath = Join-Path $repositoryRoot 'artifacts\publish\lean-win-x64'
$portablePublishPath = Join-Path $repositoryRoot 'artifacts\publish\portable-win-x64'

$dotnetCommand = Get-Command dotnet -CommandType Application -ErrorAction SilentlyContinue
if ($null -eq $dotnetCommand) {
    Write-Error @"
The dotnet CLI was not found on PATH. Install the .NET 10 SDK (10.0.100 or later)
from https://dotnet.microsoft.com/download/dotnet/10.0, then run this command again.
No software was installed automatically.
"@
    exit 1
}

$dotnetPath = $dotnetCommand.Source
$script:restoreComplete = $false
$script:buildComplete = $false

function Invoke-Dotnet {
    param(
        [Parameter(Mandatory = $true)]
        [string[]] $Arguments
    )

    Write-Host ("> dotnet " + ($Arguments -join ' '))
    & $dotnetPath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "dotnet command failed with exit code $LASTEXITCODE."
    }
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
    Invoke-Dotnet @('restore', $solutionPath, '--nologo')
    $script:restoreComplete = $true
}

function Ensure-Restore {
    if (-not $script:restoreComplete) {
        Invoke-Restore
    }
}

function Invoke-Build {
    Ensure-Restore
    Invoke-Dotnet @(
        'build', $solutionPath,
        '--configuration', $Configuration,
        '--no-restore',
        '--nologo',
        '-p:Platform=x64'
    )
    $script:buildComplete = $true
}

function Invoke-GreenTests {
    Ensure-Restore
    if (-not $script:buildComplete) {
        Invoke-Build
    }

    Invoke-Dotnet @(
        'test', $acceptanceProjectPath,
        '--configuration', $Configuration,
        '--no-build',
        '--no-restore',
        '--nologo',
        '-p:Platform=x64',
        '--filter', 'TestCategory=Acceptance'
    )
}

function Invoke-Clean {
    foreach ($cleanConfiguration in @('Debug', 'Release')) {
        Invoke-Dotnet @(
            'clean', $solutionPath,
            '--configuration', $cleanConfiguration,
            '--nologo',
            '--verbosity', 'minimal',
            '-p:Platform=x64'
        )
    }

    $artifactsPath = Join-Path $repositoryRoot 'artifacts'
    if (Test-Path -LiteralPath $artifactsPath) {
        Assert-RepositoryChildPath $artifactsPath
        Remove-Item -LiteralPath $artifactsPath -Recurse -Force
    }

    # `dotnet clean` only removes outputs for the requested configuration and
    # platform. Remove the known project output roots as well so this task has
    # Maven-clean semantics even after RID-specific/self-contained publishes.
    $generatedPaths = @(
        (Join-Path $repositoryRoot 'FrigoTab\bin'),
        (Join-Path $repositoryRoot 'FrigoTab\obj'),
        (Join-Path $repositoryRoot 'FrigoTab.Core\bin'),
        (Join-Path $repositoryRoot 'FrigoTab.Core\obj'),
        (Join-Path $repositoryRoot 'FrigoTab.AcceptanceTests\bin'),
        (Join-Path $repositoryRoot 'FrigoTab.AcceptanceTests\obj')
    )
    foreach ($generatedPath in $generatedPaths) {
        if (Test-Path -LiteralPath $generatedPath) {
            Assert-RepositoryChildPath $generatedPath
            Remove-Item -LiteralPath $generatedPath -Recurse -Force
        }
    }
}

function Invoke-ReleaseGate {
    # Publishing is release-gated: it first performs the same green build and
    # acceptance suite as Verify, but with Release binaries.
    Invoke-Dotnet @('restore', $solutionPath, '--nologo')
    Invoke-Dotnet @(
        'build', $solutionPath,
        '--configuration', 'Release',
        '--no-restore',
        '--nologo',
        '-p:Platform=x64'
    )
    Invoke-Dotnet @(
        'test', $acceptanceProjectPath,
        '--configuration', 'Release',
        '--no-build',
        '--no-restore',
        '--nologo',
        '-p:Platform=x64',
        '--filter', 'TestCategory=Acceptance'
    )

}

function Reset-PublishDirectory {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Path
    )

    if (Test-Path -LiteralPath $Path) {
        Assert-RepositoryChildPath $Path
        Remove-Item -LiteralPath $Path -Recurse -Force
    }
    New-Item -ItemType Directory -Path $Path -Force | Out-Null
}

function Invoke-Publish {
    Invoke-ReleaseGate

    # The default artifact is deliberately lean. It is a single application
    # file and relies on the .NET 10 Windows Desktop Runtime on the target PC.
    Invoke-Dotnet @('restore', $applicationProjectPath, '--runtime', 'win-x64', '--nologo')
    Reset-PublishDirectory $leanPublishPath

    Invoke-Dotnet @(
        'publish', $applicationProjectPath,
        '--configuration', 'Release',
        '--runtime', 'win-x64',
        '--self-contained', 'false',
        '--no-restore',
        '--nologo',
        '-p:Platform=x64',
        '-p:PublishSingleFile=true',
        '-p:DebugType=None',
        '-p:DebugSymbols=false',
        '--output', $leanPublishPath
    )
}

function Invoke-PublishPortable {
    Invoke-ReleaseGate

    # This fallback carries the entire Windows Desktop runtime for machines
    # without .NET. Compression cuts the old 117 MiB / 273-file output to one
    # roughly 47 MiB executable without unsupported WinForms trimming.
    Invoke-Dotnet @('restore', $applicationProjectPath, '--runtime', 'win-x64', '--nologo')
    Reset-PublishDirectory $portablePublishPath
    Invoke-Dotnet @(
        'publish', $applicationProjectPath,
        '--configuration', 'Release',
        '--runtime', 'win-x64',
        '--self-contained', 'true',
        '--no-restore',
        '--nologo',
        '-p:Platform=x64',
        '-p:PublishSingleFile=true',
        '-p:EnableCompressionInSingleFile=true',
        '-p:IncludeNativeLibrariesForSelfExtract=true',
        '-p:PublishReadyToRun=false',
        '-p:DebugType=None',
        '-p:DebugSymbols=false',
        '--output', $portablePublishPath
    )
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
        Invoke-Build
        break
    }
    'Test' {
        Invoke-Build
        Invoke-GreenTests
        break
    }
    'Verify' {
        Invoke-Build
        Invoke-GreenTests
        break
    }
    'Publish' {
        Invoke-Publish
        break
    }
    'PublishPortable' {
        Invoke-PublishPortable
        break
    }
}
