package estat

import (
	"context"
	"errors"
	"fmt"

	"cloud.google.com/go/firestore"
	"google.golang.org/api/iterator"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

const collectionPath = "api/estat/areas"

type firestoreStore struct {
	col *firestore.CollectionRef
}

func NewFirestoreStore(client *firestore.Client) Store {
	return &firestoreStore{col: client.Collection(collectionPath)}
}

func (s *firestoreStore) findIn(ctx context.Context, fields, tokens []string, limit int) ([]areaDoc, error) {
	q := s.col.Query
	for _, field := range fields {
		q = q.Where(field, "in", tokens)
	}
	q = q.OrderBy("properties.SETAI", firestore.Desc).Limit(limit).Select(selectFields...)
	return readAreaDocs(q.Documents(ctx))
}

func (s *firestoreStore) findPrefix(ctx context.Context, field, prefix string, limit int) ([]areaDoc, error) {
	q := s.col.Where(field, ">=", prefix).
		Where(field, "<=", prefix+"").
		Limit(limit).
		Select(selectFields...)
	return readAreaDocs(q.Documents(ctx))
}

func (s *firestoreStore) findGeometry(ctx context.Context, id string) (map[string]any, error) {
	snap, err := s.col.Doc(id).Get(ctx)
	if status.Code(err) == codes.NotFound {
		return nil, nil
	}
	if err != nil {
		return nil, fmt.Errorf("failed to get area %s: %w", id, err)
	}
	g, err := snap.DataAt("geometry")
	if err != nil {
		return nil, fmt.Errorf("failed to read geometry of area %s: %w", id, err)
	}
	m, ok := g.(map[string]any)
	if !ok {
		return nil, fmt.Errorf("invalid geometry of area %s: %T", id, g)
	}
	return m, nil
}

func readAreaDocs(it *firestore.DocumentIterator) ([]areaDoc, error) {
	defer it.Stop()
	var docs []areaDoc
	for {
		snap, err := it.Next()
		if errors.Is(err, iterator.Done) {
			return docs, nil
		}
		if err != nil {
			return nil, fmt.Errorf("failed to query areas: %w", err)
		}
		data := snap.Data()
		props, _ := data["properties"].(map[string]any)
		docs = append(docs, areaDoc{
			ID:         snap.Ref.ID,
			Properties: props,
			BBox:       toFloats(data["bbox"]),
		})
	}
}

func toFloats(v any) []float64 {
	items, _ := v.([]any)
	res := make([]float64, 0, len(items))
	for _, item := range items {
		switch n := item.(type) {
		case float64:
			res = append(res, n)
		case int64:
			res = append(res, float64(n))
		}
	}
	return res
}
