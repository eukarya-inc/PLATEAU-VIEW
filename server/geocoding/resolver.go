package geocoding

type Resolver struct {
	gsiClient *GSIClient
}

func NewResolver(gsiClient *GSIClient) *Resolver {
	return &Resolver{
		gsiClient: gsiClient,
	}
}
