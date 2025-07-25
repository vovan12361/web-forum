import json
import uuid
import os
import time
from locust import HttpUser, task, between
from requests.exceptions import RequestException, ConnectionError, Timeout

# OpenTelemetry imports for distributed tracing
from opentelemetry import trace, propagate
from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import OTLPSpanExporter
from opentelemetry.instrumentation.requests import RequestsInstrumentor
from opentelemetry.instrumentation.urllib3 import URLLib3Instrumentor
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor
from opentelemetry.sdk.resources import Resource

# Configure OpenTelemetry
def configure_tracing():
    """Configure OpenTelemetry for Locust load testing"""
    
    # Set up resource with service information
    resource = Resource.create({
        "service.name": "locust-load-test",
        "service.version": "1.0.0",
        "deployment.environment": "test"
    })
    
    # Set up tracer provider
    trace.set_tracer_provider(TracerProvider(resource=resource))
    tracer_provider = trace.get_tracer_provider()
    
    # Configure OTLP exporter for Jaeger
    # Jaeger OTLP gRPC receiver runs on port 4317
    jaeger_endpoint = os.getenv('JAEGER_OTLP_ENDPOINT', 'jaeger:4317')
    
    try:
        otlp_exporter = OTLPSpanExporter(
            endpoint=jaeger_endpoint,
            insecure=True,
            headers={}
        )
        
        # Add span processor
        span_processor = BatchSpanProcessor(otlp_exporter)
        tracer_provider.add_span_processor(span_processor)
        
        print(f"OpenTelemetry configured with Jaeger OTLP endpoint: {jaeger_endpoint}")
    except Exception as e:
        print(f"Warning: Failed to configure OTLP exporter: {e}")
        print("Continuing without distributed tracing...")
    
    # Instrument HTTP libraries for automatic tracing
    try:
        RequestsInstrumentor().instrument()
        URLLib3Instrumentor().instrument()
        print("HTTP instrumentation enabled")
    except Exception as e:
        print(f"Warning: Failed to instrument HTTP libraries: {e}")
    
    # Set up tracer provider
    trace.set_tracer_provider(tracer_provider)

# Initialize tracing when module is loaded
configure_tracing()

# Get tracer for creating custom spans
tracer = trace.get_tracer(__name__)

