#!/bin/bash

# Health Check Script for PCG-CC-MCP Dashboard
# Monitors all services and reports status

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_header() {
    echo ""
    echo -e "${BLUE}╔══════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║          PCG-CC-MCP Health Check                 ║${NC}"
    echo -e "${BLUE}╚══════════════════════════════════════════════════╝${NC}"
    echo ""
}

check_service() {
    local name=$1
    local url=$2
    local timeout=${3:-5}

    echo -n "  Checking $name... "

    if curl -sf -m $timeout "$url" > /dev/null 2>&1; then
        echo -e "${GREEN}✅ OK${NC}"
        return 0
    else
        echo -e "${RED}❌ FAILED${NC}"
        return 1
    fi
}

check_container() {
    local name=$1
    echo -n "  Container $name... "

    if docker-compose ps | grep -q "$name.*running"; then
        local health=$(docker inspect --format='{{.State.Health.Status}}' "pcg-$name" 2>/dev/null || echo "unknown")
        if [ "$health" = "healthy" ] || [ "$health" = "unknown" ]; then
            echo -e "${GREEN}✅ Running${NC}"
            return 0
        else
            echo -e "${YELLOW}⚠️  Running (unhealthy)${NC}"
            return 1
        fi
    else
        echo -e "${RED}❌ Not running${NC}"
        return 1
    fi
}

print_header

# Check if Docker Compose services are running
echo -e "${BLUE}Docker Containers:${NC}"
CONTAINERS_OK=true
check_container "cc-mcp" || CONTAINERS_OK=false
check_container "nginx" || CONTAINERS_OK=false
check_container "apn-bridge" || CONTAINERS_OK=false
check_container "cloudflared" || CONTAINERS_OK=false
check_container "db-backup" || CONTAINERS_OK=false

echo ""
echo -e "${BLUE}Service Health:${NC}"

# Check main services
SERVICES_OK=true

# App health endpoint
check_service "App API" "http://localhost:3001/api/health" 10 || SERVICES_OK=false

# Ollama
check_service "Ollama LLM" "http://localhost:11434/api/tags" 5 || SERVICES_OK=false

# Chatterbox (may take longer to start)
check_service "Chatterbox TTS" "http://localhost:8100" 5 || SERVICES_OK=false

# Nginx
check_service "Nginx Proxy" "http://localhost:8080" 5 || SERVICES_OK=false

# APN Bridge
check_service "APN Bridge" "http://localhost:8000/health" 5 || SERVICES_OK=false

echo ""
echo -e "${BLUE}Database:${NC}"

# Check database
if [ -f "./dev_assets/db.sqlite" ]; then
    DB_SIZE=$(du -h ./dev_assets/db.sqlite 2>/dev/null | cut -f1)
    echo -e "  Database file... ${GREEN}✅ OK${NC} (Size: $DB_SIZE)"

    # Check integrity
    if docker-compose exec -T app sqlite3 /app/dev_assets/db.sqlite "PRAGMA integrity_check;" 2>/dev/null | grep -q "ok"; then
        echo -e "  Database integrity... ${GREEN}✅ OK${NC}"
    else
        echo -e "  Database integrity... ${YELLOW}⚠️  Check needed${NC}"
    fi
else
    echo -e "  Database file... ${RED}❌ Not found${NC}"
fi

echo ""
echo -e "${BLUE}Backups:${NC}"

# Check backups
if [ -d "./backups" ]; then
    BACKUP_COUNT=$(find ./backups -name "backup_*.sqlite" -type f 2>/dev/null | wc -l)
    if [ $BACKUP_COUNT -gt 0 ]; then
        LATEST_BACKUP=$(ls -t ./backups/backup_*.sqlite 2>/dev/null | head -1)
        BACKUP_DATE=$(stat -c %y "$LATEST_BACKUP" 2>/dev/null | cut -d' ' -f1,2 | cut -d'.' -f1)
        echo -e "  Backup count... ${GREEN}✅ $BACKUP_COUNT backups${NC}"
        echo -e "  Latest backup... ${GREEN}$BACKUP_DATE${NC}"
    else
        echo -e "  Backup count... ${YELLOW}⚠️  No backups found${NC}"
    fi
else
    echo -e "  Backup directory... ${RED}❌ Not found${NC}"
fi

echo ""
echo -e "${BLUE}Resource Usage:${NC}"

# Get container stats
STATS=$(docker stats --no-stream --format "table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}" $(docker-compose ps -q 2>/dev/null) 2>/dev/null)
if [ -n "$STATS" ]; then
    echo "$STATS" | head -1
    echo "$STATS" | grep -v "CONTAINER"
else
    echo -e "  ${YELLOW}⚠️  No stats available${NC}"
fi

# Overall status
echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

if [ "$CONTAINERS_OK" = true ] && [ "$SERVICES_OK" = true ]; then
    echo -e "${GREEN}✅ Overall Status: HEALTHY${NC}"
    echo -e "${GREEN}All systems operational${NC}"
    exit 0
else
    echo -e "${YELLOW}⚠️  Overall Status: DEGRADED${NC}"
    echo -e "${YELLOW}Some services are not responding${NC}"
    echo ""
    echo "Troubleshooting:"
    echo "  1. Check logs: ./deploy.sh logs"
    echo "  2. Restart services: ./deploy.sh restart"
    echo "  3. Rebuild: ./deploy.sh clean"
    exit 1
fi
