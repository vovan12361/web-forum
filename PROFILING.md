# Rust Performance Profiling Guide

Этот проект включает полный набор инструментов для профилирования Rust приложения, аналогичный `pprof` в Go.

## Быстрый старт

### 1. Настройка окружения для профилирования

```bash
./profile.sh setup
```

Это запустит контейнеры с поддержкой профилирования и настроит все необходимые инструменты.

### 2. Основные команды профилирования

#### Flamegraph (рекомендуется для начала)
```bash
# Создать flamegraph на 30 секунд (по умолчанию)
./profile.sh flamegraph

# Создать flamegraph на 60 секунд
./profile.sh flamegraph 60
```

#### CPU профилирование
```bash
# Детальное CPU профилирование
./profile.sh cpu-profile 45

# Стандартное perf профилирование
./profile.sh perf 30
```

#### Профилирование памяти
```bash
./profile.sh memory-profile
```

#### Трассировка системных вызовов
```bash
./profile.sh strace
```

#### Нагрузочное тестирование во время профилирования
```bash
./profile.sh load-test 60
```

### 3. Анализ результатов

```bash
./profile.sh analyze
```

## Инструменты профилирования

### 🔥 Flamegraph
- **Назначение**: Визуализация стека вызовов и времени выполнения
- **Файлы**: `profiling/flamegraph_*.svg`
- **Как читать**: Ширина блока = время выполнения, высота = глубина стека
- **Открыть**: В браузере (Firefox, Chrome)

### 📊 Perf
- **Назначение**: Детальная статистика производительности
- **Файлы**: `profiling/perf_report_*.txt`, `profiling/perf_symbols_*.txt`
- **Показывает**: Горячие функции, количество циклов CPU, cache misses

### 💾 Memory Profiling
- **Назначение**: Анализ использования памяти
- **Файлы**: `profiling/memory_*.txt`
- **Показывает**: Page faults, RSS, heap usage

### 🔍 Strace
- **Назначение**: Анализ системных вызовов
- **Файлы**: `profiling/strace_*.txt`
- **Показывает**: Медленные системные вызовы, I/O операции

## Анализ эндпоинта /slow

### Пример использования для анализа `/slow` эндпоинта:

```bash
# 1. Запустить профилирование
./profile.sh setup

# 2. Создать flamegraph с нагрузкой на /slow
./profile.sh flamegraph 60

# 3. Параллельно запустить нагрузочное тестирование
./profile.sh load-test 60

# 4. Проанализировать результаты
./profile.sh analyze
```

### Что искать в результатах:

1. **Flamegraph (`*.svg`)**:
   - Найти функции, занимающие наибольшую ширину
   - Обратить внимание на функции с глубокой вложенностью
   - Искать функции связанные с `slow_endpoint`

2. **Perf report (`perf_report_*.txt`)**:
   - Функции с наибольшим процентом CPU времени
   - Функции с большим количеством samples
   - Cache misses и branch mispredictions

3. **Memory profile**:
   - Функции с наибольшим количеством page faults
   - Рост RSS памяти
   - Memory leaks

## Типичные проблемы и решения

### Проблема: "Permission denied" для perf
**Решение**: 
```bash
# Убедитесь, что контейнеры запущены с привилегированным режимом
docker-compose -f docker-compose.profiling.yml down
./profile.sh setup
```

### Проблема: Flamegraph не генерируется
**Решение**:
```bash
# Проверьте логи контейнера
docker logs web-forum-app-1

# Перезапустите профилирование
./profile.sh clean
./profile.sh setup
```

### Проблема: Нет нагрузки на /slow
**Решение**:
```bash
# Проверьте доступность эндпоинта
curl http://localhost:8080/slow

# Убедитесь, что эндпоинт разкомментирован в locustfile.py
```

## Продвинутые техники

### Кастомное профилирование
```bash
# Войти в контейнер для ручного профилирования
docker exec -it web-forum-app-1 bash

# Найти PID процесса
pidof backend

# Запустить perf с кастомными параметрами
perf record -F 997 -p <PID> -g -e cycles,cache-misses --call-graph dwarf sleep 30

# Создать flamegraph
perf script | flamegraph > custom_flamegraph.svg
```

### Профилирование конкретных функций
```bash
# Использовать perf с фильтрацией по символам
perf record -p <PID> --call-graph dwarf -e cycles -- sleep 30
perf report --stdio | grep slow_endpoint
```

### Continuous profiling
```bash
# Запустить профилирование в цикле
while true; do
    ./profile.sh flamegraph 60
    sleep 300  # 5 минут между сессиями
done
```

## Файлы проекта

- `Dockerfile.profiling` - Docker образ с инструментами профилирования
- `docker-compose.profiling.yml` - Compose файл с настройками для профилирования
- `profile.sh` - Основной скрипт профилирования
- `profiling/` - Директория с результатами профилирования

## Дополнительные инструменты

### В контейнере доступны:
- `perf` - Основной инструмент профилирования Linux
- `flamegraph` - Генератор flame graphs
- `htop` - Интерактивный monitor процессов
- `strace` - Трассировщик системных вызовов
- `curl` - Для тестирования эндпоинтов

### Rust-специфичные инструменты:
- Символы отладки включены в релизной сборке
- Frame pointers включены для лучшего профилирования
- Cargo flamegraph интегрирован

## Troubleshooting

### Если flamegraph пустой:
1. Убедитесь, что есть нагрузка на приложение
2. Проверьте, что символы отладки включены
3. Увеличьте время профилирования

### Если perf не работает:
1. Проверьте права доступа (должен быть privileged режим)
2. Убедитесь, что kernel поддерживает perf events
3. Проверьте настройки `perf_event_paranoid`

Для получения помощи: `./profile.sh help`
