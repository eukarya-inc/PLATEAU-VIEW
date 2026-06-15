module github.com/eukarya-inc/PLATEAU-VIEW/server

go 1.25.5

require (
	cloud.google.com/go/batch v1.12.2
	cloud.google.com/go/storage v1.54.0
	dario.cat/mergo v1.0.2
	github.com/99designs/gqlgen v0.17.73
	github.com/JamesLMilner/quadtree-go v0.0.0-20191212211504-d12870ffe403
	github.com/dustin/go-humanize v1.0.1
	github.com/eukarya-inc/japan-geoid-go v1.0.0
	github.com/eukarya-inc/jpareacode v1.0.1-0.20240314080116-ae89cfd85c6a
	github.com/eukarya-inc/plateau-spec/plateaudoc v0.0.0-20260615095223-6baeeef8c1d7
	github.com/eukarya-inc/plateau-spec/plateaudocsearch v0.0.0-20260615095223-6baeeef8c1d7
	github.com/go-playground/validator/v10 v10.26.0
	github.com/hako/durafmt v0.0.0-20210608085754-5c1018a4e16b
	github.com/hasura/go-graphql-client v0.14.3
	github.com/jarcoal/httpmock v1.4.0
	github.com/joho/godotenv v1.5.1
	github.com/k0kubun/pp/v3 v3.4.1
	github.com/kelseyhightower/envconfig v1.4.0
	github.com/klauspost/compress v1.18.0
	github.com/labstack/echo/v4 v4.13.3
	github.com/mark3labs/mcp-go v0.43.2
	github.com/mitchellh/mapstructure v1.5.0
	github.com/oklog/ulid/v2 v2.1.0
	github.com/orisano/gosax v1.1.2
	github.com/paulmach/go.geojson v1.5.0
	github.com/paulmach/orb v0.11.1
	github.com/reearth/reearth-cms-api/go v0.0.0-20251208103137-789348003578
	github.com/reearth/reearthx v0.0.0-20250514022647-16f9d767d93f
	github.com/samber/lo v1.52.0
	github.com/sendgrid/sendgrid-go v3.16.0+incompatible
	github.com/spf13/afero v1.14.0
	github.com/spkg/bom v1.0.1
	github.com/stretchr/testify v1.11.1
	github.com/tdewolff/canvas v0.0.0-20250508181010-75987a1ae9cc
	github.com/thanhpk/randstr v1.0.6
	github.com/vektah/gqlparser/v2 v2.5.27
	github.com/vincent-petithory/dataurl v1.0.0
	golang.org/x/net v0.47.0
	golang.org/x/sync v0.19.0
	google.golang.org/api v0.233.0
	google.golang.org/protobuf v1.36.6
)

