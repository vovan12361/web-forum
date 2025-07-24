.PHONY: help up down logs build rebuild check-monitoring check-stack test-load clean

# Default target
help:
	@echo "Forum Service - Available Commands:"
	@echo "  make up              - Start all services"
	@echo "  make down            - Stop all services"
	@echo "  make logs            - Show application logs"
	@echo "  make build           - Build the application"
	@echo "  make rebuild         - Full rebuild and restart"
	@echo "  make check-monitoring - Validate monitoring stack"
	@echo "  make check-stack     - Complete stack health check"
	@echo "  make test-load       - Run load tests"
	@echo "  make clean           - Clean up resources"

# Start all services
up:
	docker-compose up -d
	@echo "Services starting... Wait 30 seconds for initialization"
	@sleep 30
	@echo "✅ Services should be ready!"
	@echo "📚 API Documentation: http://localhost:8080/docs"
	@echo "📊 Prometheus: http://localhost:9090"
	@echo "📈 Grafana: http://localhost:3000 (admin/admin)"
	@echo "🔍 Jaeger: http://localhost:16686"
	@echo "🧪 Load Testing: http://localhost:8089"

# Stop all services
down:
	docker-compose down

# Show application logs
logs:
	docker-compose logs -f app

# Build the application
build:
	docker-compose build

# Full rebuild and restart
rebuild:
	@echo "🔄 Stopping services..."
	docker-compose down
	@echo "🔨 Building application..."
	docker-compose build
	@echo "🚀 Starting services..."
	docker-compose up -d
	@echo "⏳ Waiting for services to initialize..."
	@sleep 30
	@echo "✅ Services should be ready!"
	@echo "📚 API Documentation: http://localhost:8080/docs"
	@echo "📊 Prometheus: http://localhost:9090"
	@echo "📈 Grafana: http://localhost:3000 (admin/admin)"
	@echo "🔍 Jaeger: http://localhost:16686"
	@echo "🧪 Load Testing: http://localhost:8089"

# Validate monitoring stack
check-monitoring:
	cd tools && python3 check_all.py

# Complete stack health check
check-stack:
	./check-stack.sh

# Run load tests (requires Locust UI)
test-load:
	@echo "🧪 Load testing interface available at:"
	@echo "   http://localhost:8089"
	@echo ""
	@echo "To test alerting:"
	@echo "  1. Set users: 50+, spawn rate: 10"
	@echo "  2. Include the /slow endpoint in your test"
	@echo "  3. Run for 2+ minutes to trigger alerts"

# Clean up resources
clean:
	docker-compose down -v
	docker system prune -f
	@echo "✅ Cleaned up Docker resources"
