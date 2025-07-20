#!/usr/bin/env python3
"""
Check that logs are being collected by Loki and are available for querying
"""

import requests
import sys
import json
from datetime import datetime, timedelta
from typing import List, Dict

# Loki endpoint
LOKI_URL = "http://localhost:3100"

def check_loki_up() -> bool:
    """Check if Loki is accessible"""
    try:
        response = requests.get(f"{LOKI_URL}/ready", timeout=5)
        return response.status_code == 200
    except:
        return False

def get_log_labels() -> List[str]:
    """Get all available log labels"""
    try:
        response = requests.get(f"{LOKI_URL}/loki/api/v1/labels", timeout=10)
        if response.status_code == 200:
            return response.json()["data"]
        return []
    except:
        return []

def get_label_values(label: str) -> List[str]:
    """Get values for a specific label"""
    try:
        response = requests.get(f"{LOKI_URL}/loki/api/v1/label/{label}/values", timeout=10)
        if response.status_code == 200:
            return response.json()["data"]
        return []
    except:
        return []

def query_logs(query: str, limit: int = 100) -> Dict:
    """Query logs from Loki"""
    try:
        # Query logs from the last hour
        end_time = datetime.now()
        start_time = end_time - timedelta(hours=1)
        
        params = {
            "query": query,
            "start": int(start_time.timestamp() * 1000000000),  # nanoseconds
            "end": int(end_time.timestamp() * 1000000000),
            "limit": limit,
            "direction": "backward"
        }
        
        response = requests.get(f"{LOKI_URL}/loki/api/v1/query_range", params=params, timeout=15)
        if response.status_code == 200:
            return response.json()["data"]
        return {}
    except Exception as e:
        print(f"Error querying logs: {e}")
        return {}

def main():
    print("📝 Checking Loki log collection...")
    
    # Check if Loki is up
    if not check_loki_up():
        print("❌ Loki is not accessible at", LOKI_URL)
        sys.exit(1)
    
    print("✅ Loki is accessible")
    
    # Check available labels
    print("\n🏷️  Available Log Labels:")
    labels = get_log_labels()
    for label in labels[:10]:  # Show first 10 labels
        print(f"  📋 {label}")
    if len(labels) > 10:
        print(f"  ... and {len(labels) - 10} more labels")
    
    # Check job values (most important label)
    if "job" in labels:
        print("\n👷 Available Jobs:")
        jobs = get_label_values("job")
        for job in jobs:
            print(f"  🔧 {job}")
    
    # Check container names
    if "container_name" in labels:
        print("\n🐳 Available Containers:")
        containers = get_label_values("container_name")
        for container in containers[:10]:
            print(f"  📦 {container}")
    
    # Sample log queries
    print("\n📋 Sample Log Queries:")
    
    sample_queries = [
        ('{job="containerlogs"}', "All container logs"),
        ('{job="containerlogs", container_name=~".*backend.*"}', "Backend application logs"),
        ('{job="forum-app"}', "Forum app specific logs"),
        ('{job="syslog"}', "System logs"),
    ]
    
    for query, description in sample_queries:
        print(f"\n🔍 {description}:")
        print(f"   Query: {query}")
        
        result = query_logs(query, limit=5)
        if result and "result" in result:
            total_logs = 0
            for stream in result["result"]:
                stream_logs = len(stream.get("values", []))
                total_logs += stream_logs
                
                # Show labels for this stream
                labels_str = ", ".join([f"{k}={v}" for k, v in stream.get("stream", {}).items()])
                print(f"   📊 Stream [{labels_str}]: {stream_logs} log entries")
                
                # Show sample log entries
                for i, log_entry in enumerate(stream.get("values", [])[:2]):  # Show first 2 entries
                    timestamp_ns, log_line = log_entry
                    # Convert nanoseconds to datetime
                    timestamp = datetime.fromtimestamp(int(timestamp_ns) / 1000000000)
                    print(f"      {timestamp.strftime('%H:%M:%S')}: {log_line[:100]}{'...' if len(log_line) > 100 else ''}")
            
            if total_logs > 0:
                print(f"   ✅ Found {total_logs} log entries")
            else:
                print(f"   ⚠️  No log entries found")
        else:
            print(f"   ❌ Query failed or returned no results")
    
    # Check for recent application logs with trace information
    print("\n🔍 Checking for Trace-Enabled Logs:")
    trace_query = '{job="containerlogs", container_name=~".*backend.*"} |= "trace_id"'
    result = query_logs(trace_query, limit=3)
    
    if result and "result" in result:
        trace_logs = sum(len(stream.get("values", [])) for stream in result["result"])
        if trace_logs > 0:
            print(f"   ✅ Found {trace_logs} log entries with trace information")
            # Show sample trace log
            for stream in result["result"]:
                for log_entry in stream.get("values", [])[:1]:
                    timestamp_ns, log_line = log_entry
                    print(f"   📋 Sample: {log_line[:200]}{'...' if len(log_line) > 200 else ''}")
        else:
            print("   ⚠️  No trace-enabled logs found")
    else:
        print("   ❌ Failed to query trace logs")

if __name__ == "__main__":
    main()
