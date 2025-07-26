#!/bin/bash

# Rust Performance Profiling Toolkit
# Usage: ./profile.sh [command] [options]

set -e

CONTAINER_NAME="web-forum_app_1"
PROFILING_DIR="./profiling"
SLOW_ENDPOINT="http://localhost:8080/slow"
COMPOSE_FILE="docker-compose.profiling.yml"

# Ensure profiling directory exists
mkdir -p $PROFILING_DIR

print_usage() {
    echo "Usage: $0 <command> [options]"
    echo ""
    echo "Commands:"
    echo "  perf [duration]          - Run perf record and generate report"
    echo ""
    echo "Examples:"
    echo "  $0 perf 45                              # Perf profiling for 45 seconds"
}

run_perf_profiling() {
    local duration=${1:-30}
    local output_file="$PROFILING_DIR/perf_report_$(date +%Y%m%d_%H%M%S).txt"
    
    echo "Running perf profiling for ${duration} seconds..."
    
    local pid=$(docker exec $CONTAINER_NAME pidof backend)
    
    if [ -z "$pid" ]; then
        echo "Error: Cannot find backend process"
        exit 1
    fi
    
    echo "Backend PID: $pid"
    
    # Start load testing in background
    echo "Starting load test..."
    (
        for i in $(seq 1 $((duration * 2))); do
            curl -s "$SLOW_ENDPOINT" > /dev/null &
            sleep 0.5
        done
        wait
    ) &
    
    # Run perf record with better symbol resolution
    docker exec $CONTAINER_NAME bash -c "
        perf record -F 997 -p $pid -g --call-graph=dwarf --user-callchains -o /app/profiling/perf.data -- sleep $duration
        perf report -i /app/profiling/perf.data --no-children --stdio --header --show-total-period > /app/profiling/perf_report.txt
        perf report -i /app/profiling/perf.data --no-children --stdio --sort=symbol --header > /app/profiling/perf_symbols.txt
        perf script -i /app/profiling/perf.data --header --fields=comm,pid,tid,time,event,ip,sym,dso,addr > /app/profiling/perf_script.txt
    "
    
    echo "Perf profiling completed: $output_file"
}

# Main script logic
case "${1:-help}" in
    "perf")
        run_perf_profiling "${2:-30}"
        ;;
    "help"|*)
        print_usage
        ;;
esac