require (
	cel.dev/expr v0.24.0 // indirect
	cloud.google.com/go v0.121.1 // indirect
	cloud.google.com/go/auth v0.16.1 // indirect
	cloud.google.com/go/auth/oauth2adapt v0.2.8 // indirect
	cloud.google.com/go/compute/metadata v0.7.0 // indirect
	cloud.google.com/go/iam v1.5.2 // indirect
	cloud.google.com/go/longrunning v0.6.7 // indirect
	cloud.google.com/go/monitoring v1.24.2 // indirect
	cloud.google.com/go/trace v1.11.6 // indirect
	codeberg.org/go-latex/latex v0.1.0 // indirect
	codeberg.org/go-pdf/fpdf v0.11.1 // indirect
	github.com/BurntSushi/freetype-go v0.0.0-20160129220410-b763ddbfe298 // indirect
	github.com/BurntSushi/graphics-go v0.0.0-20160129215708-b43f31a4a966 // indirect
	github.com/BurntSushi/xgb v0.0.0-20210121224620-deaf085860bc // indirect
	github.com/BurntSushi/xgbutil v0.0.0-20190907113008-ad855c713046 // indirect
	github.com/ByteArena/poly2tri-go v0.0.0-20170716161910-d102ad91854f // indirect
	github.com/GoogleCloudPlatform/opentelemetry-operations-go/detectors/gcp v1.27.0 // indirect
	github.com/GoogleCloudPlatform/opentelemetry-operations-go/exporter/metric v0.51.0 // indirect
	github.com/GoogleCloudPlatform/opentelemetry-operations-go/exporter/trace v1.27.0 // indirect
	github.com/GoogleCloudPlatform/opentelemetry-operations-go/internal/resourcemapping v0.51.0 // indirect
	github.com/Kagami/go-avif v0.1.0 // indirect
	github.com/RoaringBitmap/roaring/v2 v2.4.5 // indirect
	github.com/agnivade/levenshtein v1.2.1 // indirect
	github.com/andybalholm/brotli v1.1.1 // indirect
	github.com/auth0/go-jwt-middleware/v2 v2.2.1 // indirect
	github.com/bahlo/generic-list-go v0.2.0 // indirect
	github.com/benoitkugler/textlayout v0.3.1 // indirect
	github.com/benoitkugler/textprocessing v0.0.3 // indirect
	github.com/bits-and-blooms/bitset v1.22.0 // indirect
	github.com/blevesearch/bleve/v2 v2.5.7 // indirect
	github.com/blevesearch/bleve_index_api v1.2.11 // indirect
	github.com/blevesearch/geo v0.2.4 // indirect
	github.com/blevesearch/go-faiss v1.0.26 // indirect
	github.com/blevesearch/go-porterstemmer v1.0.3 // indirect
	github.com/blevesearch/gtreap v0.1.1 // indirect
	github.com/blevesearch/mmap-go v1.0.4 // indirect
	github.com/blevesearch/scorch_segment_api/v2 v2.3.13 // indirect
	github.com/blevesearch/segment v0.9.1 // indirect
	github.com/blevesearch/snowballstem v0.9.0 // indirect
	github.com/blevesearch/upsidedown_store_api v1.0.2 // indirect
	github.com/blevesearch/vellum v1.1.0 // indirect
	github.com/blevesearch/zapx/v11 v11.4.2 // indirect
	github.com/blevesearch/zapx/v12 v12.4.2 // indirect
	github.com/blevesearch/zapx/v13 v13.4.2 // indirect
	github.com/blevesearch/zapx/v14 v14.4.2 // indirect
	github.com/blevesearch/zapx/v15 v15.4.2 // indirect
	github.com/blevesearch/zapx/v16 v16.2.8 // indirect
	github.com/buger/jsonparser v1.1.1 // indirect
	github.com/cespare/xxhash/v2 v2.3.0 // indirect
	github.com/cncf/xds/go v0.0.0-20250501225837-2ac532fd4443 // indirect
	github.com/coder/websocket v1.8.13 // indirect
	github.com/cpuguy83/go-md2man/v2 v2.0.5 // indirect
	github.com/envoyproxy/go-control-plane/envoy v1.32.4 // indirect
	github.com/envoyproxy/protoc-gen-validate v1.2.1 // indirect
	github.com/felixge/httpsnoop v1.0.4 // indirect
	github.com/gabriel-vasile/mimetype v1.4.9 // indirect
	github.com/go-fonts/latin-modern v0.3.3 // indirect
	github.com/go-jose/go-jose/v4 v4.1.0 // indirect
	github.com/go-logr/logr v1.4.2 // indirect
	github.com/go-logr/stdr v1.2.2 // indirect
	github.com/go-text/typesetting v0.3.0 // indirect
	github.com/go-viper/mapstructure/v2 v2.2.1 // indirect
	github.com/goccy/go-yaml v1.17.1 // indirect
	github.com/gogo/protobuf v1.3.2 // indirect
	github.com/golang/freetype v0.0.0-20170609003504-e2365dfdc4a0 // indirect
	github.com/golang/snappy v0.0.4 // indirect
	github.com/google/s2a-go v0.1.9 // indirect
	github.com/google/uuid v1.6.0 // indirect
	github.com/googleapis/enterprise-certificate-proxy v0.3.6 // indirect
	github.com/googleapis/gax-go/v2 v2.14.2 // indirect
	github.com/gorilla/websocket v1.5.3 // indirect
	github.com/hashicorp/golang-lru/v2 v2.0.7 // indirect
	github.com/ikawaha/kagome-dict v1.1.7 // indirect
	github.com/ikawaha/kagome-dict/ipa v1.2.6 // indirect
	github.com/ikawaha/kagome/v2 v2.10.3 // indirect
	github.com/invopop/jsonschema v0.13.0 // indirect
	github.com/json-iterator/go v0.0.0-20171115153421-f7279a603ede // indirect
	github.com/kolesa-team/go-webp v1.0.5 // indirect
	github.com/mailru/easyjson v0.9.0 // indirect
	github.com/maruel/panicparse/v2 v2.5.0 // indirect
	github.com/mschoch/smat v0.2.0 // indirect
	github.com/nicksnyder/go-i18n/v2 v2.6.0 // indirect
	github.com/opentracing/opentracing-go v1.2.0 // indirect
	github.com/paulmach/protoscan v0.2.1 // indirect
	github.com/planetscale/vtprotobuf v0.6.1-0.20240319094008-0393e58bdf10 // indirect
	github.com/ravilushqa/otelgqlgen v0.17.0 // indirect
	github.com/richardlehane/mscfb v1.0.4 // indirect
	github.com/richardlehane/msoleps v1.0.4 // indirect
	github.com/russross/blackfriday/v2 v2.1.0 // indirect
	github.com/sosodev/duration v1.3.1 // indirect
	github.com/spf13/cast v1.7.1 // indirect
	github.com/spiffe/go-spiffe/v2 v2.5.0 // indirect
	github.com/srwiley/rasterx v0.0.0-20220730225603-2ab79fcdd4ef // indirect
	github.com/srwiley/scanx v0.0.0-20190309010443-e94503791388 // indirect
	github.com/tdewolff/font v0.0.0-20250430140153-b654fd8acba3 // indirect
	github.com/tdewolff/minify/v2 v2.23.5 // indirect
	github.com/tdewolff/parse/v2 v2.8.0 // indirect
	github.com/tiendc/go-deepcopy v1.6.0 // indirect
	github.com/uber/jaeger-client-go v2.30.0+incompatible // indirect
	github.com/uber/jaeger-lib v2.4.1+incompatible // indirect
	github.com/urfave/cli/v2 v2.27.6 // indirect
	github.com/wcharczuk/go-chart/v2 v2.1.2 // indirect
	github.com/wk8/go-ordered-map/v2 v2.1.8 // indirect
	github.com/xrash/smetrics v0.0.0-20240521201337-686a1a2994c1 // indirect
	github.com/yosida95/uritemplate/v3 v3.0.2 // indirect
	github.com/zeebo/errs v1.4.0 // indirect
	go.etcd.io/bbolt v1.4.0 // indirect
	go.mongodb.org/mongo-driver v1.17.3 // indirect
	go.opentelemetry.io/auto/sdk v1.1.0 // indirect
	go.opentelemetry.io/contrib v1.35.0 // indirect
	go.opentelemetry.io/contrib/detectors/gcp v1.35.0 // indirect
	go.opentelemetry.io/contrib/instrumentation/google.golang.org/grpc/otelgrpc v0.60.0 // indirect
	go.opentelemetry.io/contrib/instrumentation/net/http/otelhttp v0.60.0 // indirect
	go.opentelemetry.io/otel v1.35.0 // indirect
	go.opentelemetry.io/otel/metric v1.35.0 // indirect
	go.opentelemetry.io/otel/sdk v1.35.0 // indirect
	go.opentelemetry.io/otel/sdk/metric v1.35.0 // indirect
	go.opentelemetry.io/otel/trace v1.35.0 // indirect
	golang.org/x/image v0.27.0 // indirect
	golang.org/x/mod v0.30.0 // indirect
	golang.org/x/oauth2 v0.30.0 // indirect
	golang.org/x/tools v0.39.0 // indirect
	gonum.org/v1/plot v0.16.0 // indirect
	google.golang.org/genproto v0.0.0-20250512202823-5a2f75b736a9 // indirect
	google.golang.org/genproto/googleapis/api v0.0.0-20250512202823-5a2f75b736a9 // indirect
	google.golang.org/genproto/googleapis/rpc v0.0.0-20250512202823-5a2f75b736a9 // indirect
	google.golang.org/grpc v1.72.0 // indirect
	gopkg.in/go-jose/go-jose.v2 v2.6.3 // indirect
	modernc.org/knuth v0.5.5 // indirect
	modernc.org/token v1.1.0 // indirect
	star-tex.org/x/tex v0.7.1 // indirect
)

