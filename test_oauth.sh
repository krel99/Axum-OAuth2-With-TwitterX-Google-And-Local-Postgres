#!/bin/bash

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

BASE_URL="http://localhost:8000"
COOKIE_JAR="/tmp/oauth_test_cookies.txt"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}   OAuth2 Implementation Test Suite${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Function to test endpoint
test_endpoint() {
    local method=$1
    local endpoint=$2
    local expected_status=$3
    local description=$4

    echo -e "${YELLOW}Testing: ${description}${NC}"
    echo -e "  Endpoint: ${method} ${endpoint}"

    response=$(curl -s -o /dev/null -w "%{http_code}" \
        -X ${method} \
        -b ${COOKIE_JAR} \
        -c ${COOKIE_JAR} \
        "${BASE_URL}${endpoint}")

    if [ "$response" == "$expected_status" ]; then
        echo -e "  ${GREEN}✓ PASSED${NC} - Status: ${response}"
    else
        echo -e "  ${RED}✗ FAILED${NC} - Expected: ${expected_status}, Got: ${response}"
    fi
    echo ""
}

# Function to test JSON endpoint
test_json_endpoint() {
    local endpoint=$1
    local description=$2

    echo -e "${YELLOW}Testing: ${description}${NC}"
    echo -e "  Endpoint: GET ${endpoint}"

    response=$(curl -s "${BASE_URL}${endpoint}")

    # Check if response is valid JSON
    if echo "$response" | jq . >/dev/null 2>&1; then
        echo -e "  ${GREEN}✓ Valid JSON response${NC}"
        echo -e "  Response preview:"
        echo "$response" | jq '.' | head -10
    else
        echo -e "  ${RED}✗ Invalid JSON response${NC}"
    fi
    echo ""
}

# Clean up old cookie jar
rm -f ${COOKIE_JAR}

echo -e "${BLUE}1. Testing Public Endpoints${NC}"
echo -e "${BLUE}---------------------------${NC}"
echo ""

# Test homepage
test_endpoint "GET" "/" "200" "Homepage (Public)"

# Test login page
test_endpoint "GET" "/login" "200" "Login Page (Public)"

# Test health check
test_json_endpoint "/health" "Health Check Endpoint"

echo -e "${BLUE}2. Testing Protected Endpoints (Without Auth)${NC}"
echo -e "${BLUE}---------------------------------------------${NC}"
echo ""

# These should redirect to login (302) or return 401
test_endpoint "GET" "/protected" "303" "Protected Area (Should Redirect)"
test_endpoint "GET" "/protected/profile" "303" "User Profile (Should Redirect)"

echo -e "${BLUE}3. Testing Static Files${NC}"
echo -e "${BLUE}----------------------${NC}"
echo ""

# Test static file serving
test_endpoint "GET" "/static/index.html" "200" "Static HTML File"

echo -e "${BLUE}4. Testing Authentication Flow${NC}"
echo -e "${BLUE}------------------------------${NC}"
echo ""

echo -e "${YELLOW}Manual OAuth Flow Test:${NC}"
echo ""
echo -e "To test the complete OAuth flow:"
echo -e "1. Open your browser and navigate to: ${GREEN}${BASE_URL}${NC}"
echo -e "2. Click on 'Sign in with Google'"
echo -e "3. Complete the Google authentication"
echo -e "4. Verify you can access: ${GREEN}${BASE_URL}/protected${NC}"
echo -e "5. Check your profile at: ${GREEN}${BASE_URL}/protected/profile${NC}"
echo ""

echo -e "${BLUE}5. Server Status Check${NC}"
echo -e "${BLUE}---------------------${NC}"
echo ""

# Check if server is running
if curl -s -o /dev/null -w "%{http_code}" "${BASE_URL}/health" | grep -q "200"; then
    echo -e "${GREEN}✓ Server is running and healthy${NC}"

    # Get detailed health info
    health_response=$(curl -s "${BASE_URL}/health")
    if echo "$health_response" | jq . >/dev/null 2>&1; then
        db_status=$(echo "$health_response" | jq -r '.database')
        timestamp=$(echo "$health_response" | jq -r '.timestamp')
        echo -e "  Database: ${db_status}"
        echo -e "  Timestamp: ${timestamp}"
    fi
else
    echo -e "${RED}✗ Server is not responding${NC}"
    echo -e "  Make sure the server is running with: ${YELLOW}cargo run${NC}"
fi

echo ""
echo -e "${BLUE}6. Security Headers Check${NC}"
echo -e "${BLUE}------------------------${NC}"
echo ""

# Check for security headers
echo -e "${YELLOW}Checking security headers...${NC}"
headers=$(curl -sI "${BASE_URL}/")

# Check for important headers
check_header() {
    local header=$1
    if echo "$headers" | grep -qi "$header"; then
        echo -e "  ${GREEN}✓${NC} $header present"
    else
        echo -e "  ${YELLOW}⚠${NC} $header not found (consider adding)"
    fi
}

# These are optional but recommended
check_header "X-Content-Type-Options"
check_header "X-Frame-Options"
check_header "Content-Security-Policy"

echo ""
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}   Test Suite Complete${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Cleanup
rm -f ${COOKIE_JAR}

echo -e "${GREEN}Tips:${NC}"
echo -e "- For load testing, use: ${YELLOW}ab -n 1000 -c 10 ${BASE_URL}/${NC}"
echo -e "- For detailed debugging, use: ${YELLOW}RUST_LOG=debug cargo run${NC}"
echo -e "- To test with curl and cookies: ${YELLOW}curl -b cookies.txt -c cookies.txt ${BASE_URL}/protected${NC}"
