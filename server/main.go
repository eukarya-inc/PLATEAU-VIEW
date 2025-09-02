package main

import (
	"errors"
	"flag"
	"fmt"
	"net/http"
	"os"
	"reflect"
	"runtime"
	"runtime/debug"
	"strings"

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
		outputToStdout     = flag.Bool("stdout", false, "Output JSON to stdout instead of file (use with --generate-datacatalog)")
		help               = flag.Bool("help", false, "Show help message")
	)
	
	// 既存のtoolコマンド用の処理を保持
	if len(os.Args) > 1 && !strings.HasPrefix(os.Args[1], "-") {
		conf := lo.Must(NewConfig())
		tool.Main(&tool.Config{
			CMS_BaseURL: conf.CMS_BaseURL,
			CMS_Token:   conf.CMS_Token,
		}, os.Args[1:])
		return
	}
	
	flag.Parse()
	
	if *help {
		printHelp()
		os.Exit(0)
	}
	
	// 標準出力モードの場合は早めにログ出力先を変更
	if *generateDatacatalog != "" && *outputToStdout {
		log.SetOutput(os.Stderr)
	}
	
	conf := lo.Must(NewConfig())
	
	// データカタログ生成モードの場合
	if *generateDatacatalog != "" {
		generator := NewDatacatalogGenerator(conf, *outputToStdout)
		if err := generator.Generate(*generateDatacatalog); err != nil {
			log.Fatalf("Failed to generate datacatalog: %v", err)
		}
		if !*outputToStdout {
			log.Infof("Successfully generated datacatalog cache for %s", *generateDatacatalog)
		}
		os.Exit(0)
	}

	main2(conf)
}

func printHelp() {
	fmt.Println("PLATEAU VIEW Server")
	fmt.Println()
	fmt.Println("Usage:")
	fmt.Println("  plateauview                                      # Start server")
	fmt.Println("  plateauview --generate-datacatalog plateau-2024  # Generate cache and exit")
	fmt.Println("  plateauview --generate-datacatalog plateau-2024 --stdout  # Output to stdout")
	fmt.Println()
	fmt.Println("Options:")
	fmt.Println("  --generate-datacatalog <project>  Generate datacatalog cache for specified project")
	fmt.Println("  --stdout                         Output JSON to stdout instead of file (warnings to stderr)")
	fmt.Println("  --help                           Show this help message")
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
