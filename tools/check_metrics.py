#!/usr/bin/env python3
"""
Check that all expected metrics are being scraped by Prometheus
"""

import requests
import sys
import time
from typing import List, Dict

# Prometheus endpoint
PROMETHEUS_URL = "http://localhost:9090"

# Expected metrics and their sources
EXPECTED_METRICS = {
    "api_requests_total": "forum-api",
    "http_request_duration_seconds": "forum-api", 
    "http_requests_active": "forum-api",
    "db_requests_total": "forum-api",
    "prometheus_": "prometheus",
    "jaeger_": "jaeger-collector",
    "loki_": "loki",
    "promtail_": "promtail",
    "alertmanager_": "alertmanager",
    "grafana_": "grafana",
}

def check_prometheus_up() -> bool:
    """Check if Prometheus is accessible"""
    try:
        response = requests.get(f"{PROMETHEUS_URL}/api/v1/query?query=up", timeout=5)
        return response.status_code == 200
    except:
        return False

def get_all_metrics() -> List[str]:
    """Get all metrics from Prometheus"""
    try:
        response = requests.get(f"{PROMETHEUS_URL}/api/v1/label/__name__/values", timeout=10)
        if response.status_code == 200:
            return response.json()["data"]
        return []
    except:
        return []

def check_targets() -> Dict[str, str]:
    """Check Prometheus target status"""
    try:
        response = requests.get(f"{PROMETHEUS_URL}/api/v1/targets", timeout=10)
        if response.status_code == 200:
            targets = response.json()["data"]["activeTargets"]
            return {target["job"]: target["health"] for target in targets}
        return {}
    except:
        return {}

def main():
    print("🔍 Checking Prometheus metrics collection...")
    
    # Check if Prometheus is up
    if not check_prometheus_up():
        print("❌ Prometheus is not accessible at", PROMETHEUS_URL)
        sys.exit(1)
    
    print("✅ Prometheus is accessible")
    
    # Check targets
    print("\n📊 Target Status:")
    targets = check_targets()
    for job, health in targets.items():
        status = "✅" if health == "up" else "❌"
        print(f"  {status} {job}: {health}")
    
    # Check metrics
    print("\n📈 Metrics Availability:")
    all_metrics = get_all_metrics()
    
    for metric_prefix, source in EXPECTED_METRICS.items():
        matching_metrics = [m for m in all_metrics if m.startswith(metric_prefix)]
        if matching_metrics:
            print(f"  ✅ {metric_prefix}* ({len(matching_metrics)} metrics from {source})")
            # Show first few metrics as examples
            for metric in matching_metrics[:3]:
                print(f"    - {metric}")
            if len(matching_metrics) > 3:
                print(f"    ... and {len(matching_metrics) - 3} more")
        else:
            print(f"  ❌ {metric_prefix}* (no metrics found from {source})")
    
    print(f"\n📊 Total metrics available: {len(all_metrics)}")
    
    # Check specific application metrics with data
    print("\n🔢 Sample Metric Values:")
    sample_queries = [
        ("api_requests_total", "Total API requests"),
        ("http_requests_active", "Currently active requests"),
        ("up{job='forum-api'}", "Forum API availability"),
        ("up{job='jaeger-collector'}", "Jaeger collector availability"),
        ("up{job='loki'}", "Loki availability"),
    ]
    
    for query, description in sample_queries:
        try:
            response = requests.get(f"{PROMETHEUS_URL}/api/v1/query?query={query}", timeout=5)
            if response.status_code == 200:
                result = response.json()["data"]["result"]
                if result:
                    value = result[0]["value"][1]
                    print(f"  📊 {description}: {value}")
                else:
                    print(f"  ⚠️  {description}: No data")
            else:
                print(f"  ❌ {description}: Query failed")
        except Exception as e:
            print(f"  ❌ {description}: Error - {e}")

if __name__ == "__main__":
    main()
