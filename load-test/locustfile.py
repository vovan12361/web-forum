import json
import uuid
from locust import HttpUser, task, between

class ForumUser(HttpUser):
    wait_time = between(1, 5)
    
    def on_start(self):
        self.board_ids = []
        self.post_ids = []
    
    @task(2)
    def get_boards(self):
        response = self.client.get("/boards")
        if response.status_code == 200:
            boards = response.json()
            if boards:
                self.board_ids = [board["id"] for board in boards]
    
    @task(1)
    def create_board(self):
        board_data = {
            "name": f"Test Board {uuid.uuid4()}",
            "description": "This is a test board created by load testing"
        }
        response = self.client.post("/boards", json=board_data)
        if response.status_code == 201:
            new_board = response.json()
            self.board_ids.append(new_board["id"])
    
    @task(2)
    def get_posts_by_board(self):
        if not self.board_ids:
            self.get_boards()
            return
        
        board_id = self.board_ids[0]
        self.client.get(f"/boards/{board_id}/posts")
    
    @task(1)
    def create_post(self):
        if not self.board_ids:
            self.get_boards()
            return
        
        board_id = self.board_ids[0]
        post_data = {
            "board_id": board_id,
            "title": f"Test Post {uuid.uuid4()}",
            "content": "This is a test post created by load testing",
            "author": "Load Tester"
        }
        response = self.client.post("/posts", json=post_data)
        if response.status_code == 201:
            new_post = response.json()
            self.post_ids.append(new_post["id"])
    
    @task(1)
    def get_post(self):
        if not self.post_ids:
            return
        
        post_id = self.post_ids[0]
        self.client.get(f"/posts/{post_id}")
    
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
        self.client.post("/comments", json=comment_data)
    
    @task(1)
    def get_comments(self):
        if not self.post_ids:
            return
        
        post_id = self.post_ids[0]
        self.client.get(f"/posts/{post_id}/comments")
    
    @task(1)
    def health_check(self):
        self.client.get("/health")
    
    @task(1)
    def trigger_slow_endpoint(self):
        self.client.get("/slow")

class ForumViewerUser(HttpUser):
    """User that only reads content, doesn't create anything"""
    wait_time = between(1, 3)
    
    @task(5)
    def get_boards(self):
        self.client.get("/boards")
    
    @task(3)
    def get_posts(self):
        response = self.client.get("/boards")
        if response.status_code == 200:
            boards = response.json()
            if boards:
                board_id = boards[0]["id"]
                self.client.get(f"/boards/{board_id}/posts")
    
    @task(2)
    def health_check(self):
        self.client.get("/health")
    
    @task(1)
    def metrics_check(self):
        self.client.get("/metrics")
    
    @task(1)
    def trigger_slow_endpoint(self):
        self.client.get("/slow") 