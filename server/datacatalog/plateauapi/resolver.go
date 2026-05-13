//go:generate go run github.com/99designs/gqlgen generate --config gqlgen.yml

package plateauapi

import (
	"errors"

	"github.com/99designs/gqlgen/graphql"
	"github.com/99designs/gqlgen/graphql/handler"
	"github.com/99designs/gqlgen/graphql/handler/extension"
	"github.com/99designs/gqlgen/graphql/handler/lru"
	"github.com/99designs/gqlgen/graphql/handler/transport"
	"github.com/vektah/gqlparser/v2/ast"
)

// Example

// func main() {
// 	port := os.Getenv("PORT")
// 	if port == "" {
// 		port = "8080"
// 	}

// 	srv := plateauapi.NewSchema()

// 	http.Handle("/", playground.Handler("GraphQL playground", "/query"))
// 	http.Handle("/query", srv)

// 	log.Printf("connect to http://localhost:%s/ for GraphQL playground", port)
// 	log.Fatal(http.ListenAndServe(":"+port, nil))
// }

// This file will not be regenerated automatically.
//
// It serves as dependency injection for your app, add any dependencies you require here.

var ErrDatacatalogUnavailable = errors.New("datacatalog is currently unavailable")

type Repo interface {
	QueryResolver
	Name() string
	// Revision returns an opaque token that changes whenever the repository's
	// underlying data changes. Stable across requests that observe the same
	// data, so callers can derive cheap HTTP ETags from it instead of
	// hashing the full response body.
	Revision() string
}

type Resolver struct {
	Repo Repo
	// Host is the externally reachable origin (scheme + host) of the API,
	// used to build absolute composite/latest URLs. Empty disables those
	// fields (they resolve to null).
	Host string
}

type Option func(*handler.Server)

func NewService(repo Repo, host string, opts ...Option) *handler.Server {
	srv := handler.New(NewSchema(repo, host))

	srv.AddTransport(transport.Options{})
	srv.AddTransport(transport.GET{})
	srv.AddTransport(transport.POST{})

	srv.SetQueryCache(lru.New[*ast.QueryDocument](1000))

	srv.Use(extension.Introspection{})
	srv.Use(extension.AutomaticPersistedQuery{
		Cache: lru.New[string](100),
	})

	for _, opt := range opts {
		opt(srv)
	}
	return srv
}

func NewSchema(repo Repo, host string) graphql.ExecutableSchema {
	return NewExecutableSchema(Config{Resolvers: &Resolver{Repo: repo, Host: host}})
}

func FixedComplexityLimit(limit int) Option {
	return func(s *handler.Server) {
		if limit > 0 {
			s.Use(extension.FixedComplexityLimit(limit))
		}
	}
}
