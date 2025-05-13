#!/bin/bash

# Patterns to search for
SENSITIVE_RPC_PATTERNS=(
  'https:\/\/[a-z0-9-]+\.quiknode\.pro\/[a-f0-9]{32,}'
  'https:\/\/blastapi\.io\/dashboard\/project\/[a-f0-9\-]{36}'
  'https:\/\/[^ ]*\/[a-f0-9]{32,}\/?'
  'https:\/\/[^ ]*\?(.*key|token|auth)=.+'
  'https:\/\/[^ ]+:[^ @]+@[^ ]+'
)

FOUND=0

# Get list of staged files that are Added, Copied, or Modified
FILES=$(git diff --cached --name-only --diff-filter=ACM)

for file in $FILES; do
  # Skip if file is in .gitignore
  if git check-ignore "$file" > /dev/null; then
    continue
  fi

  # Get added lines in the file
  while IFS= read -r line; do
    # Skip empty lines or lines starting with '+'
    if [[ -z "$line" || "$line" =~ ^\+[^+] ]]; then
      # Extract the actual content (remove leading '+')
      line_content=$(echo "$line" | sed 's/^+//')
      # Skip bypassable lines
      if echo "$line_content" | grep -q 'no-check'; then
        continue
      fi

      for pattern in "${SENSITIVE_RPC_PATTERNS[@]}"; do
        if echo "$line_content" | grep -E -i "$pattern" > /dev/null; then
          echo "Sensitive pattern detected: $pattern"
          echo "File: $file"
          echo "Line: $line_content"
          echo "Add '# no-check' if intentional"
          echo "-------------------"
          FOUND=1
        fi
      done
    fi
  done < <(git diff --cached --unified=0 "$file" | grep '^+[^+]')
done

if [[ "$FOUND" -eq 1 ]]; then
  echo "🚫 Commit blocked due to potential secrets or private RPCs."
  exit 1
fi

exit 0