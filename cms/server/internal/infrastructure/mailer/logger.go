package mailer

import (
	"fmt"

	"github.com/eukarya-inc/PLATEAU-VIEW-3.0/cms/server/internal/usecase/gateway"
)

const loggerSep = "======================="

type logger struct{}

func NewLogger() gateway.Mailer {
	return &logger{}
}

func (m *logger) SendMail(to []gateway.Contact, subject, plainContent, _ string) error {
	logMail(to, subject)
	fmt.Printf("%s\n%s\n%s\n", loggerSep, plainContent, loggerSep)
	return nil
}
