#!/usr/bin/env python3
import argparse
import requests
import time
import random
import threading
from concurrent.futures import ThreadPoolExecutor
import sys

class BottleneckSimulator:
    def __init__(self, base_url):
        self.base_url = base_url
        self.running = False
    
    def send_request(self, endpoint, method="GET", payload=None):
        """Send a request to the API"""
        try:
            if method.upper() == "GET":
                response = requests.get(f"{self.base_url}{endpoint}")
            elif method.upper() == "POST":
                response = requests.post(f"{self.base_url}{endpoint}", json=payload)
            
            status = response.status_code
            duration = response.elapsed.total_seconds() * 1000  # in milliseconds
            
            return {
                "status": status,
                "duration": duration,
                "trace_id": response.headers.get("X-Trace-ID", "unknown")
            }
        except requests.exceptions.RequestException as e:
            print(f"Error sending request to {endpoint}: {e}")
            return {"status": 0, "duration": 0, "trace_id": "error"}
    
    def simulate_load(self, endpoint, requests_per_second, duration_seconds, method="GET", payload=None):
        """Simulate load on a specific endpoint"""
        self.running = True
        start_time = time.time()
        end_time = start_time + duration_seconds
        request_count = 0
        error_count = 0
        
        print(f"\nSimulating {requests_per_second} RPS on {endpoint} for {duration_seconds} seconds...")
        
        with ThreadPoolExecutor(max_workers=min(50, requests_per_second)) as executor:
            while self.running and time.time() < end_time:
                loop_start = time.time()
                
                # Submit requests for this second
                futures = []
                for _ in range(requests_per_second):
                    if not self.running or time.time() >= end_time:
                        break
                    
                    futures.append(executor.submit(self.send_request, endpoint, method, payload))
                
                # Process results
                for future in futures:
                    try:
                        result = future.result()
                        request_count += 1
                        if result["status"] >= 400:
                            error_count += 1
                        
                        sys.stdout.write(f"\rRequests: {request_count}, Errors: {error_count}, Elapsed: {int(time.time() - start_time)}s")
                        sys.stdout.flush()
                    except Exception as e:
                        print(f"\nError processing request: {e}")
                
                # Calculate how long to sleep
                elapsed = time.time() - loop_start
                if elapsed < 1.0:
                    time.sleep(1.0 - elapsed)
        
        print(f"\nSimulation complete: {request_count} requests sent, {error_count} errors")
    
    def simulate_n_plus_1_problem(self, board_endpoint="/boards", post_endpoint="/posts", comment_endpoint="/comments", duration_seconds=60):
        """Simulate an N+1 query problem by creating data and then fetching it inefficiently"""
        print("\nSimulating N+1 Query Problem")
        
        # First, create some test data
        board_data = {"name": "Bottleneck Test Board", "description": "Board for testing N+1 problem"}
        
        # Create a board
        board_response = self.send_request(board_endpoint, method="POST", payload=board_data)
        if board_response["status"] != 201:
            print("Failed to create test board")
            return
        
        # Get the board ID
        boards_response = requests.get(f"{self.base_url}{board_endpoint}")
        if boards_response.status_code != 200:
            print("Failed to fetch boards")
            return
        
        boards = boards_response.json()
        if not boards:
            print("No boards available")
            return
        
        board_id = boards[0]["id"]
        
        # Create multiple posts for this board
        post_count = 20
        for i in range(post_count):
            post_data = {
                "board_id": board_id,
                "title": f"Bottleneck Test Post {i+1}",
                "content": f"This is test post {i+1} for bottleneck testing",
                "author": "Bottleneck Tester"
            }
            self.send_request(post_endpoint, method="POST", payload=post_data)
        
        # Now simulate an inefficient endpoint that fetches each post individually
        # In a real scenario, this would be an actual endpoint that's inefficient
        print(f"\nSimulating {duration_seconds} seconds of inefficient data access pattern...")
        
        self.running = True
        start_time = time.time()
        end_time = start_time + duration_seconds
        
        with ThreadPoolExecutor(max_workers=10) as executor:
            while self.running and time.time() < end_time:
                # This simulates an inefficient API that makes N+1 queries
                # First get all posts
                posts_endpoint = f"{board_endpoint}/{board_id}/posts"
                posts_response = requests.get(f"{self.base_url}{posts_endpoint}")
                
                if posts_response.status_code == 200:
                    posts = posts_response.json()
                    
                    # Then make a separate request for each post (the "+1" queries)
                    for post in posts:
                        post_id = post["id"]
                        executor.submit(self.send_request, f"{post_endpoint}/{post_id}")
                
                # Sleep a bit to avoid overwhelming the system
                time.sleep(0.5)
        
        print("\nN+1 query simulation complete")
    
    def simulate_slow_queries(self, duration_seconds=30):
        """Hit the artificially slow endpoint to generate slow traces"""
        print(f"\nSimulating slow queries for {duration_seconds} seconds...")
        
        self.running = True
        start_time = time.time()
        end_time = start_time + duration_seconds
        request_count = 0
        
        with ThreadPoolExecutor(max_workers=5) as executor:
            while self.running and time.time() < end_time:
                futures = []
                for _ in range(5):  # 5 concurrent slow requests
                    futures.append(executor.submit(self.send_request, "/slow"))
                
                for future in futures:
                    try:
                        result = future.result()
                        request_count += 1
                        sys.stdout.write(f"\rSlow requests: {request_count}, Elapsed: {int(time.time() - start_time)}s")
                        sys.stdout.flush()
                    except Exception:
                        pass
                
                # Don't hammer the system too hard
                time.sleep(0.5)
        
        print("\nSlow query simulation complete")
    
    def simulate_memory_leak(self, duration_seconds=30):
        """Create a simulation that would trigger memory growth"""
        print(f"\nSimulating memory pressure for {duration_seconds} seconds...")
        
        # This just simulates what a memory leak might do to the system
        # by sending many requests with large payloads
        self.running = True
        start_time = time.time()
        end_time = start_time + duration_seconds
        request_count = 0
        
        with ThreadPoolExecutor(max_workers=5) as executor:
            while self.running and time.time() < end_time:
                # Create a large payload
                large_content = "X" * 1024 * 10  # 10KB of data
                
                large_payload = {
                    "name": f"Memory Test Board {random.randint(1, 10000)}",
                    "description": large_content
                }
                
                futures = []
                for _ in range(5):
                    futures.append(executor.submit(
                        self.send_request, "/boards", method="POST", payload=large_payload
                    ))
                
                for future in futures:
                    try:
                        future.result()
                        request_count += 1
                        sys.stdout.write(f"\rLarge requests: {request_count}, Elapsed: {int(time.time() - start_time)}s")
                        sys.stdout.flush()
                    except Exception:
                        pass
                
                time.sleep(0.2)
        
        print("\nMemory pressure simulation complete")
    
    def stop(self):
        """Stop all simulations"""
        self.running = False

