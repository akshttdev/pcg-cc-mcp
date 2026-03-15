#!/bin/bash
# PostToolUse hook: check for `any` type in edited TypeScript files
# Exit 0 with stdout message = warning added to Claude's context

INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

# Only check .ts/.tsx files
if [[ "$FILE_PATH" != *.ts ]] && [[ "$FILE_PATH" != *.tsx ]]; then
  exit 0
fi

# Check for any types
ANY_TYPES=$(grep -n ': any\b\|as any\b\|<any>' "$FILE_PATH" 2>/dev/null | grep -v '// allow-any:' | head -5)

if [ -n "$ANY_TYPES" ]; then
  echo "WARNING: 'any' type found in $FILE_PATH. Platform standards require proper TypeScript types. Use '// allow-any: <reason>' comment to suppress for legitimate cases."
  echo ""
  echo "$ANY_TYPES"
fi

exit 0
