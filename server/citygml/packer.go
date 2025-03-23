package citygml

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"net/http"
	"net/url"
	"path"
	"slices"
	"strconv"
	"strings"
	"time"

	"cloud.google.com/go/storage"
	"github.com/labstack/echo/v4"
	"github.com/reearth/reearthx/log"
	"google.golang.org/api/cloudbuild/v1"
	"google.golang.org/api/googleapi"
)

const (
	PackStatusAccepted  = "accepted"
	PackStatusSucceeded = "succeeded"

	timeoutSignedURL = 10 * time.Minute
)

type packer struct {
	conf   Config
	bucket *storage.BucketHandle
	build  *cloudbuild.Service
}

func newPacker(conf Config) *packer {
	ctx := context.Background()
	gcs, _ := storage.NewClient(ctx)
	bucket := gcs.Bucket(conf.Bucket)
	build, _ := cloudbuild.NewService(ctx)
	return &packer{
		conf:   conf,
		bucket: bucket,
		build:  build,
	}
}

func (p *packer) handleGetZip(c echo.Context, hash string) error {
	ctx := c.Request().Context()
	obj := p.bucket.Object(hash + ".zip")
	attrs, err := obj.Attrs(ctx)
	if errors.Is(err, storage.ErrObjectNotExist) {
		return c.JSON(http.StatusNotFound, map[string]any{"error": "not found"})
	}

	if status := getStatus(attrs.Metadata); status != PackStatusSucceeded {
		return c.JSON(http.StatusBadRequest, map[string]any{
			"error":  "invalid status",
			"status": status,
		})
	}

	signedURL, err := p.bucket.SignedURL(obj.ObjectName(), &storage.SignedURLOptions{
		Method:  http.MethodGet,
		Expires: time.Now().Add(timeoutSignedURL),
	})

	if err != nil {
		log.Errorfc(ctx, "citygml: packer: failed to issue signed url: %v", err)
		return c.JSON(http.StatusInternalServerError, map[string]any{
			"error": "failed to issue url",
		})
	}

	return c.Redirect(http.StatusFound, signedURL)
}

func (p *packer) handleGetStatus(c echo.Context, hash string) error {
	ctx := c.Request().Context()
	attrs, err := p.bucket.Object(hash + ".zip").Attrs(ctx)
	if errors.Is(err, storage.ErrObjectNotExist) {
		return c.JSON(http.StatusNotFound, map[string]any{"error": "not found"})
	}
	status := getStatus(attrs.Metadata)
	resp := map[string]any{
		"status": status,
	}
	if startedAt, ok := attrs.Metadata["startedAt"]; ok {
		resp["startedAt"] = startedAt
	}
	if status == PackStatusSucceeded {
		resp["progress"] = 1.0
	} else if progress, ok := getProgress(attrs.Metadata); ok {
		resp["progress"] = progress
	}

	return c.JSON(http.StatusOK, resp)
}

func getProgress(metadata map[string]string) (float64, bool) {
	totalStr, ok := metadata["total"]
	if !ok {
		return 0, false
	}
	processedStr, ok := metadata["processed"]
	if !ok {
		return 0, false
	}
	total, err := strconv.ParseInt(totalStr, 10, 64)
	if err != nil {
		return 0, false
	}
	processed, err := strconv.ParseInt(processedStr, 10, 64)
	if err != nil {
		return 0, false
	}
	return float64(processed) / float64(total), true
}