def main():
    parser = argparse.ArgumentParser(description="API Performance Issue Simulator")
    parser.add_argument("--base-url", default="http://localhost:8080", help="Base URL of the API")
    parser.add_argument("--scenario", choices=["load", "n_plus_1", "slow", "memory", "all"], 
                        default="all", help="Scenario to simulate")
    parser.add_argument("--rps", type=int, default=10, help="Requests per second for load testing")
    parser.add_argument("--duration", type=int, default=30, help="Duration in seconds for each scenario")
    parser.add_argument("--endpoint", default="/boards", help="Endpoint to test for load scenario")
    
    args = parser.parse_args()
    
    simulator = BottleneckSimulator(args.base_url)
    
    try:
        if args.scenario == "load" or args.scenario == "all":
            simulator.simulate_load(args.endpoint, args.rps, args.duration)
        
        if args.scenario == "n_plus_1" or args.scenario == "all":
            simulator.simulate_n_plus_1_problem(duration_seconds=args.duration)
        
        if args.scenario == "slow" or args.scenario == "all":
            simulator.simulate_slow_queries(duration_seconds=args.duration)
        
        if args.scenario == "memory" or args.scenario == "all":
            simulator.simulate_memory_leak(duration_seconds=args.duration)
            
    except KeyboardInterrupt:
        print("\nSimulation interrupted by user")
        simulator.stop()
    
    print("\nAll simulations complete")

if __name__ == "__main__":
    main() 