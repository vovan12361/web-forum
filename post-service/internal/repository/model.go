package repository

import "time"

type Board struct {
	ID          int        `json:"id"`
	Name        string     `json:"name"`
	Description *string    `json:"description,omitempty"`
	CreatedAt   time.Time  `json:"created_at"`
	DeletedAt   *time.Time `json:"deleted_at,omitempty"`
}

type Post struct {
	ID        int        `json:"id"`
	BoardID   int        `json:"board_id"`
	Title     *string    `json:"title,omitempty"`
	Text      string     `json:"text"`
	HashIP    *string    `json:"hash_ip,omitempty"`
	CreatedAt time.Time  `json:"created_at"`
	DeletedAt *time.Time `json:"deleted_at,omitempty"`
}

type Comment struct {
	ID        int        `json:"id"`
	PostID    int        `json:"post_id"`
	Text      string     `json:"text"`
	HashIP    *string    `json:"hash_ip,omitempty"`
	CreatedAt time.Time  `json:"created_at"`
	DeletedAt *time.Time `json:"deleted_at,omitempty"`
}
