# Deploy to Fly.io with git SHA as Sentry release
$gitRev = try { git rev-parse --short=7 HEAD 2>$null } catch { "unknown" }
if (-not $gitRev) { $gitRev = "unknown" }
fly deploy --build-arg "GIT_REV_SHORT=$gitRev" @args
