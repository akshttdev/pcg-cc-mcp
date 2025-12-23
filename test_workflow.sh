#!/bin/bash

# Test workflow execution directly
# This simulates what should happen when Nora receives the request

DROPBOX_URL="https://www.dropbox.com/scl/fo/t1txylfh2i4r5valhbjfv/AMTf4O6HfnqyiqJKTIVKqwc/25Jan25%20Footage?rlkey=vxy38kwk9rzcocg5qbxh8r6ah&subfolder_nav_tracking=1&st=otfdfqyx&dl=0"

echo "Testing Editron workflow with Dropbox URL..."
echo "URL: $DROPBOX_URL"
echo ""
echo "Workflow should:"
echo "1. Create workflow instance"
echo "2. Monitor picks it up every 5 seconds"
echo "3. Executes Batch Intake stage → calls IngestMediaBatch"
echo "4. Downloads footage from Dropbox"
echo "5. Moves to next stage"
echo ""
echo "Monitoring logs for workflow execution..."
echo "Press Ctrl+C to stop"
echo ""

# Watch the logs for workflow activity
tail -f /tmp/dev-latest.log 2>/dev/null | strings | grep --line-buffered -E "WORKFLOW|intake|Ingest|TOOL.*Ingest"
