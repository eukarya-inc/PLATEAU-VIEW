package main

import (
	"context"
	"errors"
	"flag"
	"fmt"
	"net/http"
	"os"
	"reflect"
	"runtime"
	"runtime/debug"
	"strings"
	"time"

	"github.com/eukarya-inc/PLATEAU-VIEW/server/plateaucms"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/putil"
	"github.com/eukarya-inc/PLATEAU-VIEW/server/tool"
	"github.com/go-playground/validator/v10"
	"github.com/k0kubun/pp/v3"
	"github.com/labstack/echo/v4"
	"github.com/labstack/echo/v4/middleware"
	glog "github.com/labstack/gommon/log"
	cms "github.com/reearth/reearth-cms-api/go"
	"github.com/reearth/reearth-cms-api/go/cmswebhook"
	"github.com/reearth/reearthx/appx"
	"github.com/reearth/reearthx/log"
	"github.com/reearth/reearthx/rerror"
	"github.com/samber/lo"
	"golang.org/x/net/http2"
)

func init() {
	pp.ColoringEnabled = false
}

func main() {
	// コマンドライン引数の定義
	var (
		generateDatacatalog = flag.String("generate-datacatalog", "", "Generate datacatalog cache for specified project (e.g., plateau-2024) and exit")
		outputToStdout      = flag.Bool("stdout", false, "Output JSON to stdout instead of file (use with --generate-datacatalog)")
		outputURL           = flag.String("output", "", "Output cache to specified URL (gs://bucket/path for GCS)")
		help                = flag.Bool("help", false, "Show help message")
	)

	// 既存のtoolコマンド用の処理を保持
	if len(os.Args) > 1 && !strings.HasPrefix(os.Args[1], "-") {
		conf := lo.Must(NewConfig())
		tool.Main(&tool.Config{
			CMS_BaseURL:       conf.CMS_BaseURL,
			CMS_Token:         conf.CMS_Token,
			CMS_SystemProject: conf.CMS_SystemProject,
		}, os.Args[1:])
		return
	}

	// --generate-datacatalog が値なしで指定された場合の前処理
	// (次の引数が -- で始まるか、最後の引数の場合は "all" を補完)
	preprocessArgs()

	flag.Parse()

	if *help {
		printHelp()
		os.Exit(0)
	}

	// 標準出力モードの場合は早めにログ出力先を変更
	if *generateDatacatalog != "" && *outputToStdout {
		log.SetOutput(os.Stderr)
	}

	// --generate-datacatalog フラグが明示的に指定されたかチェック
	generateDatacatalogSet := false
	flag.Visit(func(f *flag.Flag) {
		if f.Name == "generate-datacatalog" {
			generateDatacatalogSet = true
		}
	})

	conf := lo.Must(NewConfig())

	// データカタログ生成モードの場合
	if generateDatacatalogSet {
		var projects []string
		projectValue := strings.TrimSpace(*generateDatacatalog)

		// 空文字列または "all" の場合は全v3プロジェクトを対象にする
		if projectValue == "" || projectValue == "all" {
			v3Projects, err := getAllV3Projects(conf)
			if err != nil {
				log.Fatalf("Failed to get v3 projects: %v", err)
			}
			projects = v3Projects
			if !*outputToStdout {
				log.Infof("Found %d v3 projects: %v", len(projects), projects)
			}
		} else {
			for _, p := range strings.Split(projectValue, ",") {
				p = strings.TrimSpace(p)
				if p != "" {
					projects = append(projects, p)
				}
			}
		}

		var failedProjects []string
		for _, project := range projects {
			// 出力先URLの決定（常にプロジェクト名をサフィックスとして付加）
			projectOutputURL := *outputURL
			if projectOutputURL != "" {
				// outputURLの末尾にプロジェクト名を追加
				projectOutputURL = strings.TrimSuffix(projectOutputURL, "/") + "/" + project
			}

			generator := NewDatacatalogGenerator(conf, DatacatalogGeneratorOptions{
				OutputToStdout: *outputToStdout,
				OutputURL:      projectOutputURL,
			})
			if err := generator.Generate(project); err != nil {
				log.Errorf("Failed to generate datacatalog for %s: %v", project, err)
				failedProjects = append(failedProjects, project)
				continue
			}
			if !*outputToStdout {
				log.Infof("Successfully generated datacatalog cache for %s", project)
			}
		}

		if len(failedProjects) > 0 {
			log.Warnf("Failed to generate %d project(s): %v", len(failedProjects), failedProjects)
		}
		os.Exit(0)
	}

	main2(conf)
}

// preprocessArgs は --generate-datacatalog が値なしで指定された場合に "all" を補完する
func preprocessArgs() {
	for i, arg := range os.Args {
		// --generate-datacatalog または -generate-datacatalog を探す
		if arg == "--generate-datacatalog" || arg == "-generate-datacatalog" {
			// 最後の引数、または次の引数が - で始まる場合は値なし
			if i+1 >= len(os.Args) || strings.HasPrefix(os.Args[i+1], "-") {
				// "all" を補完
				os.Args[i] = "--generate-datacatalog=all"
			}
			return
		}
		// --generate-datacatalog= 形式（値あり）の場合は何もしない
		if strings.HasPrefix(arg, "--generate-datacatalog=") || strings.HasPrefix(arg, "-generate-datacatalog=") {
			return
		}
	}
}

