param([string]$Name = 'vesper-voice')
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Set-Location -LiteralPath $root
if ((Get-Content Cargo.toml -Raw) -notmatch '(?m)^name\s*=\s*"vesper"') { throw 'This is not the VESPER source directory' }
if ($Name -notmatch '^[A-Za-z0-9_.-]+$') { throw 'Invalid repository name' }
foreach ($command in @('git','gh')) {
    if (-not (Get-Command $command -ErrorAction SilentlyContinue)) {
        throw "Install $command first. GitHub CLI: https://cli.github.com/"
    }
}
gh auth status
if ($LASTEXITCODE -ne 0) { throw 'Sign in with gh auth login, then run this script again.' }
$accountText = gh api user
if ($LASTEXITCODE -ne 0) { throw 'Could not read the signed-in GitHub account' }
$account = ($accountText -join "`n") | ConvertFrom-Json
if (-not (Test-Path -LiteralPath '.git')) {
    git init -b main
    if ($LASTEXITCODE -ne 0) { throw 'Could not initialize Git' }
}
if (-not (git config user.name)) { git config --local user.name $account.login }
if (-not (git config user.email)) { git config --local user.email "$($account.id)+$($account.login)@users.noreply.github.com" }
$items = @('README.md','LICENSE','Cargo.toml','Cargo.lock','rust-toolchain.toml','build.rs','build.ps1','.gitignore','.gitattributes','.cargo','.github','src','assets','licenses','installer','scripts','docs','examples')
git add -- @items
if ($LASTEXITCODE -ne 0) { throw 'Could not stage project source' }
$tracked = @(git ls-files)
if ($tracked -match '(^|/)(\.env($|\.)|\.tools/|target/|dist/)|\.wav$') { throw 'Local-only data was found in the Git index' }
git diff --cached --quiet
if ($LASTEXITCODE -eq 1) {
    git commit -m 'Prepare VESPER voice studio release'
    if ($LASTEXITCODE -ne 0) { throw 'Could not commit the release' }
}
$remote = git remote get-url origin 2>$null
if ($LASTEXITCODE -ne 0) {
    # A fresh repository gets the conventional default branch. Existing
    # repositories keep their configured branch and remote.
    git branch -M main
    if ($LASTEXITCODE -ne 0) { throw 'Could not set the initial branch' }
    gh repo create "$($account.login)/$Name" --public --source $root --remote origin
    if ($LASTEXITCODE -ne 0) { throw 'Could not create the GitHub repository' }
} elseif ($remote -notmatch ('github\.com[:/]' + [regex]::Escape("$($account.login)/$Name") + '(\.git)?$')) {
    throw 'Origin points at a different repository. Refusing to push unrelated projects.'
}
git push --set-upstream origin HEAD
if ($LASTEXITCODE -ne 0) { throw 'Could not push the source' }
$version = [regex]::Match((Get-Content Cargo.toml -Raw), '(?m)^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"').Groups[1].Value
$tag = "v$version"
$existing = git rev-parse --verify "refs/tags/$tag" 2>$null
if ($LASTEXITCODE -ne 0) {
    git tag $tag
    if ($LASTEXITCODE -ne 0) { throw 'Could not create the version tag' }
} elseif ((git rev-list -n 1 $tag) -ne (git rev-parse HEAD)) {
    throw 'The version tag already points at a different commit. Update the release version.'
}
git push origin "refs/tags/$tag"
if ($LASTEXITCODE -ne 0) { throw 'Could not push the version tag' }
Write-Host 'Source published. GitHub Actions will build and verify a DRAFT release.'
gh repo view --json url --jq .url
