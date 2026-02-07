#!/usr/bin/env bash
# run_tests.sh — Comprehensive test runner for NTM Tracker TUI
#
# Usage:
#   ./scripts/run_tests.sh              # Run all tests
#   ./scripts/run_tests.sh --unit-only  # Skip E2E tests
#   ./scripts/run_tests.sh --e2e-only   # Skip unit tests
#   ./scripts/run_tests.sh --verbose    # Extra detail

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

# Parse args
UNIT_ONLY=false
E2E_ONLY=false
VERBOSE=false
for arg in "$@"; do
    case "$arg" in
        --unit-only) UNIT_ONLY=true ;;
        --e2e-only)  E2E_ONLY=true ;;
        --verbose)   VERBOSE=true ;;
        --help|-h)
            echo "Usage: $0 [--unit-only] [--e2e-only] [--verbose]"
            exit 0
            ;;
    esac
done

# Setup
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
LOG_DIR="$PROJECT_DIR/logs"
mkdir -p "$LOG_DIR"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOG_FILE="$LOG_DIR/test_run_${TIMESTAMP}.log"

TOTAL=0
PASSED=0
FAILED=0
SKIPPED=0
START_TIME=$(date +%s)

log() {
    echo -e "$1" | tee -a "$LOG_FILE"
}

section() {
    log ""
    log "${BOLD}${BLUE}════════════════════════════════════════════════════${NC}"
    log "${BOLD}${BLUE}  $1${NC}"
    log "${BOLD}${BLUE}════════════════════════════════════════════════════${NC}"
}

run_test_suite() {
    local label="$1"
    shift
    local cmd=("$@")

    section "$label"
    log "Command: ${cmd[*]}"
    log ""

    local suite_start
    suite_start=$(date +%s)
    local output
    local exit_code=0

    if $VERBOSE; then
        output=$("${cmd[@]}" -- --nocapture 2>&1) || exit_code=$?
    else
        output=$("${cmd[@]}" 2>&1) || exit_code=$?
    fi

    echo "$output" >> "$LOG_FILE"

    # Parse test counts from output
    local result_line
    result_line=$(echo "$output" | grep "^test result:" | tail -1)

    if [[ -n "$result_line" ]]; then
        local suite_passed suite_failed suite_ignored
        suite_passed=$(echo "$result_line" | grep -oP '\d+ passed' | grep -oP '\d+')
        suite_failed=$(echo "$result_line" | grep -oP '\d+ failed' | grep -oP '\d+')
        suite_ignored=$(echo "$result_line" | grep -oP '\d+ ignored' | grep -oP '\d+')

        suite_passed=${suite_passed:-0}
        suite_failed=${suite_failed:-0}
        suite_ignored=${suite_ignored:-0}

        TOTAL=$((TOTAL + suite_passed + suite_failed))
        PASSED=$((PASSED + suite_passed))
        FAILED=$((FAILED + suite_failed))
        SKIPPED=$((SKIPPED + suite_ignored))

        local suite_end
        suite_end=$(date +%s)
        local suite_dur=$((suite_end - suite_start))

        if [[ "$suite_failed" -gt 0 ]]; then
            log "${RED}${BOLD}FAIL${NC} — ${suite_passed} passed, ${suite_failed} failed (${suite_dur}s)"
            # Show failing test names
            echo "$output" | grep "^test .* FAILED$" | while read -r line; do
                log "  ${RED}✗${NC} $line"
            done
        else
            log "${GREEN}${BOLD}PASS${NC} — ${suite_passed} passed (${suite_dur}s)"
        fi
    else
        log "${YELLOW}WARNING${NC}: Could not parse test results"
        if [[ $exit_code -ne 0 ]]; then
            FAILED=$((FAILED + 1))
            TOTAL=$((TOTAL + 1))
            log "${RED}Exit code: $exit_code${NC}"
        fi
    fi

    return $exit_code
}

# Header
log "${BOLD}NTM Tracker TUI — Test Runner${NC}"
log "Date: $(date -Iseconds)"
log "Log:  $LOG_FILE"

# Build first
section "Building"
if cargo build -p ntm-tracker-tui 2>&1 | tee -a "$LOG_FILE"; then
    log "${GREEN}Build OK${NC}"
else
    log "${RED}Build FAILED — aborting${NC}"
    exit 1
fi

EXIT=0

# Unit tests
if ! $E2E_ONLY; then
    run_test_suite "Unit Tests (lib)" cargo test -p ntm-tracker-tui --lib || EXIT=1

    # Daemon unit tests
    run_test_suite "Daemon Unit Tests" cargo test -p ntm-tracker-daemon --lib || EXIT=1
    run_test_suite "Daemon Integration Tests" cargo test -p ntm-tracker-daemon --tests || EXIT=1
else
    log ""
    log "${YELLOW}Skipping unit tests (--e2e-only)${NC}"
fi

# E2E tests
if ! $UNIT_ONLY; then
    run_test_suite "E2E: Daemon Integration" cargo test -p ntm-tracker-tui --test e2e_daemon || EXIT=1
    run_test_suite "E2E: Navigation" cargo test -p ntm-tracker-tui --test e2e_navigation || EXIT=1
else
    log ""
    log "${YELLOW}Skipping E2E tests (--unit-only)${NC}"
fi

# Summary
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

section "Summary"
log ""
log "  Total:   $TOTAL"
log "  ${GREEN}Passed:  $PASSED${NC}"
if [[ $FAILED -gt 0 ]]; then
    log "  ${RED}Failed:  $FAILED${NC}"
else
    log "  Failed:  $FAILED"
fi
if [[ $SKIPPED -gt 0 ]]; then
    log "  ${YELLOW}Skipped: $SKIPPED${NC}"
fi
log "  Duration: ${DURATION}s"
log "  Log: $LOG_FILE"
log ""

if [[ $FAILED -eq 0 ]]; then
    log "${GREEN}${BOLD}All tests passed!${NC}"
else
    log "${RED}${BOLD}$FAILED test(s) failed.${NC}"
fi

exit $EXIT
