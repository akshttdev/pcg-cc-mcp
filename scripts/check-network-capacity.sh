#!/bin/bash
# Check APN Network Capacity and Utilization

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color
BOLD='\033[1m'

echo ""
echo "╔════════════════════════════════════════════════════════════════════╗"
echo "║                                                                    ║"
echo "║         🌐 ALPHA PROTOCOL NETWORK - CAPACITY CHECK                ║"
echo "║                                                                    ║"
echo "╚════════════════════════════════════════════════════════════════════╝"
echo ""

# Function to get percentage bar
get_bar() {
    local percentage=$1
    local width=20
    local filled=$(printf "%.0f" $(echo "$percentage * $width / 100" | bc -l))
    local empty=$((width - filled))

    printf "["
    for ((i=0; i<filled; i++)); do printf "▓"; done
    for ((i=0; i<empty; i++)); do printf "░"; done
    printf "]"
}

# Function to get color based on utilization
get_util_color() {
    local util=$1
    if (( $(echo "$util < 50" | bc -l) )); then
        echo -e "${GREEN}"
    elif (( $(echo "$util < 80" | bc -l) )); then
        echo -e "${YELLOW}"
    else
        echo -e "${RED}"
    fi
}

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${BOLD}📍 LOCAL NODE: Pythia Master Node (apn_c83344a2)${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Get system info
CPU_CORES=$(nproc)
CPU_MODEL=$(lscpu | grep "Model name" | cut -d ':' -f 2 | xargs)
CPU_USAGE=$(top -bn1 | grep "Cpu(s)" | awk '{print $2}' | cut -d '%' -f 1)
CPU_USAGE_INT=$(printf "%.0f" "$CPU_USAGE")

MEM_TOTAL=$(free -g | awk '/^Mem:/ {print $2}')
MEM_USED=$(free -g | awk '/^Mem:/ {print $3}')
MEM_USAGE=$(echo "scale=1; $MEM_USED * 100 / $MEM_TOTAL" | bc)
MEM_USAGE_INT=$(printf "%.0f" "$MEM_USAGE")

DISK_TOTAL=$(df -h / | awk 'NR==2 {print $2}')
DISK_USED=$(df -h / | awk 'NR==2 {print $3}')
DISK_USAGE=$(df / | awk 'NR==2 {print $5}' | sed 's/%//')

# Check GPU
if command -v nvidia-smi &> /dev/null; then
    GPU_NAME=$(nvidia-smi --query-gpu=name --format=csv,noheader)
    GPU_MEM_TOTAL=$(nvidia-smi --query-gpu=memory.total --format=csv,noheader,nounits)
    GPU_MEM_USED=$(nvidia-smi --query-gpu=memory.used --format=csv,noheader,nounits)
    GPU_UTIL=$(nvidia-smi --query-gpu=utilization.gpu --format=csv,noheader,nounits)
    GPU_MEM_USAGE=$(echo "scale=1; $GPU_MEM_USED * 100 / $GPU_MEM_TOTAL" | bc)
    GPU_MEM_USAGE_INT=$(printf "%.0f" "$GPU_MEM_USAGE")
    HAS_GPU=true
else
    HAS_GPU=false
fi

# Display local resources
echo -e "${CYAN}💻 Compute Resources:${NC}"
echo ""
echo -e "  ${BOLD}CPU:${NC} $CPU_MODEL"
echo -e "       Cores: $CPU_CORES"
echo -e "       Usage: $(get_util_color $CPU_USAGE_INT)$CPU_USAGE%${NC} $(get_bar $CPU_USAGE_INT)"
echo ""

echo -e "  ${BOLD}Memory:${NC}"
echo -e "       Total: ${MEM_TOTAL}GB"
echo -e "       Used:  ${MEM_USED}GB"
echo -e "       Usage: $(get_util_color $MEM_USAGE_INT)$MEM_USAGE%${NC} $(get_bar $MEM_USAGE_INT)"
echo ""

echo -e "  ${BOLD}Storage:${NC}"
echo -e "       Total: $DISK_TOTAL"
echo -e "       Used:  $DISK_USED"
echo -e "       Usage: $(get_util_color $DISK_USAGE)$DISK_USAGE%${NC} $(get_bar $DISK_USAGE)"
echo ""

if [ "$HAS_GPU" = true ]; then
    echo -e "  ${BOLD}GPU:${NC} $GPU_NAME"
    echo -e "       Memory: ${GPU_MEM_USED}MB / ${GPU_MEM_TOTAL}MB"
    echo -e "       Usage:  $(get_util_color $GPU_UTIL)$GPU_UTIL%${NC} $(get_bar $GPU_UTIL)"
    echo -e "       Memory: $(get_util_color $GPU_MEM_USAGE_INT)$GPU_MEM_USAGE%${NC} $(get_bar $GPU_MEM_USAGE_INT)"
    echo ""
fi

# Check APN node status
if pgrep -f "apn_node" > /dev/null; then
    APN_STATUS="${GREEN}●${NC} Online"
    APN_UPTIME=$(ps -o etime= -p $(pgrep -f "apn_node") | xargs)
else
    APN_STATUS="${RED}●${NC} Offline"
    APN_UPTIME="N/A"
fi

echo -e "${CYAN}🔗 Network Status:${NC}"
echo -e "       Status: $APN_STATUS"
echo -e "       Uptime: $APN_UPTIME"
echo ""

# Parse discovered peers from log
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${BOLD}🌐 NETWORK PEERS DISCOVERED${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

if [ -f "/tmp/claude-1000/-home-pythia-topos/tasks/b88c5f6.output" ]; then
    PEER_COUNT=$(grep -c "PeerAnnouncement" /tmp/claude-1000/-home-pythia-topos/tasks/b88c5f6.output || echo "0")
    echo -e "${GREEN}Total Peers Discovered: $PEER_COUNT${NC}"
    echo ""

    # Extract unique peers
    grep "📨 Message from apn.discovery" /tmp/claude-1000/-home-pythia-topos/tasks/b88c5f6.output | \
    awk -F'[()]' '{print $2}' | sort -u | while read -r peer_id; do
        if [ ! -z "$peer_id" ]; then
            # Get capabilities for this peer
            CAPS=$(grep "($peer_id)" /tmp/claude-1000/-home-pythia-topos/tasks/b88c5f6.output | \
                   grep -o 'capabilities: \[.*\]' | head -1 | \
                   sed 's/capabilities: //; s/\[//g; s/\]//g; s/"//g')

            echo -e "${BOLD}Node:${NC} $peer_id"
            echo -e "  Capabilities: $CAPS"
            echo -e "  Resources:    ${YELLOW}⚠ Not reported (resources: None)${NC}"
            echo ""
        fi
    done
else
    echo -e "${RED}No peer discovery log found${NC}"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${BOLD}📊 NETWORK SUMMARY${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

TOTAL_NODES=$((PEER_COUNT + 1))
echo -e "  Total Nodes in Network:  ${GREEN}$TOTAL_NODES${NC} (1 master + $PEER_COUNT peers)"
echo -e "  Resource Reporting:      ${YELLOW}⚠ Disabled${NC}"
echo ""
echo -e "  ${YELLOW}Note:${NC} Peer nodes are not currently reporting resource information."
echo -e "  All peer announcements show 'resources: None'."
echo ""
echo -e "  ${CYAN}To enable resource reporting:${NC}"
echo -e "  - Update nodes to include NodeResources in PeerAnnouncePayload"
echo -e "  - Modify apn_node to collect and broadcast system metrics"
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${BOLD}💡 CAPACITY UTILIZATION${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Calculate capacity rating
if (( $(echo "$CPU_USAGE < 30 && $MEM_USAGE < 50" | bc -l) )); then
    CAPACITY_RATING="${GREEN}🟢 Excellent${NC} - High capacity available"
elif (( $(echo "$CPU_USAGE < 60 && $MEM_USAGE < 70" | bc -l) )); then
    CAPACITY_RATING="${YELLOW}🟡 Good${NC} - Moderate capacity available"
else
    CAPACITY_RATING="${RED}🔴 Limited${NC} - High utilization"
fi

echo -e "  Pythia Master: $CAPACITY_RATING"
echo -e "  Peer Nodes:    ${YELLOW}⚠ Unknown${NC} - Awaiting resource reports"
echo ""

echo "╔════════════════════════════════════════════════════════════════════╗"
echo "║  Run './check-network-capacity.sh' anytime to view capacity       ║"
echo "╚════════════════════════════════════════════════════════════════════╝"
echo ""
