#!/bin/bash
# PostToolUse hook: check for .unwrap() in edited Rust files
# Exit 0 with stdout message = warning added to Claude's context
# Exit 2 = block the action (we don't block, just warn)

INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

# Only check .rs files
if [[ "$FILE_PATH" != *.rs ]]; then
  exit 0
fi

# Skip test files and test modules
if [[ "$FILE_PATH" == *test* ]] || [[ "$FILE_PATH" == *tests/* ]]; then
  exit 0
fi

# Check for unwrap/expect in the file
UNWRAPS=$(grep -n '\.unwrap()' "$FILE_PATH" 2>/dev/null | grep -v '#\[cfg(test)\]' | grep -v '// safe:' | grep -v 'mod tests' | head -5)
EXPECTS=$(grep -n '\.expect(' "$FILE_PATH" 2>/dev/null | grep -v '#\[cfg(test)\]' | grep -v '// safe:' | grep -v 'mod tests' | head -5)

if [ -n "$UNWRAPS" ] || [ -n "$EXPECTS" ]; then
  echo "WARNING: .unwrap()/.expect() found in $FILE_PATH. Platform standards require proper error handling with ? operator. Use '// safe: <reason>' comment to suppress for compile-time-known values (e.g., hardcoded regex)."
  echo ""
  echo "$UNWRAPS"
  echo "$EXPECTS"
fi

exit 0
