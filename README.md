# Forum Service

A scalable forum API service with comprehensive monitoring, testing, and observability capabilities.

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [System Architecture](#system-architecture)
- [Setup Instructions](#setup-instructions)
- [API Documentation](#api-documentation)
- [Load Testing](#load-testing)
- [Monitoring](#monitoring)
- [Alerting](#alerting)
- [Performance Debugging](#performance-debugging)
- [Infrastructure as Code](#infrastructure-as-code)

## Overview

This project implements a forum service API with boards, posts, and comments functionality. The service is built with Rust and Actix-Web framework, using ScyllaDB (Cassandra-compatible) as the database.

The main focus of this project is not only the business logic but also implementing a comprehensive observability and monitoring infrastructure to ensure the service can be properly operated in production.

## Features

- **Core Functionality**:
  - Create and view discussion boards
  - Create and view posts on boards
  - Add and view comments on posts

- **Technical Features**:
  - RESTful API with Swagger documentation at `/docs`
  - **Complete Observability Stack**:
    - 📊 Prometheus metrics collection (application + infrastructure)
    - 🔍 Jaeger distributed tracing with automatic instrumentation  
    - 📋 Loki centralized logging with trace correlation
    - 📈 Grafana dashboards for visualization
    - 🚨 AlertManager with Telegram notifications
  - **Performance Monitoring**:
    - Request rate, latency (P50/P95/P99), and error tracking
    - Database performance monitoring
    - Active request tracking
    - Cross-service trace correlation
  - **Load Testing**: Locust integration for stress testing
  - **Infrastructure as Code**: All monitoring configs version-controlled

## System Architecture

The system consists of the following components:

- **Forum API Service**: Rust-based API for forum functionality
- **ScyllaDB**: NoSQL database for storing forum data
- **Prometheus**: Metrics collection and alerting
- **Grafana**: Dashboard visualization for metrics
- **AlertManager**: Alert routing directly to Telegram
- **Jaeger**: Distributed tracing visualization
- **Locust**: Load testing service with web UI

## Setup Instructions

### Prerequisites

- Docker and Docker Compose installed
- Telegram Bot Token (for alerts)

### Environment Setup

1. Create a `.env` file in the project root with the following variables:

```
TELEGRAM_BOT_TOKEN=your_telegram_bot_token
TELEGRAM_CHAT_ID=your_telegram_chat_id
```

To create a Telegram bot, talk to [@BotFather](https://t.me/botfather) on Telegram and follow the instructions.
To get your chat ID, create a public channel and note the username (e.g., @your_channel_name).

### Running the Services

1. Start all services:

```bash
# Using make (recommended)
make up

# Or using docker-compose directly
docker-compose up -d
```

2. Wait for all services to initialize (about 30 seconds).

3. Access the following endpoints:
   - Forum API: http://localhost:8080
   - API Documentation: http://localhost:8080/docs
   - Prometheus: http://localhost:9090
   - Grafana: http://localhost:3000 (admin/admin)
   - Jaeger UI: http://localhost:16686
   - Locust (Load Testing): http://localhost:8089

4. Verify the complete monitoring setup:

```bash
# Using make
make check-monitoring

# Or directly
cd tools && python3 check_all.py
```

This will validate that all observability components are working correctly.

### Quick Commands

```bash
make help              # Show all available commands
make up                # Start all services
make down              # Stop all services  
make logs              # View application logs
make check-monitoring  # Validate observability stack
make test-load         # Open load testing interface
make clean             # Clean up Docker resources
```

## API Documentation

The API documentation is available through Swagger UI at http://localhost:8080/docs. This interactive documentation allows you to explore and test all endpoints directly from the browser.

### Main Endpoints

- `GET /health` - Health check endpoint
- `GET /metrics` - Prometheus metrics

#### Boards
- `GET /boards` - Get all boards
- `POST /boards` - Create a new board
- `GET /boards/{board_id}` - Get a specific board

#### Posts
- `POST /posts` - Create a new post
- `GET /posts/{post_id}` - Get a specific post
- `GET /boards/{board_id}/posts` - Get all posts for a board

#### Comments
- `POST /comments` - Create a new comment
- `GET /posts/{post_id}/comments` - Get all comments for a post

#### Test Endpoint
- `GET /slow` - Intentionally slow endpoint for testing alerts

## Load Testing

The Locust load testing UI is available at http://localhost:8089. This tool allows you to simulate various load patterns on the service.

### Running a Load Test

1. Open http://localhost:8089 in your browser
2. Enter the number of users to simulate and the spawn rate
3. Click "Start swarming" to begin the test
4. Monitor the real-time statistics and charts

To trigger the high latency alert, run a load test with at least 50 users that includes the `/slow` endpoint.

To trigger the high DB RPS alert, run a load test with at least 200 users focused on write operations.

## Monitoring

### Prometheus Integration

The service has complete integration with Prometheus for metrics collection, including:

- **HTTP Request Metrics**: Request rates, latencies (P50, P95, P99), active requests
- **Database Metrics**: Query rates, database response times
- **Application Metrics**: Business logic performance, error rates
- **Jaeger Tracing Metrics**: Trace collection and processing statistics
- **Log Aggregation Metrics**: Loki and Promtail performance metrics

All endpoints are instrumented with distributed tracing using OpenTelemetry and Jaeger, providing:
- Detailed request traces with timing information
- Cross-service correlation (when extended)
- Automatic trace context propagation
- Structured logging with trace correlation

### Grafana Dashboards

Access Grafana at http://localhost:3000 (login with admin/admin)

Available dashboards:
1. **Forum API Dashboard** - Shows API request rates, response times, error rates, and active requests
2. **ScyllaDB Dashboard** - Shows database metrics including request rates, latencies, and resource usage
3. **Logs Dashboard** - Aggregated application and system logs from Loki with filtering capabilities
4. **Jaeger Tracing Dashboard** - Tracing system performance and trace processing metrics

### Prometheus Metrics

Access Prometheus at http://localhost:9090

Key metrics available:
- `api_requests_total` - Total number of API requests
- `http_request_duration_seconds` - Request duration histogram for P50/P95/P99 calculations
- `http_requests_active` - Number of currently active requests
- `db_requests_total` - Total database requests
- `jaeger_*` - Jaeger collector and query service metrics
- `loki_*` - Log aggregation service metrics

### Log Collection

Logs are automatically collected from all services using Promtail and stored in Loki:
- **Application Logs**: Structured JSON logs with trace correlation
- **Container Logs**: All Docker container logs are scraped automatically
- **System Logs**: System-level logs from the host

All application logs include:
- Trace ID for correlation with Jaeger traces
- Structured fields (log level, component, timing)
- Request context (method, path, status, duration)
- Business logic events (board created, post published, etc.)

Example log queries in Grafana:
```
{job="containerlogs", container_name=~".*backend.*"} |= "ERROR"
{job="forum-app"} | json | level="INFO" | line_format "{{.timestamp}} [{{.level}}] {{.message}}"
```

### Distributed Tracing

Jaeger tracing is fully integrated with the application:
- **Automatic Instrumentation**: All HTTP endpoints are automatically traced
- **Database Operations**: All ScyllaDB queries are included in traces
- **Cross-Service Ready**: Ready for microservice environments
- **Prometheus Integration**: Jaeger metrics are collected by Prometheus

Access Jaeger UI at http://localhost:16686 to:
- View individual request traces
- Analyze service dependencies
- Debug performance bottlenecks
- Correlate with application logs using trace IDs

## Alerting

Alerts are configured to be sent directly to Telegram via AlertManager's native Telegram integration.

### Configured Alerts

1. **High P99 Latency** - Triggered when the P99 latency exceeds 500ms for more than 1 minute
2. **High DB Request Rate** - Triggered when the database request rate exceeds 100 RPS for more than 1 minute

### Testing Alerts

To test the high latency alert:
1. Run a load test with at least 50 users targeting the `/slow` endpoint
2. Continue for at least 2 minutes
3. Check your Telegram channel for alerts from AlertManager

## Performance Debugging

For performance debugging, the following tools are available:

### Integrated Observability Stack

The service includes a complete observability stack with automatic integration:

1. **📊 Prometheus Metrics Collection**:
   - Application metrics (request rates, latencies, errors)
   - Infrastructure metrics (Jaeger, Loki, AlertManager, Grafana)
   - Database metrics (ScyllaDB performance)
   - All metrics automatically scraped and stored

2. **📋 Distributed Tracing with Jaeger** (http://localhost:16686):
   - Automatic instrumentation of all HTTP endpoints
   - Database query tracing with timing
   - Cross-service correlation ready
   - Trace metrics exported to Prometheus

3. **📝 Centralized Logging with Loki**:
   - Automatic log collection from all containers
   - Structured application logs with trace correlation
   - Log metrics exported to Prometheus
   - Query logs directly in Grafana

4. **🎯 Grafana Dashboards** (http://localhost:3000):
   - API performance dashboard with P50/P95/P99 latencies
   - Database performance monitoring
   - Log aggregation and search interface
   - Jaeger tracing metrics visualization

### Validation Tools

Use the included monitoring validation tools to verify your observability setup:

```bash
# Check that all metrics are being collected
cd tools && python3 check_metrics.py

# Verify log collection and parsing
python3 check_logs.py

# Validate tracing system health
python3 check_tracing.py
```

### Request Tracing Headers

Each request receives correlation headers:
- `X-Trace-ID`: Unique trace identifier for correlation
- `X-Response-Time-Ms`: Request processing time

Use the trace ID to correlate requests across:
- Application logs (structured with trace_id field)
- Jaeger traces (search by trace ID)
- Prometheus metrics (via tracing exemplars)

## Infrastructure as Code

All monitoring and alerting configurations are defined as code in the following locations:

- Prometheus configuration: `monitoring/prometheus/prometheus.yml`
- Alert rules: `monitoring/prometheus/rules.yml`
- AlertManager configuration: `monitoring/alertmanager/config.yml`
- Grafana dashboards: `monitoring/grafana/dashboards/`
- Grafana provisioning: `monitoring/grafana/provisioning/` 