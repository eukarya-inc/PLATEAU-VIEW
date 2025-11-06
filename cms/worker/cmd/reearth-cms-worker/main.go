package main

import "github.com/eukarya-inc/PLATEAU-VIEW-3.0/cms/worker/internal/app"

var version = ""

func main() {
	app.Start(debug, version)
}
