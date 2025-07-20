#!/usr/bin/env python3
"""
Complete monitoring system health check
Validates all aspects of the observability stack
"""

import subprocess
import sys
import time
from typing import List, Tuple

def run_check(script_name: str, description: str) -> Tuple[bool, str]:
    """Run a monitoring check script and return success status"""
    try:
        print(f"\n{'='*60}")
        print(f"🔍 {description}")
        print(f"{'='*60}")
        
        result = subprocess.run([
            "python3", script_name
        ], capture_output=False, text=True, cwd="/app/tools" if "/app" in sys.executable else ".")
        
        success = result.returncode == 0
        return success, f"Exit code: {result.returncode}"
        
    except Exception as e:
        return False, f"Error running {script_name}: {e}"

def check_docker_services() -> List[Tuple[str, bool]]:
    """Check that all required Docker services are running"""
    required_services = [
        "app", "scylladb", "prometheus", "grafana", 
        "jaeger", "loki", "promtail", "alertmanager"
    ]
    
    try:
        result = subprocess.run([
            "docker-compose", "ps", "--services", "--filter", "status=running"
        ], capture_output=True, text=True)
        
        if result.returncode != 0:
            return [(service, False) for service in required_services]
            
        running_services = set(result.stdout.strip().split('\n'))
        return [(service, service in running_services) for service in required_services]
        
    except:
        return [(service, False) for service in required_services]

def main():
    print("🚀 Complete Monitoring System Health Check")
    print("=" * 60)
    
    # Check Docker services first
    print("\n🐳 Docker Services Status:")
    services_status = check_docker_services()
    all_services_up = True
    
    for service, is_running in services_status:
        status_icon = "✅" if is_running else "❌"
        print(f"  {status_icon} {service}")
        if not is_running:
            all_services_up = False
    
    if not all_services_up:
        print("\n⚠️  Some services are not running. Please start them with:")
        print("   docker-compose up -d")
        print("\nWaiting for services to be ready...")
        time.sleep(30)  # Give services time to start
    
    # Run all monitoring checks
    checks = [
        ("check_metrics.py", "Prometheus Metrics Collection"),
        ("check_logs.py", "Loki Log Aggregation"),
        ("check_tracing.py", "Jaeger Distributed Tracing"),
    ]
    
    results = []
    for script, description in checks:
        success, message = run_check(script, description)
        results.append((description, success, message))
    
    # Summary
    print(f"\n{'='*60}")
    print("📊 MONITORING SYSTEM HEALTH SUMMARY")
    print(f"{'='*60}")
    
    print("\n🐳 Infrastructure Services:")
    for service, is_running in services_status:
        status_icon = "✅" if is_running else "❌"
        print(f"  {status_icon} {service}")
    
    print("\n📈 Observability Components:")
    all_passed = True
    for description, success, message in results:
        status_icon = "✅" if success else "❌"
        print(f"  {status_icon} {description}")
        if not success:
            print(f"      {message}")
            all_passed = False
    
    print(f"\n{'='*60}")
    if all_passed and all_services_up:
        print("🎉 ALL SYSTEMS OPERATIONAL")
        print("Your complete observability stack is working correctly!")
        print("\nQuick Access URLs:")
        print("  📊 Prometheus: http://localhost:9090")
        print("  📈 Grafana: http://localhost:3000 (admin/admin)")
        print("  🔍 Jaeger: http://localhost:16686")
        print("  🔧 Forum API: http://localhost:8080")
        print("  📚 API Docs: http://localhost:8080/docs")
        print("  🧪 Load Testing: http://localhost:8089")
    else:
        print("⚠️  ISSUES DETECTED")
        print("Please check the detailed output above and resolve any issues.")
        sys.exit(1)

if __name__ == "__main__":
    main()
