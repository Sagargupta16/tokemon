#!/usr/bin/env bash
# MCP server integration test
# Sends JSON-RPC requests to `tokemon mcp` and validates responses.
set -euo pipefail

TOKEMON="${1:-./target/release/tokemon}"
PASS=0
FAIL=0

check() {
    local label="$1"
    local input="$2"
    local expected="$3"

    local output
    output=$(echo "$input" | "$TOKEMON" mcp 2>/dev/null)

    if echo "$output" | grep -qF "$expected"; then
        echo "  PASS: $label"
        PASS=$((PASS + 1))
    else
        echo "  FAIL: $label"
        echo "    expected to contain: $expected"
        echo "    got: $output"
        FAIL=$((FAIL + 1))
    fi
}

check_jq() {
    local label="$1"
    local input="$2"
    local jq_expr="$3"

    local output
    output=$(echo "$input" | "$TOKEMON" mcp 2>/dev/null)

    if echo "$output" | python3 -c "import sys,json; d=json.load(sys.stdin); assert $jq_expr, f'assertion failed: {d}'" 2>/dev/null; then
        echo "  PASS: $label"
        PASS=$((PASS + 1))
    else
        echo "  FAIL: $label"
        echo "    assertion: $jq_expr"
        echo "    got: $output"
        FAIL=$((FAIL + 1))
    fi
}

echo "=== MCP Server Tests ==="
echo ""

# --- Protocol tests ---
echo "Protocol:"

check "initialize returns protocol version" \
    '{"jsonrpc":"2.0","method":"initialize","id":1}' \
    '"protocolVersion"'

check "initialize returns server info" \
    '{"jsonrpc":"2.0","method":"initialize","id":1}' \
    '"name":"tokemon"'

check "unknown method returns error" \
    '{"jsonrpc":"2.0","method":"nonexistent","id":99}' \
    '"Method not found: nonexistent"'

check "parse error on invalid JSON" \
    'not json at all' \
    '"Parse error'

# --- tools/list ---
echo ""
echo "tools/list:"

check "lists get_usage_today" \
    '{"jsonrpc":"2.0","method":"tools/list","id":2}' \
    '"get_usage_today"'

check "lists get_usage_period" \
    '{"jsonrpc":"2.0","method":"tools/list","id":2}' \
    '"get_usage_period"'

check "lists get_budget_status" \
    '{"jsonrpc":"2.0","method":"tools/list","id":2}' \
    '"get_budget_status"'

check "lists get_session_cost" \
    '{"jsonrpc":"2.0","method":"tools/list","id":2}' \
    '"get_session_cost"'

check_jq "returns exactly 4 tools" \
    '{"jsonrpc":"2.0","method":"tools/list","id":2}' \
    'len(d["result"]["tools"]) == 4'

# --- tools/call: get_usage_today ---
echo ""
echo "tools/call:"

check_jq "get_usage_today returns date and cost" \
    '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_usage_today","arguments":{}},"id":3}' \
    '"date" in json.loads(d["result"]["content"][0]["text"]) and "cost_usd" in json.loads(d["result"]["content"][0]["text"])'

check_jq "get_usage_today returns total_tokens" \
    '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_usage_today","arguments":{}},"id":3}' \
    '"total_tokens" in json.loads(d["result"]["content"][0]["text"])'

# --- tools/call: get_usage_period ---

check_jq "get_usage_period returns summaries" \
    '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_usage_period","arguments":{"period":"daily"}},"id":4}' \
    '"summaries" in json.loads(d["result"]["content"][0]["text"])'

check_jq "get_usage_period with since filter works" \
    '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_usage_period","arguments":{"since":"2099-01-01"}},"id":5}' \
    'json.loads(d["result"]["content"][0]["text"])["total_tokens"] == 0'

# --- tools/call: get_budget_status ---

check_jq "get_budget_status returns valid JSON" \
    '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_budget_status","arguments":{}},"id":6}' \
    'isinstance(json.loads(d["result"]["content"][0]["text"]), dict)'

# --- tools/call: get_session_cost ---

check "get_session_cost with invalid session returns error" \
    '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_session_cost","arguments":{"session_id":"nonexistent-session-xyz"}},"id":7}' \
    '"isError":true'

check "unknown tool returns error" \
    '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"fake_tool","arguments":{}},"id":8}' \
    'Unknown tool: fake_tool'

# --- Multi-message test ---
echo ""
echo "Multi-message:"

MULTI_INPUT='{"jsonrpc":"2.0","method":"initialize","id":1}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","method":"tools/list","id":2}'

MULTI_OUTPUT=$(echo "$MULTI_INPUT" | "$TOKEMON" mcp 2>/dev/null)
LINE_COUNT=$(echo "$MULTI_OUTPUT" | wc -l | tr -d ' ')

if [ "$LINE_COUNT" -eq 2 ]; then
    echo "  PASS: multi-message returns 2 responses (initialize + tools/list, notification skipped)"
    PASS=$((PASS + 1))
else
    echo "  FAIL: multi-message expected 2 lines, got $LINE_COUNT"
    echo "    output: $MULTI_OUTPUT"
    FAIL=$((FAIL + 1))
fi

# --- Summary ---
echo ""
TOTAL=$((PASS + FAIL))
echo "Results: $PASS/$TOTAL passed"
if [ "$FAIL" -gt 0 ]; then
    echo "FAILED ($FAIL failures)"
    exit 1
else
    echo "ALL PASSED"
fi