class ForumUser(HttpUser):
    # Set the host from environment variable or use default
    host = os.getenv('TARGET_HOST', 'http://localhost:8080')
    wait_time = between(1, 5)
    
    # Add connection settings to reduce timeouts and improve reliability
    connection_timeout = 10.0
    network_timeout = 30.0
    
    def on_start(self):
        self.board_ids = []
        self.post_ids = []
        # Configure session for better reliability
        self.client.timeout = self.network_timeout
        # Limit the number of stored IDs to prevent memory issues
        self.max_stored_ids = 100
        
        # Add trace headers for distributed tracing
        self.trace_headers = {
            'User-Agent': 'Locust-Load-Test/1.0',
            'X-Load-Test': 'true',
            'Content-Type': 'application/json',
            'Accept': 'application/json'
        }
    
    def safe_request(self, method, url, task_name=None, **kwargs):
        """Make a safe HTTP request with error handling, retries, and distributed tracing"""
        max_retries = 3
        retry_delay = 0.5
        
        # Inject trace context into headers for distributed tracing
        headers = kwargs.get('headers', {})
        headers.update(self.trace_headers)
        
        # Inject the current span context into the headers
        propagate.inject(headers)
        
        # Debug: log which trace headers are being sent
        trace_headers = {k: v for k, v in headers.items() if k.lower().startswith(('traceparent', 'tracestate', 'baggage'))}
        if trace_headers:
            print(f"Sending trace headers: {trace_headers}")
        
        kwargs['headers'] = headers
        
        for attempt in range(max_retries):
            try:
                if method.upper() == 'GET':
                    response = self.client.get(url, timeout=(self.connection_timeout, self.network_timeout), **kwargs)
                elif method.upper() == 'POST':
                    response = self.client.post(url, timeout=(self.connection_timeout, self.network_timeout), **kwargs)
                else:
                    response = self.client.request(method, url, timeout=(self.connection_timeout, self.network_timeout), **kwargs)
                
                return response
                    
            except (ConnectionError, Timeout, RequestException) as e:
                if attempt == max_retries - 1:
                    # Log error but don't crash
                    print(f"Request failed after {max_retries} attempts: {e}")
                    return None
                time.sleep(retry_delay * (attempt + 1))
                
        return None
    
    @task(2)
    def get_boards(self):
        with tracer.start_as_current_span("load_test.get_boards") as span:
            span.set_attribute("load_test.task", "get_boards")
            span.set_attribute("http.method", "GET")
            span.set_attribute("http.url", f"{self.host}/boards")
            span.set_attribute("load_test.user", "locust")
            span.set_attribute("load_test.scenario", self.__class__.__name__)
            
            response = self.safe_request("GET", "/boards", task_name="get_boards")
            
            if response is not None:
                span.set_attribute("http.status_code", response.status_code)
                span.set_attribute("http.response.size", len(response.content) if response.content else 0)
                
                if response.status_code == 200:
                    try:
                        result = response.json()
                        # Handle paginated response format
                        if isinstance(result, dict) and "data" in result:
                            boards = result["data"]
                            self.board_ids = [board["id"] for board in boards if "id" in board]
                            span.set_attribute("boards.count", len(boards))
                        elif isinstance(result, list):
                            # Backwards compatibility for non-paginated response
                            self.board_ids = [board["id"] for board in result if "id" in board]
                            span.set_attribute("boards.count", len(result))
                    except (ValueError, KeyError) as e:
                        span.set_attribute("error", True)
                        span.set_attribute("error.message", f"Error parsing boards response: {e}")
                        print(f"Error parsing boards response: {e}")
                else:
                    span.set_attribute("error", True)
                    span.set_attribute("error.message", f"HTTP {response.status_code}")
            else:
                span.set_attribute("error", True)
                span.set_attribute("error.message", "Request failed")

    @task(1)
    def get_boards_with_pagination(self):
        """Test pagination functionality for boards"""
        with tracer.start_as_current_span("load_test.get_boards_paginated") as span:
            span.set_attribute("load_test.task", "get_boards_with_pagination")
            span.set_attribute("http.method", "GET")
            span.set_attribute("http.url", f"{self.host}/boards?page_size=5")
            span.set_attribute("load_test.user", "locust")
            span.set_attribute("load_test.scenario", self.__class__.__name__)
            
            response = self.safe_request("GET", "/boards?page_size=5", task_name="get_boards_paginated")
            
            if response is not None:
                span.set_attribute("http.status_code", response.status_code)
                
                if response.status_code == 200:
                    try:
                        result = response.json()
                        if isinstance(result, dict) and "data" in result:
                            boards = result["data"]
                            self.board_ids.extend([board["id"] for board in boards if "id" in board])
                            span.set_attribute("boards.count", len(boards))
                            span.set_attribute("pagination.has_more", result.get("has_more", False))
                            # Test next page if available
                            if result.get("has_more") and result.get("next_page_state"):
                                next_url = f"/boards?page_size=5&page_state={result['next_page_state']}"
                                self.safe_request("GET", next_url, task_name="get_boards_next_page")
                    except (ValueError, KeyError) as e:
                        span.set_attribute("error", True)
                        span.set_attribute("error.message", f"Error parsing paginated boards response: {e}")
                        print(f"Error parsing paginated boards response: {e}")
                else:
                    span.set_attribute("error", True)
                    span.set_attribute("error.message", f"HTTP {response.status_code}")
            else:
                span.set_attribute("error", True)
                span.set_attribute("error.message", "Request failed")
    
    @task(1)
    def create_board(self):
        with tracer.start_as_current_span("load_test.create_board") as span:
            span.set_attribute("load_test.task", "create_board")
            span.set_attribute("http.method", "POST")
            span.set_attribute("http.url", f"{self.host}/boards")
            span.set_attribute("load_test.user", "locust")
            span.set_attribute("load_test.scenario", self.__class__.__name__)
            
            board_name = f"Test Board {uuid.uuid4()}"
            board_data = {
                "name": board_name,
                "description": "This is a test board created by load testing"
            }
            span.set_attribute("board.name", board_name)
            
            response = self.safe_request("POST", "/boards", task_name="create_board", json=board_data)
            
            if response is not None:
                span.set_attribute("http.status_code", response.status_code)
                
                if response.status_code == 201:
                    try:
                        new_board = response.json()
                        if "id" in new_board:
                            self.board_ids.append(new_board["id"])
                            span.set_attribute("board.id", new_board["id"])
                            # Keep only recent IDs to prevent memory issues
                            if len(self.board_ids) > self.max_stored_ids:
                                self.board_ids = self.board_ids[-self.max_stored_ids:]
                    except (ValueError, KeyError) as e:
                        span.set_attribute("error", True)
                        span.set_attribute("error.message", f"Error parsing create board response: {e}")
                        print(f"Error parsing create board response: {e}")
                else:
                    span.set_attribute("error", True)
                    span.set_attribute("error.message", f"HTTP {response.status_code}")
            else:
                span.set_attribute("error", True)
                span.set_attribute("error.message", "Request failed")
    
    @task(2)
    def get_posts_by_board(self):
        if not self.board_ids:
            self.get_boards()
            if not self.board_ids:
                return
        
        board_id = self.board_ids[0]
        with tracer.start_as_current_span("load_test.get_posts_by_board") as span:
            span.set_attribute("load_test.task", "get_posts_by_board")
            span.set_attribute("board.id", board_id)
            span.set_attribute("http.method", "GET")
            span.set_attribute("http.url", f"{self.host}/boards/{board_id}/posts")
            
            response = self.safe_request("GET", f"/boards/{board_id}/posts")
            if response is not None:
                span.set_attribute("http.status_code", response.status_code)
                if response.status_code != 200:
                    span.set_attribute("error", True)
                    span.set_attribute("error.message", f"HTTP {response.status_code}")
            else:
                span.set_attribute("error", True)
                span.set_attribute("error.message", "Request failed")

    @task(1)
    def get_posts_by_board_with_pagination(self):
        """Test pagination functionality for posts"""
        if not self.board_ids:
            self.get_boards()
            if not self.board_ids:
                return
        
        board_id = self.board_ids[0]
        with tracer.start_as_current_span("load_test.get_posts_paginated") as span:
            span.set_attribute("load_test.task", "get_posts_by_board_with_pagination")
            span.set_attribute("board.id", board_id)
            span.set_attribute("http.method", "GET")
            span.set_attribute("http.url", f"{self.host}/boards/{board_id}/posts?page_size=3")
            
            response = self.safe_request("GET", f"/boards/{board_id}/posts?page_size=3")
            if response is not None and response.status_code == 200:
                try:
                    result = response.json()
                    if isinstance(result, dict) and "data" in result:
                        posts = result["data"]
                        self.post_ids.extend([post["id"] for post in posts if "id" in post])
                        span.set_attribute("posts.count", len(posts))
                        # Test next page if available
                        if result.get("has_more") and result.get("next_page_state"):
                            next_url = f"/boards/{board_id}/posts?page_size=3&page_state={result['next_page_state']}"
                            self.safe_request("GET", next_url)
                except (ValueError, KeyError) as e:
                    span.set_attribute("error", True)
                    span.set_attribute("error.message", f"Error parsing paginated posts response: {e}")
                    print(f"Error parsing paginated posts response: {e}")
    
    @task(1)
    def create_post(self):
        if not self.board_ids:
            self.get_boards()
            if not self.board_ids:
                return
        
        board_id = self.board_ids[0]
        with tracer.start_as_current_span("load_test.create_post") as span:
            span.set_attribute("load_test.task", "create_post")
            span.set_attribute("board.id", board_id)
            span.set_attribute("http.method", "POST")
            span.set_attribute("http.url", f"{self.host}/posts")
            
            post_data = {
                "board_id": board_id,
                "title": f"Test Post {uuid.uuid4()}",
                "content": "This is a test post created by load testing",
                "author": "Load Tester"
            }
            response = self.safe_request("POST", "/posts", json=post_data)
            
            if response is not None:
                span.set_attribute("http.status_code", response.status_code)
                
                if response.status_code == 201:
                    try:
                        new_post = response.json()
                        if "id" in new_post:
                            self.post_ids.append(new_post["id"])
                            span.set_attribute("post.id", new_post["id"])
                            # Keep only recent IDs to prevent memory issues
                            if len(self.post_ids) > self.max_stored_ids:
                                self.post_ids = self.post_ids[-self.max_stored_ids:]
                    except (ValueError, KeyError) as e:
                        span.set_attribute("error", True)
                        span.set_attribute("error.message", f"Error parsing create post response: {e}")
                        print(f"Error parsing create post response: {e}")
                else:
                    span.set_attribute("error", True)
                    span.set_attribute("error.message", f"HTTP {response.status_code}")
            else:
                span.set_attribute("error", True)
                span.set_attribute("error.message", "Request failed")
    
    @task(1)
    def get_post(self):
        if not self.post_ids:
            return
        
        post_id = self.post_ids[0]
        with tracer.start_as_current_span("load_test.get_post") as span:
            span.set_attribute("load_test.task", "get_post")
            span.set_attribute("post.id", post_id)
            span.set_attribute("http.method", "GET")
            span.set_attribute("http.url", f"{self.host}/posts/{post_id}")
            
            response = self.safe_request("GET", f"/posts/{post_id}")
            if response is not None:
                span.set_attribute("http.status_code", response.status_code)
                if response.status_code != 200:
                    span.set_attribute("error", True)
                    span.set_attribute("error.message", f"HTTP {response.status_code}")
            else:
                span.set_attribute("error", True)
                span.set_attribute("error.message", "Request failed")
    
    @task(1)
    def create_comment(self):
        if not self.post_ids:
            return
        
        post_id = self.post_ids[0]
        with tracer.start_as_current_span("load_test.create_comment") as span:
            span.set_attribute("load_test.task", "create_comment")
            span.set_attribute("post.id", post_id)
            span.set_attribute("http.method", "POST")
            span.set_attribute("http.url", f"{self.host}/comments")
            
            comment_data = {
                "post_id": post_id,
                "content": f"Test comment {uuid.uuid4()}",
                "author": "Load Tester"
            }
            response = self.safe_request("POST", "/comments", json=comment_data)
            if response is not None:
                span.set_attribute("http.status_code", response.status_code)
                if response.status_code != 201:
                    span.set_attribute("error", True)
                    span.set_attribute("error.message", f"HTTP {response.status_code}")
            else:
                span.set_attribute("error", True)
                span.set_attribute("error.message", "Request failed")
    
    @task(1)
    def get_comments(self):
        if not self.post_ids:
            return
        
        post_id = self.post_ids[0]
        with tracer.start_as_current_span("load_test.get_comments") as span:
            span.set_attribute("load_test.task", "get_comments")
            span.set_attribute("post.id", post_id)
            span.set_attribute("http.method", "GET")
            span.set_attribute("http.url", f"{self.host}/posts/{post_id}/comments")
            
            response = self.safe_request("GET", f"/posts/{post_id}/comments")
            if response is not None:
                span.set_attribute("http.status_code", response.status_code)
                if response.status_code != 200:
                    span.set_attribute("error", True)
                    span.set_attribute("error.message", f"HTTP {response.status_code}")
            else:
                span.set_attribute("error", True)
                span.set_attribute("error.message", "Request failed")

    @task(1)
    def get_comments_with_pagination(self):
        """Test pagination functionality for comments"""
        if not self.post_ids:
            return
        
        post_id = self.post_ids[0]
        with tracer.start_as_current_span("load_test.get_comments_paginated") as span:
            span.set_attribute("load_test.task", "get_comments_with_pagination")
            span.set_attribute("post.id", post_id)
            span.set_attribute("http.method", "GET")
            span.set_attribute("http.url", f"{self.host}/posts/{post_id}/comments?page_size=5")
            
            response = self.safe_request("GET", f"/posts/{post_id}/comments?page_size=5")
            if response is not None and response.status_code == 200:
                try:
                    result = response.json()
                    if isinstance(result, dict) and "data" in result:
                        # Test next page if available
                        if result.get("has_more") and result.get("next_page_state"):
                            next_url = f"/posts/{post_id}/comments?page_size=5&page_state={result['next_page_state']}"
                            self.safe_request("GET", next_url)
                except (ValueError, KeyError) as e:
                    span.set_attribute("error", True)
                    span.set_attribute("error.message", f"Error parsing paginated comments response: {e}")
                    print(f"Error parsing paginated comments response: {e}")
    
    @task(1)
    def health_check(self):
        with tracer.start_as_current_span("load_test.health_check") as span:
            span.set_attribute("load_test.task", "health_check")
            span.set_attribute("http.method", "GET")
            span.set_attribute("http.url", f"{self.host}/health")
            
            response = self.safe_request("GET", "/health")
            if response is not None:
                span.set_attribute("http.status_code", response.status_code)
                if response.status_code != 200:
                    span.set_attribute("error", True)
                    span.set_attribute("error.message", f"HTTP {response.status_code}")
            else:
                span.set_attribute("error", True)
                span.set_attribute("error.message", "Request failed")
    
    # @task(1)
    # def trigger_slow_endpoint(self):
    #     self.client.get("/slow")

