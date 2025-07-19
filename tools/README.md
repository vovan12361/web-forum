# Performance Analysis Tools

This directory contains tools for performance analysis, debugging, and load testing of the Forum API.

## Setup

Install the required dependencies:

```bash
pip install -r requirements.txt
```

## Available Tools

### 1. Performance Analyzer

Analyzes the performance of the API by sending requests and measuring response times.

```bash
./performance_analyzer.py --test-all
```

Options:
- `--base-url`: The base URL of the API (default: http://localhost:8080)
- `--jaeger-url`: The URL of the Jaeger UI (default: http://localhost:16686)
- `--test-all`: Test all default endpoints
- `--endpoint`: Test a specific endpoint
- `--iterations`: Number of requests to make per endpoint (default: 10)
- `--output`: Output directory for reports (default: reports)

### 2. Bottleneck Detector

Analyzes metrics from Prometheus and traces from Jaeger to identify bottlenecks.

```bash
./bottleneck_detector.py
```

Options:
- `--prometheus-url`: The URL of Prometheus (default: http://localhost:9090)
- `--jaeger-url`: The URL of the Jaeger UI (default: http://localhost:16686)
- `--output`: Output directory for reports (default: reports)

### 3. Bottleneck Simulator

Simulates various performance issues to test monitoring and alerting.

```bash
./simulate_bottleneck.py --scenario all
```

Options:
- `--base-url`: The base URL of the API (default: http://localhost:8080)
- `--scenario`: Scenario to simulate (load, n_plus_1, slow, memory, all)
- `--rps`: Requests per second for load testing (default: 10)
- `--duration`: Duration in seconds for each scenario (default: 30)
- `--endpoint`: Endpoint to test for load scenario (default: /boards)

## Common Performance Issues

1. **High Latency**: Check for slow database queries, inefficient code, or resource constraints.
2. **N+1 Query Problem**: Check for loops that make a database query for each item in a collection.
3. **Memory Leaks**: Check for memory that's not being released properly.
4. **Connection Pooling**: Check for improper database connection management.
5. **Missing Indexes**: Check for database queries that perform full table scans.

## Performance Debugging Workflow

1. **Identify the Issue**: Use monitoring tools to identify where the performance problem lies.
2. **Reproduce the Issue**: Use the bottleneck simulator to reproduce the issue in a controlled environment.
3. **Analyze the Problem**: Use the performance analyzer and bottleneck detector to analyze the root cause.
4. **Fix the Issue**: Implement the necessary code changes to address the problem.
5. **Verify the Fix**: Run tests again to ensure the issue is resolved.

## Sample Usage for Debugging a Slow Endpoint

```bash
# Step 1: Run performance analyzer to identify slow endpoints
./performance_analyzer.py --test-all

# Step 2: Generate load on the slow endpoint to collect more data
./simulate_bottleneck.py --scenario slow --duration 60

# Step 3: Analyze bottlenecks using metrics
./bottleneck_detector.py

# Step 4: Fix the issue in the code

# Step 5: Verify the fix
./performance_analyzer.py --endpoint /slow
``` 