require (
	cloud.google.com/go/run v1.9.3
	github.com/davecgh/go-spew v1.1.1 // indirect
	github.com/go-playground/locales v0.14.1 // indirect
	github.com/go-playground/universal-translator v0.18.1 // indirect
	github.com/golang-jwt/jwt v3.2.2+incompatible // indirect
	github.com/labstack/gommon v0.4.2
	github.com/leodido/go-urn v1.4.0 // indirect
	github.com/mattn/go-colorable v0.1.14 // indirect
	github.com/mattn/go-isatty v0.0.20 // indirect
	github.com/pkg/errors v0.9.1
	github.com/pmezard/go-difflib v1.0.0 // indirect
	github.com/sendgrid/rest v2.6.9+incompatible // indirect
	github.com/valyala/bytebufferpool v1.0.0 // indirect
	github.com/valyala/fasttemplate v1.2.2 // indirect
	github.com/xuri/efp v0.0.1 // indirect
	github.com/xuri/excelize/v2 v2.9.1
	github.com/xuri/nfp v0.0.1 // indirect
	go.uber.org/atomic v1.11.0 // indirect
	go.uber.org/multierr v1.11.0 // indirect
	go.uber.org/zap v1.27.0 // indirect
	golang.org/x/crypto v0.44.0 // indirect
	golang.org/x/exp v0.0.0-20250506013437-ce4c2cf36ca6
	golang.org/x/sys v0.38.0 // indirect
	golang.org/x/text v0.32.0
	golang.org/x/time v0.11.0 // indirect
	gopkg.in/yaml.v3 v3.0.1
)

tool github.com/99designs/gqlgen