func getAllV3Projects(conf *Config) ([]string, error) {
	pcms, err := plateaucms.New(plateaucms.Config{
		CMSBaseURL:       conf.CMS_BaseURL,
		CMSMainToken:     conf.CMS_Token,
		CMSSystemProject: conf.CMS_SystemProject,
	})
	if err != nil {
		return nil, fmt.Errorf("failed to create PLATEAU CMS client: %w", err)
	}

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	allMetadata, err := pcms.AllMetadata(ctx, true)
	if err != nil {
		return nil, fmt.Errorf("failed to get metadata: %w", err)
	}

	v3Projects := allMetadata.V3Projects()
	projects := make([]string, 0, len(v3Projects))
	for _, m := range v3Projects {
		projects = append(projects, m.DataCatalogProjectAlias)
	}

	return projects, nil
}

func printHelp() {
	fmt.Println("PLATEAU VIEW Server")
	fmt.Println()
	fmt.Println("Usage:")
	fmt.Println("  plateauview                                      # Start server")
	fmt.Println("  plateauview --generate-datacatalog               # Generate cache for all v3 projects")
	fmt.Println("  plateauview --generate-datacatalog plateau-2024  # Generate cache for specific project")
	fmt.Println("  plateauview --generate-datacatalog plateau-2024,plateau-2023  # Generate cache for multiple projects")
	fmt.Println("  plateauview --generate-datacatalog --output=gs://bucket/path  # Output to GCS")
	fmt.Println("  plateauview --generate-datacatalog plateau-2024 --stdout  # Output to stdout")
	fmt.Println()
	fmt.Println("Options:")
	fmt.Println("  --generate-datacatalog [projects]  Generate datacatalog cache (no value for all v3 projects, or comma-separated project names)")
	fmt.Println("  --stdout                           Output JSON to stdout instead of file (warnings to stderr)")
	fmt.Println("  --output <url>                     Output cache to specified URL (gs://bucket/path for GCS)")
	fmt.Println("  --help                             Show this help message")
}

func main2(conf *Config) {
	log.Infof("reearth-plateauview\n")
	log.Infof("config: %s", conf.Print())

	if conf.GCParcent > 0 {
		debug.SetGCPercent(conf.GCParcent)
	}

	logger := log.NewEcho()
	e := echo.New()
	e.HideBanner = true
	e.HidePort = true
	e.Logger = logger
	e.HTTPErrorHandler = errorHandler(e.DefaultHTTPErrorHandler)
	e.Validator = &customValidator{validator: validator.New()}
	e.Use(
		middleware.RecoverWithConfig(middleware.RecoverConfig{
			LogLevel: glog.ERROR,
		}),
		middleware.RequestID(),
		echo.WrapMiddleware(appx.RequestIDMiddleware()),
		logger.AccessLogger(),
		middleware.CORSWithConfig(middleware.CORSConfig{
			AllowOrigins: conf.Origin,
		}),
	)

	e.GET("/ping", func(c echo.Context) error {
		return c.JSON(http.StatusOK, "pong")
	}, putil.NoCacheMiddleware)

	services := lo.Must(Services(conf))
	serviceNames := lo.Map(services, func(s *Service, _ int) string { return s.Name })
	webhookHandlers := []cmswebhook.Handler{}
	for _, s := range services {
		if s.Echo != nil {
			g := e.Group("")
			if !s.DisableNoCache {
				g.Use(putil.NoCacheMiddleware)
			}
			lo.Must0(s.Echo(g))
		}
		if s.Webhook != nil {
			webhookHandlers = append(webhookHandlers, s.Webhook)
		}
	}

	cmsWebhookHandler(
		e.Group("/webhook"),
		[]byte(conf.CMS_Webhook_Secret),
		webhookHandlers,
	)

	log.Infof("enabled services: %v", serviceNames)
	addr := fmt.Sprintf("[::]:%d", conf.Port)
	log.Infof("http server started on %s", addr)
	log.Fatalf("%v", e.StartH2CServer(addr, &http2.Server{}))
}

func errorHandler(next func(error, echo.Context)) func(error, echo.Context) {
	return func(err error, c echo.Context) {
		if c.Response().Committed {
			return
		}

		code, msg := errorMessage(err, func(f string, args ...interface{}) {
			c.Echo().Logger.Errorf(f, args...)
		})
		if err := c.JSON(code, map[string]string{
			"error": msg,
		}); err != nil {
			next(err, c)
		}
	}
}

func errorMessage(err error, log func(string, ...interface{})) (int, string) {
	code := http.StatusBadRequest
	msg := err.Error()

	if err2, ok := err.(*echo.HTTPError); ok {
		code = err2.Code
		if msg2, ok := err2.Message.(string); ok {
			msg = msg2
		} else if msg2, ok := err2.Message.(error); ok {
			msg = msg2.Error()
		} else {
			msg = "error"
		}
		if err2.Internal != nil {
			log("echo internal err: %+v", err2)
		}
	} else if errors.Is(err, rerror.ErrNotFound) {
		code = http.StatusNotFound
		msg = "not found"
	} else if errors.Is(err, cms.ErrNotFound) {
		code = http.StatusNotFound
		msg = "not found"
	} else {
		if ierr := rerror.UnwrapErrInternal(err); ierr != nil {
			code = http.StatusInternalServerError
			msg = "internal server error"
		}
	}

	return code, msg
}

type customValidator struct {
	validator *validator.Validate
}

func (cv *customValidator) Validate(i any) error {
	if err := cv.validator.Struct(i); err != nil {
		return echo.NewHTTPError(http.StatusBadRequest, err.Error())
	}
	return nil
}

func funcName(i interface{}) string {
	return strings.TrimPrefix(runtime.FuncForPC(reflect.ValueOf(i).Pointer()).Name(), "main.")
}
