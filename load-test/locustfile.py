import json
import uuid
import os
import time
from locust import HttpUser, task, between
from requests.exceptions import RequestException, ConnectionError, Timeout

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
    
    @task(2)
    def get_boards(self):
        response = self.safe_request("GET", "/boards")
        if response is not None and response.status_code == 200:
            try:
                boards = response.json()
                if boards and isinstance(boards, list):
                    self.board_ids = [board["id"] for board in boards if "id" in board]
            except (ValueError, KeyError) as e:
                print(f"Error parsing boards response: {e}")
    
    @task(1)
    def create_board(self):
        board_data = {
            "name": f"Test Board {uuid.uuid4()}",
            "description": "This is a test board created by load testing"
        }
        response = self.safe_request("POST", "/boards", json=board_data)
        if response is not None and response.status_code == 201:
            try:
                new_board = response.json()
                if "id" in new_board:
                    self.board_ids.append(new_board["id"])
                    # Keep only recent IDs to prevent memory issues
                    if len(self.board_ids) > self.max_stored_ids:
                        self.board_ids = self.board_ids[-self.max_stored_ids:]
            except (ValueError, KeyError) as e:
                print(f"Error parsing create board response: {e}")
    
    @task(2)
    def get_posts_by_board(self):
        if not self.board_ids:
            self.get_boards()
            if not self.board_ids:
                return
        
        board_id = self.board_ids[0]
        self.safe_request("GET", f"/boards/{board_id}/posts")
    
    @task(1)
    def create_post(self):
        if not self.board_ids:
            self.get_boards()
            if not self.board_ids:
                return
        
        board_id = self.board_ids[0]
        post_data = {
            "board_id": board_id,
            "title": f"Test Post {uuid.uuid4()}",
            "content": "This is a test post created by load testing",
            "author": "Load Tester"
        }
        response = self.safe_request("POST", "/posts", json=post_data)
        if response is not None and response.status_code == 201:
            try:
                new_post = response.json()
                if "id" in new_post:
                    self.post_ids.append(new_post["id"])
                    # Keep only recent IDs to prevent memory issues
                    if len(self.post_ids) > self.max_stored_ids:
                        self.post_ids = self.post_ids[-self.max_stored_ids:]
            except (ValueError, KeyError) as e:
                print(f"Error parsing create post response: {e}")
    
    @task(1)
    def get_post(self):
        if not self.post_ids:
            return
        
        post_id = self.post_ids[0]
        self.safe_request("GET", f"/posts/{post_id}")
    
    @task(1)
    def create_comment(self):
        if not self.post_ids:
            return
        
        post_id = self.post_ids[0]
        comment_data = {
            "post_id": post_id,
            "content": f"Test comment {uuid.uuid4()}",
            "author": "Load Tester"
        }
        self.safe_request("POST", "/comments", json=comment_data)
    
    @task(1)
    def get_comments(self):
        if not self.post_ids:
            return
        
        post_id = self.post_ids[0]
        self.safe_request("GET", f"/posts/{post_id}/comments")
    
    @task(1)
    def health_check(self):
        self.safe_request("GET", "/health")
    
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
                boards = response.json()
                if boards and isinstance(boards, list):
                    self.board_ids = [board["id"] for board in boards if "id" in board]
            except (ValueError, KeyError) as e:
                print(f"Error parsing boards response: {e}")
    
    @task(3)
    def get_posts(self):
        if not self.board_ids:
            self.get_boards()
            if not self.board_ids:
                return
                
        board_id = self.board_ids[0]
        self.safe_request("GET", f"/boards/{board_id}/posts")
    
    @task(2)
    def health_check(self):
        self.safe_request("GET", "/health")
    
    @task(1)
    def metrics_check(self):
        self.safe_request("GET", "/metrics")
    
    # @task(1)
    # def trigger_slow_endpoint(self):
    #     self.client.get("/slow") 