func (p *packer) handlePackRequest(c echo.Context) error {
	ctx := c.Request().Context()
	var req struct {
		URLs []string `json:"urls"`
	}
	if err := c.Bind(&req); err != nil {
		return c.JSON(http.StatusBadRequest, map[string]any{
			"error":  "invalid request body",
			"reason": err.Error(),
		})
	}
	if len(req.URLs) == 0 {
		return c.JSON(http.StatusBadRequest, map[string]any{
			"error": "no urls provided",
		})
	}
	for _, citygmlURL := range req.URLs {
		u, err := url.Parse(citygmlURL)
		if err != nil {
			return c.JSON(http.StatusBadRequest, map[string]any{
				"url":   citygmlURL,
				"error": "invalid url",
			})
		}
		if p.conf.Domain != "" && u.Host != p.conf.Domain {
			return c.JSON(http.StatusBadRequest, map[string]any{
				"url":   citygmlURL,
				"error": "invalid domain",
			})
		}
	}
	slices.Sort(req.URLs)
	checksum := sha256.Sum256([]byte(strings.Join(req.URLs, ",")))
	hash := hex.EncodeToString(checksum[:])
	var resp struct {
		ID string `json:"id"`
	}
	resp.ID = hash

	obj := p.bucket.Object(hash + ".zip").If(storage.Conditions{DoesNotExist: true})
	w := obj.NewWriter(ctx)
	w.ObjectAttrs.Metadata = Status(PackStatusAccepted)
	_, _ = w.Write(nil)
	if err := w.Close(); err != nil {
		var gErr *googleapi.Error
		if !(errors.As(err, &gErr) && gErr.Code == http.StatusPreconditionFailed) {
			log.Errorfc(ctx, "citygml: packer: failed to write metadata: %v", err)
			return c.JSON(http.StatusInternalServerError, map[string]any{
				"error": "failed to write metadata",
			})
		}
		return c.JSON(http.StatusOK, resp)
	}
	packReq := PackAsyncRequest{
		Dest:    toURL(obj),
		Domain:  p.conf.Domain,
		URLs:    req.URLs,
		Timeout: time.Duration(p.conf.PackerTimeout) * time.Second,
	}

	// when args are too long, Cloud Build sometimes fails, but retrying usually works.
	// googleapi: Error 400: invalid build: invalid .steps field: build step 0 arg 5 too long (max: 10000), badRequest
	const retry = 2
	for i := range retry {
		err := p.packAsync(ctx, packReq)
		if err == nil {
			break
		}

		if i == retry-1 {
			log.Errorfc(ctx, "citygml: packer: failed to enqueue pack job: %v", err)
			return c.JSON(http.StatusInternalServerError, map[string]any{
				"error": "failed to enqueue pack job",
			})
		}

		log.Debugfc(ctx, "citygml: packer: failed to enqueue pack job (%d): %v", i, err)
	}

	return c.JSON(http.StatusOK, resp)
}

func (p *packer) packAsync(ctx context.Context, req PackAsyncRequest) error {
	for _, u := range req.URLs {
		if strings.Contains(u, ",") {
			return fmt.Errorf("invalid url: %s", u)
		}
	}

	urls := strings.Join(req.URLs, ",")
	log.Debugfc(ctx, "citygml: packer: enqueue pack job: dest=%s, domain=%s, urls=%s", req.Dest, req.Domain, urls)

	args := []string{"citygml-packer", "-dest", req.Dest, "-domain", req.Domain}
	if req.Timeout > 0 {
		args = append(args, "-timeout", req.Timeout.String())
	}

	build := &cloudbuild.Build{
		Timeout:  "86400s", // 1 day
		QueueTtl: "86400s", // 1 day
		Steps: []*cloudbuild.BuildStep{
			{
				Name: p.conf.CityGMLPackerImage,
				Args: append(args, urls),
			},
		},
		Tags: []string{"citygml-packer"},
	}

	var op *cloudbuild.Operation
	var err error

	if p.conf.WorkerRegion != "" {
		call := p.build.Projects.Locations.Builds.Create(path.Join("projects", p.conf.WorkerProject, "locations", p.conf.WorkerRegion), build)
		op, err = call.Do()
	} else {
		call := p.build.Projects.Builds.Create(p.conf.WorkerProject, build)
		op, err = call.Context(ctx).Do()
	}
	if err != nil {
		return fmt.Errorf("create build: %w", err)
	}

	log.Debugfc(ctx, "citygml: packer: enqueued pack job: %s", op.Metadata)
	return nil
}

func Status(s string) map[string]string {
	return map[string]string{
		"status": s,
	}
}

func getStatus(metadata map[string]string) string {
	return metadata["status"]
}

func toURL(obj *storage.ObjectHandle) string {
	return "gs://" + obj.BucketName() + "/" + obj.ObjectName()
}

type PackAsyncRequest struct {
	Dest    string        `json:"dest"`
	Domain  string        `json:"domain"`
	URLs    []string      `json:"urls"`
	Timeout time.Duration `json:"timeout,omitempty"`
}
