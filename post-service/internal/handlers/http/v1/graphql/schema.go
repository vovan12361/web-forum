package graphql

import (
	"time"

	"github.com/graphql-go/graphql"
)

var DateTime = graphql.NewScalar(
	graphql.ScalarConfig{
		Name:        "DateTime",
		Description: "DateTime scalar type",
		Serialize: func(value interface{}) interface{} {
			switch v := value.(type) {
			case time.Time:
				return v.Format(time.RFC3339)
			case *time.Time:
				return v.Format(time.RFC3339)
			default:
				return nil
			}
		},
	},
)

func (gh *gqlHandler) initSchema() error {
	boardType := graphql.NewObject(
		graphql.ObjectConfig{
			Name: "Board",
			Fields: graphql.Fields{
				"id":          &graphql.Field{Type: graphql.ID},
				"name":        &graphql.Field{Type: graphql.String},
				"description": &graphql.Field{Type: graphql.String},
				"createdAt":   &graphql.Field{Type: DateTime},
				"deletedAt":   &graphql.Field{Type: DateTime},
			},
		},
	)

	postType := graphql.NewObject(
		graphql.ObjectConfig{
			Name: "Post",
			Fields: graphql.Fields{
				"id":        &graphql.Field{Type: graphql.ID},
				"boardId":   &graphql.Field{Type: graphql.ID},
				"title":     &graphql.Field{Type: graphql.String},
				"text":      &graphql.Field{Type: graphql.String},
				"hashIp":    &graphql.Field{Type: graphql.String},
				"createdAt": &graphql.Field{Type: DateTime},
				"deletedAt": &graphql.Field{Type: DateTime},
			},
		},
	)

	commentType := graphql.NewObject(
		graphql.ObjectConfig{
			Name: "Comment",
			Fields: graphql.Fields{
				"id":        &graphql.Field{Type: graphql.ID},
				"postId":    &graphql.Field{Type: graphql.ID},
				"text":      &graphql.Field{Type: graphql.String},
				"hashIp":    &graphql.Field{Type: graphql.String},
				"createdAt": &graphql.Field{Type: DateTime},
				"deletedAt": &graphql.Field{Type: DateTime},
			},
		},
	)

	queryType := graphql.NewObject(
		graphql.ObjectConfig{
			Name: "Query",
			Fields: graphql.Fields{
				"board":    getBoardQuery(gh, boardType),
				"boards":   getBoardsQuery(gh, boardType),
				"post":     getPostQuery(gh, postType),
				"posts":    getPostsQuery(gh, postType),
				"comment":  getCommentQuery(gh, commentType),
				"comments": getCommentsQuery(gh, commentType),
			},
		},
	)

	mutationType := graphql.NewObject(
		graphql.ObjectConfig{
			Name: "Mutation",
			Fields: graphql.Fields{
				"createBoard":   createBoardMutation(gh, boardType),
				"deleteBoard":   deleteBoardMutation(gh),
				"restoreBoard":  restoreBoardMutation(gh),
				"createPost":    createPostMutation(gh, postType),
				"deletePost":    deletePostMutation(gh),
				"createComment": createCommentMutation(gh, commentType),
				"deleteComment": deleteCommentMutation(gh),
			},
		},
	)

	schemaConfig := graphql.SchemaConfig{
		Query:    queryType,
		Mutation: mutationType,
	}

	schema, err := graphql.NewSchema(schemaConfig)
	if err != nil {
		return err
	}
	gh.schema = schema

	return nil
}
