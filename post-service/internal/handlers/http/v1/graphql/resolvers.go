package graphql

import (
	"fmt"

	"github.com/graphql-go/graphql"
)

func getBoardQuery(gh *gqlHandler, boardType *graphql.Object) *graphql.Field {
	return &graphql.Field{
		Type: boardType,
		Args: graphql.FieldConfigArgument{
			"id": &graphql.ArgumentConfig{Type: graphql.NewNonNull(graphql.ID)},
		},
		Resolve: func(p graphql.ResolveParams) (interface{}, error) {
			id := p.Args["id"].(string)
			return gh.svc.GetBoard(p.Context, id)
		},
	}
}

func getBoardsQuery(gh *gqlHandler, boardType *graphql.Object) *graphql.Field {
	return &graphql.Field{
		Type: graphql.NewList(boardType),
		Args: graphql.FieldConfigArgument{
			"includeDeleted": &graphql.ArgumentConfig{Type: graphql.Boolean, DefaultValue: false},
		},
		Resolve: func(p graphql.ResolveParams) (interface{}, error) {
			includeDeleted := p.Args["includeDeleted"].(bool)
			return gh.svc.GetBoards(p.Context, includeDeleted)
		},
	}
}

func getPostQuery(gh *gqlHandler, postType *graphql.Object) *graphql.Field {
	return &graphql.Field{
		Type: postType,
		Args: graphql.FieldConfigArgument{
			"id": &graphql.ArgumentConfig{Type: graphql.NewNonNull(graphql.ID)},
		},
		Resolve: func(p graphql.ResolveParams) (interface{}, error) {
			id := p.Args["id"].(string)
			return gh.svc.GetPost(p.Context, id)
		},
	}
}

func getPostsQuery(gh *gqlHandler, postType *graphql.Object) *graphql.Field {
	return &graphql.Field{
		Type: graphql.NewList(postType),
		Args: graphql.FieldConfigArgument{
			"boardId":        &graphql.ArgumentConfig{Type: graphql.NewNonNull(graphql.ID)},
			"includeDeleted": &graphql.ArgumentConfig{Type: graphql.Boolean, DefaultValue: false},
			"limit":          &graphql.ArgumentConfig{Type: graphql.Int, DefaultValue: 100},
			"offset":         &graphql.ArgumentConfig{Type: graphql.Int, DefaultValue: 0},
		},
		Resolve: func(p graphql.ResolveParams) (interface{}, error) {
			fmt.Printf("Args: %+v\n", p.Args) // debug
			return gh.svc.GetPosts(
				p.Context,
				p.Args["boardId"].(string),
				p.Args["includeDeleted"].(bool),
				p.Args["limit"].(int),
				p.Args["offset"].(int),
			)
		},
	}
}

func getCommentQuery(gh *gqlHandler, commentType *graphql.Object) *graphql.Field {
	return &graphql.Field{
		Type: commentType,
		Args: graphql.FieldConfigArgument{
			"id": &graphql.ArgumentConfig{Type: graphql.NewNonNull(graphql.ID)},
		},
		Resolve: func(p graphql.ResolveParams) (interface{}, error) {
			id := p.Args["id"].(string)
			return gh.svc.GetComment(p.Context, id)
		},
	}
}

func getCommentsQuery(gh *gqlHandler, commentType *graphql.Object) *graphql.Field {
	return &graphql.Field{
		Type: graphql.NewList(commentType),
		Args: graphql.FieldConfigArgument{
			"postId":         &graphql.ArgumentConfig{Type: graphql.NewNonNull(graphql.ID)},
			"includeDeleted": &graphql.ArgumentConfig{Type: graphql.Boolean, DefaultValue: false},
			"limit":          &graphql.ArgumentConfig{Type: graphql.Int, DefaultValue: 500},
			"offset":         &graphql.ArgumentConfig{Type: graphql.Int, DefaultValue: 0},
		},
		Resolve: func(p graphql.ResolveParams) (interface{}, error) {
			return gh.svc.GetComments(
				p.Context,
				p.Args["postId"].(string),
				p.Args["includeDeleted"].(bool),
				p.Args["limit"].(int),
				p.Args["offset"].(int),
			)
		},
	}
}

func createBoardMutation(gh *gqlHandler, boardType *graphql.Object) *graphql.Field {
	return &graphql.Field{
		Type: boardType,
		Args: graphql.FieldConfigArgument{
			"input": &graphql.ArgumentConfig{
				Type: graphql.NewInputObject(
					graphql.InputObjectConfig{
						Name: "CreateBoardInput",
						Fields: graphql.InputObjectConfigFieldMap{
							"name":        &graphql.InputObjectFieldConfig{Type: graphql.NewNonNull(graphql.String)},
							"description": &graphql.InputObjectFieldConfig{Type: graphql.String},
						},
					},
				),
			},
		},
		Resolve: func(p graphql.ResolveParams) (interface{}, error) {
			input := p.Args["input"].(map[string]interface{})
			return gh.svc.CreateBoard(
				p.Context,
				input["name"].(string),
				input["description"].(string),
			)
		},
	}
}

