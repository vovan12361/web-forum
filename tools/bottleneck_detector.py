#!/usr/bin/env python3
import argparse
import requests
import json
import time
import sys
import os
from datetime import datetime, timedelta

class BottleneckDetector:
    def __init__(self, prometheus_url, jaeger_url):
        self.prometheus_url = prometheus_url
        self.jaeger_url = jaeger_url
    
    def query_prometheus(self, query, time_range="5m"):
        """Query Prometheus for metrics data"""
        try:
            response = requests.get(
                f"{self.prometheus_url}/api/v1/query",
                params={
                    "query": query,
                    "time": datetime.now().timestamp()
                }
            )
            
            if response.status_code == 200:
                result = response.json()
                if result["status"] == "success" and result["data"]["result"]:
                    return result["data"]["result"]
                else:
                    print(f"No data found for query: {query}")
                    return []
            else:
                print(f"Error querying Prometheus: {response.status_code}")
                return []
                
        except requests.exceptions.RequestException as e:
            print(f"Connection error to Prometheus: {e}")
            return []
    
    def detect_slow_endpoints(self, threshold_ms=500):
        """Detect endpoints with high latency"""
        query = f"histogram_quantile(0.95, sum(rate(http_request_duration_seconds_bucket[5m])) by (le, path)) * 1000 > {threshold_ms}"
        results = self.query_prometheus(query)
        
        print("\n=== Slow Endpoints (P95 > 500ms) ===")
        
        if not results:
            print("No slow endpoints detected.")
            return
        
        for result in results:
            if "metric" in result and "value" in result:
                metric = result["metric"]
                value = float(result["value"][1])
                path = metric.get("path", "unknown")
                print(f"Path: {path} - P95 Latency: {value:.2f}ms")
        
        print("\nRecommendation: Review these endpoints for optimization opportunities.")
    
    def detect_high_db_usage(self, threshold=50):
        """Detect high database usage"""
        query = f"rate(db_requests_total[1m]) > {threshold}"
        results = self.query_prometheus(query)
        
        print("\n=== High Database Usage (>50 RPS) ===")
        
        if not results:
            print("No high database usage detected.")
            return
            
        for result in results:
            if "metric" in result and "value" in result:
                value = float(result["value"][1])
                print(f"Database Request Rate: {value:.2f} RPS")
        
        print("\nRecommendation: Consider query optimization, caching, or database scaling.")
    
    def detect_memory_usage(self):
        """Detect high memory usage"""
        query = "process_resident_memory_bytes / 1024 / 1024"  # Convert to MB
        results = self.query_prometheus(query)
        
        print("\n=== Memory Usage ===")
        
        if not results:
            print("No memory usage data available.")
            return
            
        for result in results:
            if "metric" in result and "value" in result:
                value = float(result["value"][1])
                print(f"Memory Usage: {value:.2f} MB")
                
                if value > 500:
                    print("Warning: High memory usage detected.")
                    print("Recommendation: Check for memory leaks or consider scaling.")
    
    def detect_error_rates(self, threshold=0.01):  # 1% error rate
        """Detect high error rates"""
        # Query for HTTP 5xx errors
        query_errors = 'sum(rate(http_server_requests_seconds_count{status=~"5.."}[5m])) / sum(rate(http_server_requests_seconds_count[5m]))'
        results = self.query_prometheus(query_errors)
        
        print("\n=== Error Rates ===")
        
        if not results:
            print("No error rate data available.")
            return
            
        for result in results:
            if "value" in result:
                value = float(result["value"][1])
                error_percentage = value * 100
                print(f"HTTP Error Rate: {error_percentage:.2f}%")
                
                if error_percentage > threshold * 100:
                    print(f"Warning: Error rate exceeds threshold of {threshold * 100}%")
                    print("Recommendation: Investigate logs and traces for error sources.")
    
    def get_slow_traces(self, limit=5):
        """Get slowest traces from Jaeger"""
        # This would normally use Jaeger's API
        # For this example, we'll just print instructions
        print("\n=== Slowest Traces ===")
        print(f"To view slow traces, visit: {self.jaeger_url}")
        print("Search for traces with duration > 500ms")
    
    def analyze_bottlenecks(self):
        """Run all bottleneck detection analyses"""
        print(f"Running bottleneck detection at {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
        print(f"Prometheus URL: {self.prometheus_url}")
        print(f"Jaeger URL: {self.jaeger_url}")
        
        self.detect_slow_endpoints()
        self.detect_high_db_usage()
        self.detect_memory_usage()
        self.detect_error_rates()
        self.get_slow_traces()
        
        print("\n=== Common Performance Issues & Solutions ===")
        print("1. N+1 Query Problem: Look for loops fetching database data")
        print("2. Missing Database Indexes: Check query plans for table scans")
        print("3. Large Payload Sizes: Check response sizes")
        print("4. Resource Contention: Check CPU and memory usage patterns")
        print("5. Connection Pooling: Ensure database connections are pooled properly")
        
        print("\n=== Next Steps ===")
        print("1. Review the source code of slow endpoints")
        print("2. Analyze database queries via tracing")
        print("3. Consider adding caching for frequently accessed data")
        print("4. Profile the application during peak load")
        print("5. Review resource allocation for all services")

def main():
    parser = argparse.ArgumentParser(description="API Bottleneck Detection Tool")
    parser.add_argument("--prometheus-url", default="http://localhost:9090", help="Prometheus URL")
    parser.add_argument("--jaeger-url", default="http://localhost:16686", help="Jaeger UI URL")
    parser.add_argument("--output", default="reports", help="Output directory for reports")
    
    args = parser.parse_args()
    
    # Ensure output directory exists
    os.makedirs(args.output, exist_ok=True)
    
    detector = BottleneckDetector(args.prometheus_url, args.jaeger_url)
    
    # Run analysis
    detector.analyze_bottlenecks()
    
    # Generate a report file
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    report_file = os.path.join(args.output, f"bottleneck_report_{timestamp}.txt")
    
    # Redirect stdout to file to capture the report
    original_stdout = sys.stdout
    with open(report_file, 'w') as f:
        sys.stdout = f
        detector.analyze_bottlenecks()
    
    # Reset stdout
    sys.stdout = original_stdout
    
    print(f"\nBottleneck analysis complete. Report saved to: {report_file}")

if __name__ == "__main__":
    main() 