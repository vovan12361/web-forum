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
  - Distributed tracing with Jaeger
  - Metrics collection with Prometheus
  - Performance visualization with Grafana
  - Load testing with Locust
  - Automated alerts to Telegram

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
docker-compose up -d
```

2. Wait for all services to initialize.

3. Access the following endpoints:
   - Forum API: http://localhost:8080
   - API Documentation: http://localhost:8080/docs
   - Prometheus: http://localhost:9090
   - Grafana: http://localhost:3000 (admin/admin)
   - Jaeger UI: http://localhost:16686
   - Locust (Load Testing): http://localhost:8089

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

### Grafana Dashboards

Access Grafana at http://localhost:3000 (login with admin/admin)

Available dashboards:
1. **Forum API Dashboard** - Shows API request rates, response times, and error rates
2. **ScyllaDB Dashboard** - Shows database metrics including request rates, latencies, and resource usage

### Prometheus Metrics

Access Prometheus at http://localhost:9090

Key metrics available:
- `api_requests_total` - Total number of API requests
- `db_requests_total` - Total number of database requests
- `http_request_duration_seconds` - API request latency histograms

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

1. **Jaeger Tracing** (http://localhost:16686):
   - View detailed traces of requests through the system
   - Identify bottlenecks in request processing
   - Analyze service dependencies

2. **Grafana Dashboards**:
   - View real-time metrics and historical trends
   - Compare performance before and after changes

3. **Request Tracing Headers**:
   - Each request receives a `X-Trace-ID` header
   - Use this ID to correlate requests across logs and traces

## Infrastructure as Code

All monitoring and alerting configurations are defined as code in the following locations:

- Prometheus configuration: `monitoring/prometheus/prometheus.yml`
- Alert rules: `monitoring/prometheus/rules.yml`
- AlertManager configuration: `monitoring/alertmanager/config.yml`
- Grafana dashboards: `monitoring/grafana/dashboards/`
- Grafana provisioning: `monitoring/grafana/provisioning/` 