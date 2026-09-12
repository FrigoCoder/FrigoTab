[CmdletBinding()]
param(
    [ValidateSet('Clean', 'Restore', 'Build', 'Test', 'Verify', 'TestKnownIssues', 'Publish')]
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
$publishPath = Join-Path $repositoryRoot 'artifacts\publish\win-x64'

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
        '--filter', 'TestCategory=Acceptance&TestCategory!=KnownIssue'
    )
}

function Invoke-KnownIssueTests {
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
        '--filter', 'TestCategory=KnownIssue'
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

function Invoke-Publish {
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
        '--filter', 'TestCategory=Acceptance&TestCategory!=KnownIssue'
    )

    # Restore the application with its RID so the self-contained runtime pack
    # is present before publishing with --no-restore.
    Invoke-Dotnet @('restore', $applicationProjectPath, '--runtime', 'win-x64', '--nologo')

    if (Test-Path -LiteralPath $publishPath) {
        Assert-RepositoryChildPath $publishPath
        Remove-Item -LiteralPath $publishPath -Recurse -Force
    }
    New-Item -ItemType Directory -Path $publishPath -Force | Out-Null

    Invoke-Dotnet @(
        'publish', $applicationProjectPath,
        '--configuration', 'Release',
        '--runtime', 'win-x64',
        '--self-contained', 'true',
        '--no-restore',
        '--nologo',
        '-p:Platform=x64',
        '-p:PublishSingleFile=false',
        '--output', $publishPath
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
    'TestKnownIssues' {
        Invoke-Build
        Invoke-KnownIssueTests
        break
    }
    'Publish' {
        Invoke-Publish
        break
    }
}
