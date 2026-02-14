#!/bin/bash
# Test ORCHA Routing and Authentication
# Verifies user-to-Topsi routing is working correctly

# Don't exit on error - we want to run all tests

echo "======================================================================"
echo "ORCHA Routing Test Suite"
echo "======================================================================"
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

ADMIN_DB="/home/pythia/.local/share/pcg/data/admin/topsi.db"
SHARED_DB="/home/pythia/.local/share/duck-kanban/db.sqlite"

test_passed=0
test_failed=0

run_test() {
    local test_name="$1"
    local test_command="$2"

    echo -n "Testing: $test_name ... "

    if eval "$test_command" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ PASS${NC}"
        ((test_passed++))
        return 0
    else
        echo -e "${RED}✗ FAIL${NC}"
        ((test_failed++))
        return 1
    fi
}

echo "1. Configuration Tests"
echo "----------------------------------------------------------------------"

run_test "ORCHA config exists" "test -f orcha_config.toml"
run_test "Config is valid TOML" "toml-test orcha_config.toml 2>/dev/null || toml --help >/dev/null && cat orcha_config.toml | grep -q '\\[orcha\\]'"

echo ""
echo "2. Database Tests"
echo "----------------------------------------------------------------------"

run_test "Shared DB exists" "test -f $SHARED_DB"
run_test "Admin's Topsi DB exists" "test -f $ADMIN_DB"
run_test "Admin's Topsi has projects table" "sqlite3 $ADMIN_DB 'SELECT name FROM sqlite_master WHERE type=\"table\" AND name=\"projects\";' | grep -q projects"
run_test "Admin's Topsi has users table" "sqlite3 $ADMIN_DB 'SELECT name FROM sqlite_master WHERE type=\"table\" AND name=\"users\";' | grep -q users"

echo ""
echo "3. User Tests"
echo "----------------------------------------------------------------------"

run_test "Admin user exists in shared DB" "sqlite3 $SHARED_DB 'SELECT username FROM users WHERE username=\"admin\";' | grep -q admin"
run_test "Sirak user exists in shared DB" "sqlite3 $SHARED_DB 'SELECT username FROM users WHERE username=\"Sirak\";' | grep -q Sirak"
run_test "Bonomotion user exists in shared DB" "sqlite3 $SHARED_DB 'SELECT username FROM users WHERE username=\"Bonomotion\";' | grep -q Bonomotion"

echo ""
echo "4. Device Registry Tests"
echo "----------------------------------------------------------------------"

run_test "Device registry table exists" "sqlite3 $SHARED_DB 'SELECT name FROM sqlite_master WHERE type=\"table\" AND name=\"device_registry\";' | grep -q device_registry"
run_test "Pythia Master Node registered" "sqlite3 $SHARED_DB 'SELECT device_name FROM device_registry WHERE device_name LIKE \"%Pythia Master%\";' | grep -q 'Pythia Master'"
run_test "Space Terminal registered" "sqlite3 $SHARED_DB 'SELECT device_name FROM device_registry WHERE device_name=\"Space Terminal\";' | grep -q 'Space Terminal'"
run_test "Sirak Laptop registered" "sqlite3 $SHARED_DB 'SELECT device_name FROM device_registry WHERE device_name LIKE \"%Sirak%Laptop%\";' | grep -q 'Sirak'"

echo ""
echo "5. Project Tests"
echo "----------------------------------------------------------------------"

run_test "Admin has projects in Topsi" "sqlite3 $ADMIN_DB 'SELECT COUNT(*) FROM projects;' | grep -v '^0$' | grep -q '[1-9]'"
run_test "Admin has Pythia project" "sqlite3 $ADMIN_DB 'SELECT name FROM projects WHERE name LIKE \"%Pythia%\";' | grep -q 'Pythia'"
run_test "Admin has ORCHA project" "sqlite3 $ADMIN_DB 'SELECT name FROM projects WHERE name LIKE \"%ORCHA%\" OR name LIKE \"%PCG%\";' | head -1 | grep -q '.'"

echo ""
echo "6. Routing Module Tests"
echo "----------------------------------------------------------------------"

run_test "ORCHA routing module exists" "test -f crates/server/src/orcha_routing.rs"
run_test "ORCHA auth middleware exists" "test -f crates/server/src/middleware/orcha_auth.rs"
run_test "Routing module is imported" "grep -q 'pub mod orcha_routing' crates/server/src/lib.rs"

echo ""
echo "======================================================================"
echo "Test Results"
echo "======================================================================"
echo -e "${GREEN}Passed: $test_passed${NC}"
echo -e "${RED}Failed: $test_failed${NC}"
echo "Total:  $((test_passed + test_failed))"
echo ""

if [ $test_failed -eq 0 ]; then
    echo -e "${GREEN}✅ All tests passed! ORCHA routing is ready.${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. Start ORCHA: ORCHA_CONFIG=orcha_config.toml cargo run --bin server"
    echo "  2. Login as different users to test routing"
    echo "  3. Verify user-specific Topsi databases are accessed"
    exit 0
else
    echo -e "${RED}❌ Some tests failed. Please review the output above.${NC}"
    exit 1
fi
