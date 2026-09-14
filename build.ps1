[CmdletBinding()]
param(
    [ValidateSet(
        'Clean', 'Restore', 'Build', 'Test', 'Verify', 'Publish', 'PublishPortable',
        'RustRestore', 'RustCoreTest', 'RustCoreVerify', 'RustCheck', 'RustBuild',
        'RustTest', 'RustVerify', 'RustNativeSmoke', 'RustClean'
    )]
    [string] $Task = 'Verify',

    [ValidateSet('Debug', 'Release')]
    [string] $Configuration = 'Debug',

    [string] $RustTarget = '',

    [string] $RustToolchain = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$solutionPath = Join-Path $repositoryRoot 'FrigoTab.sln'
$applicationProjectPath = Join-Path $repositoryRoot 'FrigoTab\FrigoTab.csproj'
$acceptanceProjectPath = Join-Path $repositoryRoot 'FrigoTab.AcceptanceTests\FrigoTab.AcceptanceTests.csproj'
$rustWorkspacePath = Join-Path $repositoryRoot 'rust\Cargo.toml'
$leanPublishPath = Join-Path $repositoryRoot 'artifacts\publish\lean-win-x64'
$portablePublishPath = Join-Path $repositoryRoot 'artifacts\publish\portable-win-x64'

$script:dotnetPath = $null
$script:cargoPath = $null
$script:restoreComplete = $false
$script:buildComplete = $false

function Get-DotnetPath {
    if( $null -eq $script:dotnetPath ) {
        $dotnetCommand = Get-Command dotnet -CommandType Application -ErrorAction SilentlyContinue
        if( $null -eq $dotnetCommand ) {
            throw @"
The dotnet CLI was not found on PATH. Install the .NET 10 SDK (10.0.100 or later)
from https://dotnet.microsoft.com/download/dotnet/10.0, then run this command again.
No software was installed automatically.
"@
        }
        $script:dotnetPath = $dotnetCommand.Source
    }
    return $script:dotnetPath
}

function Get-CargoPath {
    if( $null -eq $script:cargoPath ) {
        $cargoCommand = Get-Command cargo -CommandType Application -ErrorAction SilentlyContinue
        if( $null -eq $cargoCommand ) {
            throw @"
The cargo CLI was not found on PATH. Install a stable Rust toolchain from
https://rustup.rs/, then run this command again. No software was installed automatically.
"@
        }
        $script:cargoPath = $cargoCommand.Source
    }
    return $script:cargoPath
}

function Invoke-Dotnet {
    param(
        [Parameter(Mandatory = $true)]
        [string[]] $Arguments
    )

    $dotnetPath = Get-DotnetPath
    Write-Host ("> dotnet " + ($Arguments -join ' '))
    & $dotnetPath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "dotnet command failed with exit code $LASTEXITCODE."
    }
}

function Invoke-Cargo {
    param(
        [Parameter(Mandatory = $true)]
        [string[]] $Arguments
    )

    if( -not (Test-Path -LiteralPath $rustWorkspacePath) ) {
        throw "The Rust workspace was not found at $rustWorkspacePath."
    }

    $cargoPath = Get-CargoPath
    $effectiveArguments = [System.Collections.Generic.List[string]]::new()
    if( -not [string]::IsNullOrWhiteSpace($RustToolchain) ) {
        $effectiveArguments.Add("+$RustToolchain")
    }
    $effectiveArguments.AddRange($Arguments)

    Write-Host ("> cargo " + ($effectiveArguments -join ' '))
    Push-Location (Split-Path -Parent $rustWorkspacePath)
    try {
        & $cargoPath @effectiveArguments
        if( $LASTEXITCODE -ne 0 ) {
            throw "cargo command failed with exit code $LASTEXITCODE."
        }
    }
    finally {
        Pop-Location
    }
}

function Add-RustBuildArguments {
    param(
        [Parameter(Mandatory = $true)]
        [System.Collections.Generic.List[string]] $Arguments
    )

    if( $Configuration -eq 'Release' ) {
        $Arguments.Add('--release')
    }
    if( -not [string]::IsNullOrWhiteSpace($RustTarget) ) {
        $Arguments.Add('--target')
        $Arguments.Add($RustTarget)
    }
}

