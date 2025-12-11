#!/bin/bash

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

BASE_URL="http://localhost:8000"
COOKIE_JAR="/tmp/oauth_logout_test_cookies.txt"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}   OAuth2 Logout Functionality Test${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Clean up old cookie jar
rm -f ${COOKIE_JAR}

# Function to test endpoint
test_endpoint() {
    local endpoint=$1
    local expected_status=$2
    local description=$3

    echo -e "${YELLOW}Testing: ${description}${NC}"

    response=$(curl -s -o /dev/null -w "%{http_code}" \
        -b ${COOKIE_JAR} \
        -c ${COOKIE_JAR} \
        -L \
        "${BASE_URL}${endpoint}")

    if [ "$response" == "$expected_status" ]; then
        echo -e "  ${GREEN}✓ PASSED${NC} - Status: ${response}"
        return 0
    else
        echo -e "  ${RED}✗ FAILED${NC} - Expected: ${expected_status}, Got: ${response}"
        return 1
    fi
}

echo -e "${BLUE}Step 1: Testing without authentication${NC}"
echo -e "${BLUE}---------------------------------------${NC}"
echo ""

test_endpoint "/protected" "200" "Protected area should redirect to login (and login returns 200)"
echo ""

echo -e "${BLUE}Step 2: Manual Login Required${NC}"
echo -e "${BLUE}-----------------------------${NC}"
echo ""
echo -e "${YELLOW}Please perform the following steps:${NC}"
echo -e "1. Open your browser and go to: ${GREEN}${BASE_URL}/login${NC}"
echo -e "2. Click 'Sign in with Google'"
echo -e "3. Complete the Google authentication"
echo -e "4. Wait until you're redirected to the protected area"
echo ""
read -p "Press Enter when you've successfully logged in..."
echo ""

echo -e "${BLUE}Step 3: Copy browser cookies${NC}"
echo -e "${BLUE}---------------------------${NC}"
echo ""
echo -e "${YELLOW}We need to extract your session cookie from the browser.${NC}"
echo -e "Open browser developer tools (F12) and go to:"
echo -e "  Application/Storage -> Cookies -> http://localhost:8000"
echo -e "  Find the cookie named 'sid' and copy its value"
echo ""
read -p "Paste the 'sid' cookie value here: " sid_cookie
echo ""

if [ -z "$sid_cookie" ]; then
    echo -e "${RED}No cookie value provided. Exiting test.${NC}"
    exit 1
fi

# Save the cookie to our cookie jar
echo "localhost:8000	FALSE	/	FALSE	0	sid	${sid_cookie}" > ${COOKIE_JAR}

echo -e "${BLUE}Step 4: Testing with authentication${NC}"
echo -e "${BLUE}-----------------------------------${NC}"
echo ""

test_endpoint "/protected" "200" "Protected area (should be accessible)"
if [ $? -ne 0 ]; then
    echo -e "${RED}Authentication test failed. Cookie might be invalid.${NC}"
    exit 1
fi

test_endpoint "/protected/profile" "200" "User profile (should be accessible)"
echo ""

echo -e "${BLUE}Step 5: Testing logout${NC}"
echo -e "${BLUE}---------------------${NC}"
echo ""

echo -e "${YELLOW}Logging out...${NC}"
curl -s -b ${COOKIE_JAR} -c ${COOKIE_JAR} -L "${BASE_URL}/api/auth/logout" > /dev/null
echo -e "${GREEN}✓ Logout request sent${NC}"
echo ""

echo -e "${BLUE}Step 6: Testing after logout${NC}"
echo -e "${BLUE}----------------------------${NC}"
echo ""

test_endpoint "/protected" "200" "Protected area after logout (should redirect to login page)"
echo ""

# Check if we're actually seeing the login page content
response_content=$(curl -s -b ${COOKIE_JAR} -c ${COOKIE_JAR} -L "${BASE_URL}/protected")
if echo "$response_content" | grep -q "Sign in with Google"; then
    echo -e "${GREEN}✓ CONFIRMED: User is logged out (login page shown)${NC}"
    logout_works=true
elif echo "$response_content" | grep -q "Protected Area"; then
    echo -e "${RED}✗ FAILED: User still has access to protected area!${NC}"
    echo -e "${RED}  The logout function is not working properly.${NC}"
    logout_works=false
else
    echo -e "${YELLOW}⚠ Cannot determine logout status from response${NC}"
    logout_works=unknown
fi

echo ""
echo -e "${BLUE}Step 7: Database Session Check${NC}"
echo -e "${BLUE}-----------------------------${NC}"
echo ""

echo -e "${YELLOW}Checking database for active sessions...${NC}"
echo -e "Run this command to check sessions in the database:"
echo -e "${GREEN}docker exec -it <your_postgres_container> psql -U postgres -d oauth_db -c \"SELECT * FROM sessions WHERE expires_at > NOW();\"${NC}"
echo ""

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}   Test Results Summary${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

if [ "$logout_works" = true ]; then
    echo -e "${GREEN}✓ Logout functionality is working correctly!${NC}"
    echo -e "  - Session cookie is invalidated"
    echo -e "  - Protected routes are no longer accessible"
    echo -e "  - User is redirected to login page"
elif [ "$logout_works" = false ]; then
    echo -e "${RED}✗ Logout functionality is NOT working!${NC}"
    echo -e "  - User can still access protected routes after logout"
    echo -e "  - Session may not be properly cleared from database"
    echo -e "  - Cookie may not be properly invalidated"
    echo ""
    echo -e "${YELLOW}Troubleshooting steps:${NC}"
    echo -e "1. Check that the logout function deletes the session from database"
    echo -e "2. Verify cookie is being expired/removed properly"
    echo -e "3. Check browser developer tools to see if 'sid' cookie is removed"
    echo -e "4. Verify the database session is deleted after logout"
else
    echo -e "${YELLOW}⚠ Test results inconclusive${NC}"
    echo -e "  Manual verification required"
fi

echo ""
echo -e "${BLUE}Additional Manual Tests:${NC}"
echo -e "1. After logout, try to directly access ${BASE_URL}/protected"
echo -e "2. Check browser cookies to ensure 'sid' is removed/expired"
echo -e "3. Try the back button after logout"
echo -e "4. Try to refresh the protected page after logout"

# Cleanup
rm -f ${COOKIE_JAR}
