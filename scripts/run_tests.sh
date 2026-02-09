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
COVERAGE=false
PARALLEL=false
for arg in "$@"; do
    case "$arg" in
        --unit-only) UNIT_ONLY=true ;;
        --e2e-only)  E2E_ONLY=true ;;
        --verbose)   VERBOSE=true ;;
        --coverage)  COVERAGE=true ;;
        --parallel)  PARALLEL=true ;;
        --help|-h)
            echo "Usage: $0 [--unit-only] [--e2e-only] [--verbose] [--coverage] [--parallel]"
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

# Per-suite summary rows.
SUITE_LABELS=()
SUITE_PASSED=()
SUITE_FAILED=()
SUITE_IGNORED=()
SUITE_DURATION=()
SUITE_EXIT=()

# Parallel-run bookkeeping (only used with --parallel).
BG_PIDS=()
BG_STARTS=()
BG_LOGS=()
BG_LABELS=()

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

        SUITE_LABELS+=("$label")
        SUITE_PASSED+=("$suite_passed")
        SUITE_FAILED+=("$suite_failed")
        SUITE_IGNORED+=("$suite_ignored")
        SUITE_DURATION+=("$suite_dur")
        SUITE_EXIT+=("$exit_code")
    else
        log "${YELLOW}WARNING${NC}: Could not parse test results"
        if [[ $exit_code -ne 0 ]]; then
            FAILED=$((FAILED + 1))
            TOTAL=$((TOTAL + 1))
            log "${RED}Exit code: $exit_code${NC}"
        fi

        SUITE_LABELS+=("$label")
        SUITE_PASSED+=("0")
        SUITE_FAILED+=("0")
        SUITE_IGNORED+=("0")
        SUITE_DURATION+=("$(( $(date +%s) - suite_start ))")
        SUITE_EXIT+=("$exit_code")
    fi

    return $exit_code
}

slugify() {
    echo "$1" | tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9]+/_/g' | sed -E 's/^_+|_+$//g'
}

parse_suite_log() {
    local label="$1"
    local log_path="$2"
    local suite_dur="$3"
    local exit_code="$4"

    section "$label"
    log "Command log: $log_path"
    log ""

    # Append suite log into the main log for one-stop review.
    cat "$log_path" >> "$LOG_FILE"

    local result_line suite_passed suite_failed suite_ignored
    result_line=$(grep "^test result:" "$log_path" | tail -1 || true)
    if [[ -n "$result_line" ]]; then
        suite_passed=$(echo "$result_line" | grep -oP '\d+ passed' | grep -oP '\d+' || true)
        suite_failed=$(echo "$result_line" | grep -oP '\d+ failed' | grep -oP '\d+' || true)
        suite_ignored=$(echo "$result_line" | grep -oP '\d+ ignored' | grep -oP '\d+' || true)

        suite_passed=${suite_passed:-0}
        suite_failed=${suite_failed:-0}
        suite_ignored=${suite_ignored:-0}

        TOTAL=$((TOTAL + suite_passed + suite_failed))
        PASSED=$((PASSED + suite_passed))
        FAILED=$((FAILED + suite_failed))
        SKIPPED=$((SKIPPED + suite_ignored))

        if [[ "$suite_failed" -gt 0 || "$exit_code" -ne 0 ]]; then
            log "${RED}${BOLD}FAIL${NC} — ${suite_passed} passed, ${suite_failed} failed (${suite_dur}s)"
            grep "^test .* FAILED$" "$log_path" | while read -r line; do
                log "  ${RED}✗${NC} $line"
            done
        else
            log "${GREEN}${BOLD}PASS${NC} — ${suite_passed} passed (${suite_dur}s)"
        fi

        SUITE_LABELS+=("$label")
        SUITE_PASSED+=("$suite_passed")
        SUITE_FAILED+=("$suite_failed")
        SUITE_IGNORED+=("$suite_ignored")
        SUITE_DURATION+=("$suite_dur")
        SUITE_EXIT+=("$exit_code")
    else
        log "${YELLOW}WARNING${NC}: Could not parse test results"
        if [[ "$exit_code" -ne 0 ]]; then
            FAILED=$((FAILED + 1))
            TOTAL=$((TOTAL + 1))
            log "${RED}Exit code: $exit_code${NC}"
        fi

        SUITE_LABELS+=("$label")
        SUITE_PASSED+=("0")
        SUITE_FAILED+=("0")
        SUITE_IGNORED+=("0")
        SUITE_DURATION+=("$suite_dur")
        SUITE_EXIT+=("$exit_code")
    fi
}