function Invoke-RustRestore {
    Invoke-Cargo @('fetch', '--locked')
}

function Invoke-RustCoreTests {
    $arguments = [System.Collections.Generic.List[string]]::new()
    $arguments.Add('test')
    $arguments.Add('--package')
    $arguments.Add('frigo-tab-core')
    $arguments.Add('--all-targets')
    $arguments.Add('--locked')
    Add-RustBuildArguments $arguments
    Invoke-Cargo -Arguments ($arguments.ToArray())
}

function Invoke-RustCoreVerify {
    Invoke-Cargo @('fmt', '--all', '--check')
    $arguments = [System.Collections.Generic.List[string]]::new()
    $arguments.Add('clippy')
    $arguments.Add('--package')
    $arguments.Add('frigo-tab-core')
    $arguments.Add('--all-targets')
    $arguments.Add('--locked')
    Add-RustBuildArguments $arguments
    $arguments.Add('--')
    $arguments.Add('-D')
    $arguments.Add('warnings')
    Invoke-Cargo -Arguments ($arguments.ToArray())
    Invoke-RustCoreTests
}

function Invoke-RustCheck {
    $arguments = [System.Collections.Generic.List[string]]::new()
    $arguments.Add('check')
    $arguments.Add('--workspace')
    $arguments.Add('--all-targets')
    $arguments.Add('--locked')
    Add-RustBuildArguments $arguments
    Invoke-Cargo -Arguments ($arguments.ToArray())
}

function Invoke-RustBuild {
    $arguments = [System.Collections.Generic.List[string]]::new()
    $arguments.Add('build')
    $arguments.Add('--workspace')
    $arguments.Add('--locked')
    Add-RustBuildArguments $arguments
    Invoke-Cargo -Arguments ($arguments.ToArray())
}

function Invoke-RustTests {
    $arguments = [System.Collections.Generic.List[string]]::new()
    $arguments.Add('test')
    $arguments.Add('--workspace')
    $arguments.Add('--all-targets')
    $arguments.Add('--locked')
    Add-RustBuildArguments $arguments
    Invoke-Cargo -Arguments ($arguments.ToArray())
}

function Invoke-RustVerify {
    Invoke-Cargo @('fmt', '--all', '--check')
    Invoke-RustCheck

    $arguments = [System.Collections.Generic.List[string]]::new()
    $arguments.Add('clippy')
    $arguments.Add('--workspace')
    $arguments.Add('--all-targets')
    $arguments.Add('--locked')
    Add-RustBuildArguments $arguments
    $arguments.Add('--')
    $arguments.Add('-D')
    $arguments.Add('warnings')
    Invoke-Cargo -Arguments ($arguments.ToArray())

    Invoke-RustBuild
    Invoke-RustTests
}

function Invoke-RustNativeSmoke {
    $arguments = [System.Collections.Generic.List[string]]::new()
    $arguments.Add('run')
    $arguments.Add('--package')
    $arguments.Add('frigo-tab-spike')
    $arguments.Add('--locked')
    Add-RustBuildArguments $arguments
    $arguments.Add('--')
    $arguments.Add('--native-smoke')
    Invoke-Cargo -Arguments ($arguments.ToArray())
}

function Invoke-RustClean {
    Invoke-Cargo @('clean')
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
    'RustRestore' {
        Invoke-RustRestore
        break
    }
    'RustCoreTest' {
        Invoke-RustCoreTests
        break
    }
    'RustCoreVerify' {
        Invoke-RustCoreVerify
        break
    }
    'RustCheck' {
        Invoke-RustCheck
        break
    }
    'RustBuild' {
        Invoke-RustBuild
        break
    }
    'RustTest' {
        Invoke-RustTests
        break
    }
    'RustVerify' {
        Invoke-RustVerify
        break
    }
    'RustNativeSmoke' {
        Invoke-RustNativeSmoke
        break
    }
    'RustClean' {
        Invoke-RustClean
        break
    }
}
