#!/usr/bin/env python3
import argparse
import requests
import time
import json
import sys
import matplotlib.pyplot as plt
import numpy as np
from collections import defaultdict
import datetime
import os

class PerformanceAnalyzer:
    def __init__(self, base_url, jaeger_url):
        self.base_url = base_url
        self.jaeger_url = jaeger_url
        self.results = defaultdict(list)
        self.trace_ids = []
        self.endpoint_stats = defaultdict(lambda: {"count": 0, "total_time": 0, "max_time": 0})
        
    def run_test(self, endpoint, method="GET", iterations=10, payload=None):
        print(f"Testing endpoint: {endpoint} with method {method}")
        
        total_time = 0
        trace_ids = []
        response_times = []
        
        for i in range(iterations):
            start_time = time.time()
            
            try:
                if method.upper() == "GET":
                    response = requests.get(f"{self.base_url}{endpoint}")
                elif method.upper() == "POST":
                    response = requests.post(f"{self.base_url}{endpoint}", json=payload)
                else:
                    print(f"Unsupported method: {method}")
                    return
                
                end_time = time.time()
                elapsed = end_time - start_time
                response_times.append(elapsed * 1000)  # Convert to ms
                
                # Extract trace ID from response headers
                trace_id = response.headers.get("X-Trace-ID", "unknown")
                trace_ids.append(trace_id)
                
                # Extract server processing time if available
                server_time = float(response.headers.get("X-Response-Time-Ms", 0))
                
                print(f"  Request {i+1}/{iterations}: {elapsed*1000:.2f}ms (Server: {server_time}ms) - Status: {response.status_code} - Trace ID: {trace_id}")
                
                # Pause to avoid overwhelming the server
                time.sleep(0.1)
                
            except requests.exceptions.RequestException as e:
                print(f"  Error on request {i+1}: {e}")
        
        self.results[endpoint] = response_times
        self.trace_ids.extend(trace_ids)
        
        # Calculate statistics
        if response_times:
            avg_time = sum(response_times) / len(response_times)
            max_time = max(response_times)
            min_time = min(response_times)
            p95_time = sorted(response_times)[int(len(response_times) * 0.95)]
            
            print(f"\nResults for {endpoint}:")
            print(f"  Average response time: {avg_time:.2f}ms")
            print(f"  P95 response time: {p95_time:.2f}ms")
            print(f"  Min response time: {min_time:.2f}ms")
            print(f"  Max response time: {max_time:.2f}ms")
            
            return {
                "endpoint": endpoint,
                "avg_time": avg_time,
                "p95_time": p95_time,
                "min_time": min_time,
                "max_time": max_time,
                "trace_ids": trace_ids
            }
        
        return None
    
    def analyze_database_queries(self, trace_id):
        # This would fetch trace data from Jaeger API
        # For now, we'll simulate some analysis
        print(f"\nAnalyzing database queries for trace ID: {trace_id}")
        print("  Simulated analysis - in a real system, this would query Jaeger API")
        print("  Database query breakdown:")
        print("    - SELECT queries: 3")
        print("    - INSERT queries: 1")
        print("    - Total DB time: 45ms")
    
    def generate_report(self, output_dir="reports"):
        # Create directory if it doesn't exist
        os.makedirs(output_dir, exist_ok=True)
        
        # Generate timestamp
        timestamp = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
        
        # Create plots
        plt.figure(figsize=(12, 8))
        
        # Bar chart of average response times
        endpoints = list(self.results.keys())
        avg_times = [sum(times) / len(times) if times else 0 for times in self.results.values()]
        
        plt.subplot(2, 1, 1)
        plt.bar(endpoints, avg_times)
        plt.title('Average Response Time by Endpoint')
        plt.xlabel('Endpoint')
        plt.ylabel('Response Time (ms)')
        plt.xticks(rotation=45, ha='right')
        
        # Box plot of response time distributions
        plt.subplot(2, 1, 2)
        plt.boxplot([self.results[endpoint] for endpoint in endpoints], labels=endpoints)
        plt.title('Response Time Distribution by Endpoint')
        plt.xlabel('Endpoint')
        plt.ylabel('Response Time (ms)')
        plt.xticks(rotation=45, ha='right')
        
        plt.tight_layout()
        
        # Save the plot
        plot_path = os.path.join(output_dir, f"performance_report_{timestamp}.png")
        plt.savefig(plot_path)
        
        # Save raw data
        data_path = os.path.join(output_dir, f"performance_data_{timestamp}.json")
        with open(data_path, 'w') as f:
            json.dump({endpoint: times for endpoint, times in self.results.items()}, f)
        
        print(f"\nReport generated:")
        print(f"  Plot: {plot_path}")
        print(f"  Data: {data_path}")
        
        return plot_path, data_path

