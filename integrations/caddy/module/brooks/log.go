package brooks

// #cgo LDFLAGS: -L${SRCDIR}/c/install/ -lbrooks_lib -lbrooks_support
// #cgo CFLAGS: -I${SRCDIR}/c/
// #include "brooks.h"
import "C"
import (
	"unsafe"

	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
)

type BrooksLogMsg struct {
	lvl BrooksCaddyLogLevel
	msg string
}
type BrooksLog struct {
	msgs []BrooksLogMsg
}

type BrooksCaddyLogLevel C.uint8_t

const (
	Trace BrooksCaddyLogLevel = iota
	Debug
	Warn
	Error
)

func (bcll BrooksCaddyLogLevel) toZapLevel() zapcore.Level {
	switch bcll {
	case Trace:
		{
			return zap.InfoLevel
		}
	case Debug:
		{
			return zap.DebugLevel
		}
	case Warn:
		{
			return zap.WarnLevel
		}
	case Error:
		{
			return zap.ErrorLevel
		}
	}

	return zap.ErrorLevel
}

//export caddy_log
func caddy_log(log uintptr, level BrooksCaddyLogLevel, msg *C.char) {
	z := (*BrooksLog)(unsafe.Pointer(log))
	logmsg := BrooksLogMsg{lvl: level, msg: C.GoString(msg)}
	z.msgs = append(z.msgs, logmsg)
}

func (bl *BrooksLog) LogToZap(log *zap.Logger) {
	for msgi := range bl.msgs {
		log.Log(bl.msgs[msgi].lvl.toZapLevel(), bl.msgs[msgi].msg)
	}
}