class ForumViewerUser(HttpUser):
    """User that only reads content, doesn't create anything"""
    host = os.getenv('TARGET_HOST', 'http://localhost:8080')
    wait_time = between(1, 3)
    
    # Add connection settings to reduce timeouts and improve reliability
    connection_timeout = 10.0
    network_timeout = 30.0
    
    def on_start(self):
        self.board_ids = []
        # Configure session for better reliability
        self.client.timeout = self.network_timeout
        # Limit the number of stored IDs to prevent memory issues
        self.max_stored_ids = 100
    
    def safe_request(self, method, url, **kwargs):
        """Make a safe HTTP request with error handling and retries"""
        max_retries = 3
        retry_delay = 0.5
        
        for attempt in range(max_retries):
            try:
                if method.upper() == 'GET':
                    response = self.client.get(url, timeout=(self.connection_timeout, self.network_timeout), **kwargs)
                elif method.upper() == 'POST':
                    response = self.client.post(url, timeout=(self.connection_timeout, self.network_timeout), **kwargs)
                else:
                    response = self.client.request(method, url, timeout=(self.connection_timeout, self.network_timeout), **kwargs)
                
                # Check if response is valid
                if response is not None:
                    return response
                    
            except (ConnectionError, Timeout, RequestException) as e:
                if attempt == max_retries - 1:
                    # Log error but don't crash
                    print(f"Request failed after {max_retries} attempts: {e}")
                    return None
                time.sleep(retry_delay * (attempt + 1))
                
        return None
    
    @task(5)
    def get_boards(self):
        response = self.safe_request("GET", "/boards")
        if response is not None and response.status_code == 200:
            try:
                result = response.json()
                # Handle paginated response format
                if isinstance(result, dict) and "data" in result:
                    boards = result["data"]
                    self.board_ids = [board["id"] for board in boards if "id" in board]
                elif isinstance(result, list):
                    # Backwards compatibility for non-paginated response
                    self.board_ids = [board["id"] for board in result if "id" in board]
            except (ValueError, KeyError) as e:
                print(f"Error parsing boards response: {e}")

    @task(2)
    def get_boards_paginated(self):
        """Test pagination for read-only user"""
        self.safe_request("GET", "/boards?page_size=10")
    
    @task(3)
    def get_posts(self):
        if not self.board_ids:
            self.get_boards()
            if not self.board_ids:
                return
                
        board_id = self.board_ids[0]
        self.safe_request("GET", f"/boards/{board_id}/posts")

    @task(2)
    def get_posts_paginated(self):
        """Test pagination for posts"""
        if not self.board_ids:
            self.get_boards()
            if not self.board_ids:
                return
                
        board_id = self.board_ids[0]
        self.safe_request("GET", f"/boards/{board_id}/posts?page_size=5")
    
    @task(2)
    def health_check(self):
        self.safe_request("GET", "/health")
    
    @task(1)
    def metrics_check(self):
        self.safe_request("GET", "/metrics")
    
    # @task(1)
    # def trigger_slow_endpoint(self):
    #     self.client.get("/slow") 