def main():
    parser = argparse.ArgumentParser(description="API Performance Analysis Tool")
    parser.add_argument("--base-url", default="http://localhost:8080", help="Base URL of the API")
    parser.add_argument("--jaeger-url", default="http://localhost:16686", help="Jaeger UI URL")
    parser.add_argument("--test-all", action="store_true", help="Test all default endpoints")
    parser.add_argument("--endpoint", help="Specific endpoint to test")
    parser.add_argument("--method", default="GET", help="HTTP method (GET, POST)")
    parser.add_argument("--iterations", type=int, default=10, help="Number of requests to make")
    parser.add_argument("--output", default="reports", help="Output directory for reports")
    
    args = parser.parse_args()
    
    analyzer = PerformanceAnalyzer(args.base_url, args.jaeger_url)
    
    if args.test_all:
        # Test a variety of endpoints
        analyzer.run_test("/health", iterations=args.iterations)
        analyzer.run_test("/boards", iterations=args.iterations)
        
        # Create a board first to test other endpoints
        board_data = {
            "name": "Performance Test Board",
            "description": "Board created for performance testing"
        }
        board_result = analyzer.run_test("/boards", method="POST", iterations=1, payload=board_data)
        
        if board_result:
            # Now we can test other endpoints that depend on having data
            try:
                board_response = requests.get(f"{args.base_url}/boards")
                if board_response.status_code == 200:
                    boards = board_response.json()
                    if boards:
                        board_id = boards[0]["id"]
                        # Test endpoints that need a board ID
                        analyzer.run_test(f"/boards/{board_id}", iterations=args.iterations)
                        analyzer.run_test(f"/boards/{board_id}/posts", iterations=args.iterations)
                        
                        # Create a post for comment testing
                        post_data = {
                            "board_id": board_id,
                            "title": "Performance Test Post",
                            "content": "Post created for performance testing",
                            "author": "Performance Tester"
                        }
                        post_result = analyzer.run_test("/posts", method="POST", iterations=1, payload=post_data)
                        
                        if post_result:
                            try:
                                posts_response = requests.get(f"{args.base_url}/boards/{board_id}/posts")
                                if posts_response.status_code == 200:
                                    posts = posts_response.json()
                                    if posts:
                                        post_id = posts[0]["id"]
                                        # Test more endpoints
                                        analyzer.run_test(f"/posts/{post_id}", iterations=args.iterations)
                                        analyzer.run_test(f"/posts/{post_id}/comments", iterations=args.iterations)
                            except:
                                pass
            except:
                pass
        
        # Test the slow endpoint
        analyzer.run_test("/slow", iterations=2)  # Fewer iterations since it's slow
        
    elif args.endpoint:
        # Test just the specified endpoint
        analyzer.run_test(args.endpoint, method=args.method, iterations=args.iterations)
    else:
        print("Error: You must specify --test-all or --endpoint")
        parser.print_help()
        sys.exit(1)
    
    # Generate report
    analyzer.generate_report(args.output)
    
    print("\nPerformance testing completed.")
    if analyzer.trace_ids:
        print(f"\nSample trace IDs for further analysis:")
        for i, trace_id in enumerate(analyzer.trace_ids[:5]):
            print(f"  {i+1}. {trace_id}")
        
        print(f"\nTo view traces, go to: {args.jaeger_url}")
        
        # Analyze one trace as an example
        if analyzer.trace_ids:
            analyzer.analyze_database_queries(analyzer.trace_ids[0])

if __name__ == "__main__":
    main() 