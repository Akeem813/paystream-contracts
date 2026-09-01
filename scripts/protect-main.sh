#!/usr/bin/env bash
# Applies branch protection rules to `main` via the GitHub API.
# Requirements: gh CLI authenticated with repo admin rights.
# Usage: ./scripts/protect-main.sh [OWNER/REPO]
set -euo pipefail

REPO="${1:-veracindarella/paystream-contracts}"

echo "Applying branch protection to main on ${REPO}..."

gh api \
  --method PUT \
  -H "Accept: application/vnd.github+json" \
  "/repos/${REPO}/branches/main/protection" \
  --input - <<'EOF'
{
  "required_status_checks": {
    "strict": true,
    "contexts": ["build"]
  },
  "enforce_admins": false,
  "required_pull_request_reviews": {
    "required_approving_review_count": 1,
    "dismiss_stale_reviews": true
  },
  "restrictions": null,
  "allow_force_pushes": false,
  "allow_deletions": false
}
EOF

echo "Branch protection applied successfully."
