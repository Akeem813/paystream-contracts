#!/usr/bin/env bash
set -euo pipefail

# Candidate issues suitable for beginners (good-first-issue)
ISSUES=(87 90 95 7 16 17 20 57 77 79 108)

echo "Tagging good-first-issue candidates in veracindarella/paystream-contracts..."

for issue in "${ISSUES[@]}"; do
    echo "Tagging issue #$issue..."
    gh issue edit "$issue" --repo veracindarella/paystream-contracts --add-label "good-first-issue" || true
done

echo "Done tagging candidate issues."
