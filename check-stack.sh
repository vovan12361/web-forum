#!/bin/bash

# Скрипт для проверки состояния Scylla Monitoring Stack

echo "🔍 Проверка состояния Scylla Monitoring Stack..."
echo "=================================================="

# Функция для проверки HTTP endpoint
check_endpoint() {
    local name=$1
    local url=$2
    local expected_code=${3:-200}
    
    echo -n "Проверка $name... "
    
    if curl -s -o /dev/null -w "%{http_code}" "$url" | grep -q "$expected_code"; then
        echo "✅ OK"
        return 0
    else
        echo "❌ FAIL"
        return 1
    fi
}

# Функция для проверки Docker контейнера
check_container() {
    local name=$1
    echo -n "Проверка контейнера $name... "
    
    if docker-compose ps "$name" | grep -q "Up"; then
        echo "✅ UP"
        return 0
    else
        echo "❌ DOWN"
        return 1
    fi
}

echo "📋 Статус Docker контейнеров:"
echo "-------------------------------"
check_container "app"
check_container "scylladb" 
check_container "prometheus"
check_container "grafana"
check_container "loki"
check_container "promtail"
check_container "jaeger"
check_container "alertmanager"

echo ""
echo "🌐 Проверка HTTP endpoints:"
echo "---------------------------"
check_endpoint "Forum API Health" "http://localhost:8080/health"
check_endpoint "Forum API Docs" "http://localhost:8080/docs"
check_endpoint "Prometheus" "http://localhost:9090/-/healthy"
check_endpoint "Grafana" "http://localhost:3000/api/health"
check_endpoint "Loki" "http://localhost:3100/ready"
check_endpoint "Jaeger" "http://localhost:16686/"
check_endpoint "Alertmanager" "http://localhost:9093/-/healthy"

echo ""
echo "📊 Проверка ScyllaDB метрик:"
echo "-----------------------------"
check_endpoint "ScyllaDB Metrics" "http://localhost:9180/metrics"

# Проверка Prometheus targets
echo -n "Проверка Prometheus targets... "
targets_response=$(curl -s "http://localhost:9090/api/v1/targets")
if echo "$targets_response" | jq -r '.data.activeTargets[].health' | grep -q "up"; then
    echo "✅ Targets активны"
    
    # Подробная информация о targets
    echo ""
    echo "📈 Статус Prometheus targets:"
    echo "-----------------------------"
    echo "$targets_response" | jq -r '.data.activeTargets[] | "\(.labels.job): \(.health)"'
else
    echo "❌ Проблемы с targets"
fi

echo ""
echo "📝 Проверка логов Loki:"
echo "------------------------"
echo -n "Проверка наличия логов... "
loki_logs=$(curl -s "http://localhost:3100/loki/api/v1/label" | jq -r '.data[]' 2>/dev/null)
if [ ! -z "$loki_logs" ]; then
    echo "✅ Логи поступают"
    echo "Доступные labels: $loki_logs"
else
    echo "❌ Логи не поступают"
fi

echo ""
echo "🔧 Диагностика проблем:"
echo "------------------------"

# Проверка плагина ScyllaDB в Grafana
echo -n "Проверка плагина ScyllaDB... "
if [ -d "monitoring/grafana/plugins/scylladb-scylla-datasource" ]; then
    echo "✅ Плагин установлен"
else
    echo "❌ Плагин не установлен"
    echo "   💡 Запустите: ./setup-scylla-plugin.sh"
fi

# Проверка размера логов Docker
echo -n "Проверка размера логов Docker... "
docker_logs_size=$(du -sh /var/lib/docker/containers 2>/dev/null | cut -f1)
if [ ! -z "$docker_logs_size" ]; then
    echo "📁 $docker_logs_size"
else
    echo "⚠️  Не удалось определить размер"
fi

echo ""
echo "🎯 Рекомендации:"
echo "----------------"

# Проверим использование ресурсов
echo "💾 Использование ресурсов:"
docker stats --no-stream --format "table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}" | head -10

echo ""
echo "📋 Полезные ссылки:"
echo "-------------------"
echo "• API Docs: http://localhost:8080/docs"
echo "• Grafana: http://localhost:3000 (admin/admin)"
echo "• Prometheus: http://localhost:9090"
echo "• Jaeger: http://localhost:16686"
echo "• Load Testing: http://localhost:8089"
echo ""
echo "🔧 Команды для диагностики:"
echo "---------------------------"
echo "• Логи приложения: docker-compose logs app"
echo "• Логи ScyllaDB: docker-compose logs scylladb"
echo "• Перезапуск Grafana: docker-compose restart grafana"
echo "• Проверка метрик: curl http://localhost:9180/metrics"
