#!/usr/bin/env python3
"""
Check that Jaeger is collecting traces and exporting metrics to Prometheus
"""

import requests
import sys
import json
from datetime import datetime, timedelta
from typing import List, Dict

# Jaeger endpoints
JAEGER_API_URL = "http://localhost:16686"
JAEGER_METRICS_URL = "http://localhost:14269"

def check_jaeger_up() -> bool:
    """Check if Jaeger UI is accessible"""
    try:
        response = requests.get(f"{JAEGER_API_URL}/", timeout=5)
        return response.status_code == 200
    except:
        return False

def check_jaeger_metrics() -> bool:
    """Check if Jaeger metrics endpoint is accessible"""
    try:
        response = requests.get(f"{JAEGER_METRICS_URL}/metrics", timeout=5)
        return response.status_code == 200
    except:
        return False

def get_services() -> List[str]:
    """Get list of services from Jaeger"""
    try:
        response = requests.get(f"{JAEGER_API_URL}/api/services", timeout=10)
        if response.status_code == 200:
            return response.json()["data"]
        return []
    except:
        return []

def get_operations(service: str) -> List[str]:
    """Get operations for a service"""
    try:
        params = {"service": service}
        response = requests.get(f"{JAEGER_API_URL}/api/services/{service}/operations", timeout=10)
        if response.status_code == 200:
            return [op["operationName"] for op in response.json()["data"]]
        return []
    except:
        return []

def search_traces(service: str, limit: int = 20) -> Dict:
    """Search for recent traces"""
    try:
        # Look for traces in the last hour
        end_time = datetime.now()
        start_time = end_time - timedelta(hours=1)
        
        params = {
            "service": service,
            "start": int(start_time.timestamp() * 1000000),  # microseconds
            "end": int(end_time.timestamp() * 1000000),
            "limit": limit,
        }
        
        response = requests.get(f"{JAEGER_API_URL}/api/traces", params=params, timeout=15)
        if response.status_code == 200:
            return response.json()
        return {}
    except Exception as e:
        print(f"Error searching traces: {e}")
        return {}

def get_jaeger_metrics() -> List[str]:
    """Get Jaeger metrics"""
    try:
        response = requests.get(f"{JAEGER_METRICS_URL}/metrics", timeout=10)
        if response.status_code == 200:
            lines = response.text.split('\n')
            # Extract metric names (lines that don't start with # and contain a metric)
            metrics = []
            for line in lines:
                if line and not line.startswith('#') and '=' not in line:
                    continue
                if line and not line.startswith('#') and ' ' in line:
                    metric_name = line.split(' ')[0].split('{')[0]
                    if metric_name and metric_name not in metrics:
                        metrics.append(metric_name)
            return sorted(metrics)
        return []
    except:
        return []

def main():
    print("🔍 Checking Jaeger tracing system...")
    
    # Check if Jaeger UI is up
    if not check_jaeger_up():
        print("❌ Jaeger UI is not accessible at", JAEGER_API_URL)
        sys.exit(1)
    
    print("✅ Jaeger UI is accessible")
    
    # Check if Jaeger metrics are available
    if check_jaeger_metrics():
        print("✅ Jaeger metrics endpoint is accessible")
    else:
        print("⚠️  Jaeger metrics endpoint is not accessible")
    
    # Check available services
    print("\n🛠️  Services Reporting to Jaeger:")
    services = get_services()
    if services:
        for service in services:
            print(f"  🔧 {service}")
        
        # Check operations for each service
        for service in services:
            print(f"\n📋 Operations for '{service}':")
            operations = get_operations(service)
            if operations:
                for op in operations[:10]:  # Show first 10 operations
                    print(f"  📊 {op}")
                if len(operations) > 10:
                    print(f"  ... and {len(operations) - 10} more operations")
            else:
                print("  ⚠️  No operations found")
    else:
        print("  ⚠️  No services found reporting to Jaeger")
    
    # Search for recent traces
    if services:
        print("\n🔍 Recent Traces:")
        for service in services[:2]:  # Check first 2 services
            print(f"\n  Service: {service}")
            traces_result = search_traces(service, limit=5)
            
            if traces_result and "data" in traces_result:
                traces = traces_result["data"]
                if traces:
                    print(f"    ✅ Found {len(traces)} recent traces")
                    for i, trace in enumerate(traces[:3]):  # Show details for first 3
                        trace_id = trace["traceID"]
                        spans = trace.get("spans", [])
                        duration_us = trace.get("duration", 0)
                        duration_ms = duration_us / 1000 if duration_us else 0
                        
                        start_time = spans[0].get("startTime", 0) if spans else 0
                        start_datetime = datetime.fromtimestamp(start_time / 1000000) if start_time else "Unknown"
                        
                        print(f"      🔍 Trace {i+1}: {trace_id[:16]}... ({len(spans)} spans, {duration_ms:.1f}ms)")
                        print(f"         ⏰ Started: {start_datetime}")
                        
                        # Show span operations
                        operations = list(set([span.get("operationName", "unknown") for span in spans[:5]]))
                        print(f"         📊 Operations: {', '.join(operations)}")
                else:
                    print(f"    ⚠️  No recent traces found for {service}")
            else:
                print(f"    ❌ Failed to search traces for {service}")
    
    # Check Jaeger-specific metrics
    print("\n📊 Jaeger Metrics:")
    metrics = get_jaeger_metrics()
    if metrics:
        jaeger_metrics = [m for m in metrics if 'jaeger' in m.lower()]
        collector_metrics = [m for m in metrics if 'collector' in m.lower()]
        
        print(f"  📈 Total metrics available: {len(metrics)}")
        
        if jaeger_metrics:
            print(f"  🔧 Jaeger-specific metrics: {len(jaeger_metrics)}")
            for metric in jaeger_metrics[:5]:
                print(f"    - {metric}")
            if len(jaeger_metrics) > 5:
                print(f"    ... and {len(jaeger_metrics) - 5} more")
        
        if collector_metrics:
            print(f"  📥 Collector metrics: {len(collector_metrics)}")
            for metric in collector_metrics[:5]:
                print(f"    - {metric}")
            if len(collector_metrics) > 5:
                print(f"    ... and {len(collector_metrics) - 5} more")
                
        # Look for key metrics
        important_metrics = [
            "jaeger_collector_spans_received_total",
            "jaeger_collector_spans_saved_total", 
            "jaeger_collector_traces_received_total",
            "jaeger_collector_traces_saved_total",
        ]
        
        print("\n🔑 Key Metrics Status:")
        for metric in important_metrics:
            status = "✅" if metric in metrics else "❌"
            print(f"  {status} {metric}")
            
    else:
        print("  ❌ No metrics available from Jaeger")

if __name__ == "__main__":
    main()
