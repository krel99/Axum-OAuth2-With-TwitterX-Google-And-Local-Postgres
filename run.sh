#!/bin/bash

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}   OAuth2 Axum Application Launcher${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""

# Check if .env file exists
if [ ! -f .env ]; then
    echo -e "${RED}Error: .env file not found!${NC}"
    echo "Please create a .env file with the following variables:"
    echo "  GOOGLE_OAUTH_CLIENT_ID=your_client_id"
    echo "  GOOGLE_OAUTH_CLIENT_SECRET=your_client_secret"
    echo "  DATABASE_URL=postgres://postgres:password@localhost/oauth_db"
    exit 1
fi

# Load environment variables
export $(cat .env | grep -v '^#' | xargs)

# Check if Docker is installed and running
if command -v docker &> /dev/null && docker info &> /dev/null; then
    echo -e "${GREEN}Docker detected.${NC}"

    # Check if docker-compose exists
    if [ -f docker-compose.yml ]; then
        echo -e "${YELLOW}Starting PostgreSQL with Docker...${NC}"
        docker-compose up -d postgres

        # Wait for PostgreSQL to be ready
        echo -e "${YELLOW}Waiting for PostgreSQL to be ready...${NC}"
        for i in {1..30}; do
            if docker-compose exec -T postgres pg_isready -U postgres &> /dev/null; then
                echo -e "${GREEN}PostgreSQL is ready!${NC}"
                break
            fi
            echo -n "."
            sleep 1
        done
        echo ""
    fi
else
    echo -e "${YELLOW}Docker not found. Assuming PostgreSQL is already running.${NC}"

    # Check if PostgreSQL is accessible
    if command -v psql &> /dev/null; then
        if ! psql -U postgres -h localhost -p 5432 -d postgres -c '\q' 2>/dev/null; then
            echo -e "${RED}Warning: Could not connect to PostgreSQL.${NC}"
            echo "Please ensure PostgreSQL is running and accessible."
            echo "You can start it manually or use Docker:"
            echo "  docker-compose up -d postgres"
            read -p "Continue anyway? (y/N) " -n 1 -r
            echo
            if [[ ! $REPLY =~ ^[Yy]$ ]]; then
                exit 1
            fi
        else
            echo -e "${GREEN}PostgreSQL connection successful!${NC}"
        fi
    fi
fi

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Rust/Cargo not found!${NC}"
    echo "Please install Rust from https://rustup.rs/"
    exit 1
fi

# Build the application
echo -e "${YELLOW}Building the application...${NC}"
cargo build --release

if [ $? -ne 0 ]; then
    echo -e "${RED}Build failed!${NC}"
    exit 1
fi

echo -e "${GREEN}Build successful!${NC}"

# Run the application
echo -e "${YELLOW}Starting OAuth2 server on http://localhost:8000${NC}"
echo -e "${GREEN}========================================${NC}"
echo -e "Available endpoints:"
echo -e "  ${GREEN}http://localhost:8000${NC} - Homepage"
echo -e "  ${GREEN}http://localhost:8000/login${NC} - Login with Google"
echo -e "  ${GREEN}http://localhost:8000/protected${NC} - Protected area (requires auth)"
echo -e "${GREEN}========================================${NC}"
echo ""
echo -e "${YELLOW}Press Ctrl+C to stop the server${NC}"
echo ""

# Run the application
RUST_LOG=oauth_axum=debug,tower_http=debug cargo run --release