func deleteBoardMutation(gh *gqlHandler) *graphql.Field {
	return &graphql.Field{
		Type: graphql.Boolean,
		Args: graphql.FieldConfigArgument{
			"id": &graphql.ArgumentConfig{Type: graphql.NewNonNull(graphql.ID)},
		},
		Resolve: func(p graphql.ResolveParams) (interface{}, error) {
			id := p.Args["id"].(string)
			return gh.svc.DeleteBoard(p.Context, id)
		},
	}
}

func restoreBoardMutation(gh *gqlHandler) *graphql.Field {
	return &graphql.Field{
		Type: graphql.Boolean,
		Args: graphql.FieldConfigArgument{
			"id": &graphql.ArgumentConfig{Type: graphql.NewNonNull(graphql.ID)},
		},
		Resolve: func(p graphql.ResolveParams) (interface{}, error) {
			id := p.Args["id"].(string)
			return gh.svc.RestoreBoard(p.Context, id)
		},
	}
}

func createPostMutation(gh *gqlHandler, postType *graphql.Object) *graphql.Field {
	return &graphql.Field{
		Type: postType,
		Args: graphql.FieldConfigArgument{
			"input": &graphql.ArgumentConfig{
				Type: graphql.NewInputObject(
					graphql.InputObjectConfig{
						Name: "CreatePostInput",
						Fields: graphql.InputObjectConfigFieldMap{
							"boardId": &graphql.InputObjectFieldConfig{Type: graphql.NewNonNull(graphql.ID)},
							"title":   &graphql.InputObjectFieldConfig{Type: graphql.String},
							"text":    &graphql.InputObjectFieldConfig{Type: graphql.NewNonNull(graphql.String)},
							"hashIp":  &graphql.InputObjectFieldConfig{Type: graphql.String},
						},
					},
				),
			},
		},
		Resolve: func(p graphql.ResolveParams) (interface{}, error) {
			input := p.Args["input"].(map[string]interface{})
			fmt.Printf("Args: %+v\n", p.Args) // debug
			return gh.svc.CreatePost(
				p.Context,
				input["boardId"].(string),
				input["title"].(string),
				input["text"].(string),
				input["hashIp"].(string),
			)
		},
	}
}

func deletePostMutation(gh *gqlHandler) *graphql.Field {
	return &graphql.Field{
		Type: graphql.Boolean,
		Args: graphql.FieldConfigArgument{
			"id": &graphql.ArgumentConfig{Type: graphql.NewNonNull(graphql.ID)},
		},
		Resolve: func(p graphql.ResolveParams) (interface{}, error) {
			id := p.Args["id"].(string)
			return gh.svc.DeletePost(p.Context, id)
		},
	}
}

func createCommentMutation(gh *gqlHandler, commentType *graphql.Object) *graphql.Field {
	return &graphql.Field{
		Type: commentType,
		Args: graphql.FieldConfigArgument{
			"input": &graphql.ArgumentConfig{
				Type: graphql.NewInputObject(
					graphql.InputObjectConfig{
						Name: "CreateCommentInput",
						Fields: graphql.InputObjectConfigFieldMap{
							"postId": &graphql.InputObjectFieldConfig{Type: graphql.NewNonNull(graphql.ID)},
							"text":   &graphql.InputObjectFieldConfig{Type: graphql.NewNonNull(graphql.String)},
							"hashIp": &graphql.InputObjectFieldConfig{Type: graphql.String},
						},
					},
				),
			},
		},
		Resolve: func(p graphql.ResolveParams) (interface{}, error) {
			input := p.Args["input"].(map[string]interface{})
			return gh.svc.CreateComment(
				p.Context,
				input["postId"].(string),
				input["text"].(string),
				input["hashIp"].(string),
			)
		},
	}
}

func deleteCommentMutation(gh *gqlHandler) *graphql.Field {
	return &graphql.Field{
		Type: graphql.Boolean,
		Args: graphql.FieldConfigArgument{
			"id": &graphql.ArgumentConfig{Type: graphql.NewNonNull(graphql.ID)},
		},
		Resolve: func(p graphql.ResolveParams) (interface{}, error) {
			id := p.Args["id"].(string)
			return gh.svc.DeleteComment(p.Context, id)
		},
	}
}
