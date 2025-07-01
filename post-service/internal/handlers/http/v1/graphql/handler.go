package graphql

import (
	"encoding/json"
	"log"
	"net/http"

	"github.com/gfdmit/web-forum/post-service/internal/service"
	"github.com/graphql-go/graphql"
)

type gqlHandler struct {
	svc *service.Service

	schema graphql.Schema
}

func New(svc *service.Service) (*gqlHandler, error) {
	gh := &gqlHandler{
		svc: svc,
	}

	if err := gh.initSchema(); err != nil {
		return nil, err
	}

	return gh, nil
}

func (gh *gqlHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	queryJson := make(map[string]interface{})

	err := json.NewDecoder(r.Body).Decode(&queryJson)
	if err != nil {
		log.Println(err)
		w.Write([]byte("1internal server error"))
		return
	}

	queryField, ok := queryJson["query"]
	if !ok {
		log.Println(err)
		w.Write([]byte("2internal server error"))
		return
	}

	queryString, ok := queryField.(string)
	if !ok {
		log.Println(err)
		w.Write([]byte("3internal server error"))
		return
	}

	varField, ok := queryJson["variables"]
	if !ok {
		log.Println(err)
		w.Write([]byte("4internal server error"))
		return
	}
	varQuery, ok := varField.(map[string]interface{})
	if !ok {
		log.Println(err)
		w.Write([]byte("5internal server error"))
		return
	}
	res := graphql.Do(graphql.Params{
		Context:        r.Context(),
		Schema:         gh.schema,
		RequestString:  queryString,
		VariableValues: varQuery,
	})
	json.NewEncoder(w).Encode(res)
}