start_suite_bg() {
    local label="$1"
    shift
    local cmd=("$@")

    local slug suite_log
    slug=$(slugify "$label")
    suite_log="$LOG_DIR/test_suite_${TIMESTAMP}_${slug}.log"

    local suite_start
    suite_start=$(date +%s)

    (
        echo "=== $label ==="
        echo "Command: ${cmd[*]}"
        echo ""
        if $VERBOSE; then
            "${cmd[@]}" -- --nocapture
        else
            "${cmd[@]}"
        fi
    ) >"$suite_log" 2>&1 &

    BG_PIDS+=("$!")
    BG_STARTS+=("$suite_start")
    BG_LOGS+=("$suite_log")
    BG_LABELS+=("$label")
}

run_coverage() {
    section "Coverage"
    if cargo llvm-cov --version >/dev/null 2>&1; then
        log "Using: cargo llvm-cov"
        log ""
        cargo llvm-cov -p ntm-tracker-tui --lib --summary-only 2>&1 | tee -a "$LOG_FILE" || return 1
        cargo llvm-cov -p ntm-tracker-daemon --lib --summary-only 2>&1 | tee -a "$LOG_FILE" || return 1
        return 0
    fi

    if cargo tarpaulin --version >/dev/null 2>&1; then
        log "Using: cargo tarpaulin"
        log ""
        cargo tarpaulin -p ntm-tracker-tui --lib --quiet 2>&1 | tee -a "$LOG_FILE" || return 1
        cargo tarpaulin -p ntm-tracker-daemon --lib --quiet 2>&1 | tee -a "$LOG_FILE" || return 1
        return 0
    fi

    log "${RED}${BOLD}ERROR${NC}: --coverage requested but no coverage tool found."
    log "Install one of: cargo-llvm-cov or cargo-tarpaulin"
    return 2
}

print_suite_table() {
    log ""
    log "${BOLD}Per-suite summary${NC}"
    log ""
    log "  Suite                                   | Result | Passed | Failed | Ignored | Time"
    log "  ----------------------------------------+--------+--------+--------+---------+------"

    local i
    for i in "${!SUITE_LABELS[@]}"; do
        local label="${SUITE_LABELS[$i]}"
        local passed="${SUITE_PASSED[$i]}"
        local failed="${SUITE_FAILED[$i]}"
        local ignored="${SUITE_IGNORED[$i]}"
        local dur="${SUITE_DURATION[$i]}"
        local code="${SUITE_EXIT[$i]}"

        local result="PASS"
        if [[ "$failed" -gt 0 || "$code" -ne 0 ]]; then
            result="FAIL"
        fi

        printf "  %-40.40s | %-6s | %6s | %6s | %7s | %4ss\n" \
            "$label" "$result" "$passed" "$failed" "$ignored" "$dur" | tee -a "$LOG_FILE"
    done
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
    if $PARALLEL; then
        section "Unit Tests (parallel)"

        BG_PIDS=()
        BG_STARTS=()
        BG_LOGS=()
        BG_LABELS=()

        start_suite_bg "Unit Tests (lib)" cargo test -p ntm-tracker-tui --lib
        start_suite_bg "Daemon Unit Tests" cargo test -p ntm-tracker-daemon --lib
        start_suite_bg "Daemon Integration Tests" cargo test -p ntm-tracker-daemon --tests

        any_fail=0

        for i in "${!BG_PIDS[@]}"; do
            exit_code=0
            wait "${BG_PIDS[$i]}" || exit_code=$?
            end_time=$(date +%s)
            suite_dur=$((end_time - BG_STARTS[$i]))
            parse_suite_log "${BG_LABELS[$i]}" "${BG_LOGS[$i]}" "$suite_dur" "$exit_code"
            if [[ "$exit_code" -ne 0 ]]; then any_fail=1; fi
        done

        if [[ "$any_fail" -ne 0 ]]; then
            EXIT=1
        fi
    else
        run_test_suite "Unit Tests (lib)" cargo test -p ntm-tracker-tui --lib || EXIT=1

        # Daemon unit tests
        run_test_suite "Daemon Unit Tests" cargo test -p ntm-tracker-daemon --lib || EXIT=1
        run_test_suite "Daemon Integration Tests" cargo test -p ntm-tracker-daemon --tests || EXIT=1
    fi
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

# Coverage (optional)
if $COVERAGE; then
    run_coverage || EXIT=1
fi

# Summary
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

section "Summary"
log ""
print_suite_table
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
