#!/bin/bash

SENSITIVE_RPC_PATTERNS=(
  'https:\/\/[a-z0-9-]+\.quiknode\.pro\/[a-f0-9]{32,}'
  'https:\/\/blastapi\.io\/dashboard\/project\/[a-f0-9\-]{36}'
  'https:\/\/[a-z0-9-]+\.infura\.io\/v3\/[a-f0-9]{32}'
  'https:\/\/\d+\.rpc\.thirdweb\.com\/[a-f0-9]{32}'
  'https:\/\/[^ ]*\/[a-f0-9]{32,}\/?'
  'https:\/\/[^ ]*\?(.*key|token|auth)=.+'
  'https:\/\/[^ ]+:[^ @]+@[^ ]+'
)

FOUND=0

# Get list of staged files that are Added, Copied, or Modified
FILES=$(git diff --cached --name-only --diff-filter=ACM)

for file in $FILES; do
  # --- EOL Check ---
  if [ -f "$file" ]; then
    last_byte=$(tail -c 1 "$file" | od -An -t u1 | tr -d ' ')
    if [ "$last_byte" != "10" ]; then
      echo "File '$file' does not end with a newline (EOL)."
      FOUND=1
    fi
  fi

  # -- Private RPC Check --
  while IFS= read -r line; do
    # Skip empty lines or lines starting with '+'
    if [[ -z "$line" || "$line" =~ ^\+[^+] ]]; then
      # Extract the actual content (remove leading '+')
      line_content=$(echo "$line" | sed 's/^+//')
      # Skip bypassable lines
      if echo "$line_content" | grep -q 'skip-check'; then
        continue
      fi

      for pattern in "${SENSITIVE_RPC_PATTERNS[@]}"; do
        if echo "$line_content" | grep -E -i "$pattern" > /dev/null; then
          echo "Sensitive pattern detected: $pattern"
          echo "File: $file"
          echo "Line: $line_content"
          echo "Add '# skip-check' if intentional"
          echo ""
          FOUND=1
        fi
      done
    fi
  done < <(git diff --cached --unified=0 "$file" | grep '^+[^+]')
done

if [[ "$FOUND" -eq 1 ]]; then
  echo "Commit blocked."
  exit 1
fi

exit 0
