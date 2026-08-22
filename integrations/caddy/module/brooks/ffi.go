package brooks

// #cgo LDFLAGS: -L${SRCDIR}/c/install/ -lbrooks_lib -lbrooks_support
// #cgo CFLAGS: -I${SRCDIR}/c/
// #include "brooks.h"
import "C"
import (
	"fmt"
	"net/http"
	"runtime"
	"strings"
	"unsafe"

	"go.uber.org/zap"
)

type BrooksLibConfiguration struct {
	Cookie *C.struct_BrooksC
}

// Configure the brooks library to use `path` for retrieving host metadata.
func (blc *BrooksLibConfiguration) Configure(path string, caddylogger *zap.Logger) error {
	pinner := runtime.Pinner{}
	defer pinner.Unpin()

	bl := new(BrooksLog)
	defer bl.LogToZap(caddylogger)

	pinner.Pin(bl)

	if !C.caddy_brooks_configure(&blc.Cookie, C.CString(path), unsafe.Pointer(bl)) {
		return fmt.Errorf("Error occurred while configuring the brooks proxy library.")
	}

	return nil
}

// Use the available metadata to proxy a `req`.
func (blc *BrooksLibConfiguration) ServeHTTP(req *http.Request, caddylogger *zap.Logger) (*http.Response, error) {

	bl := new(BrooksLog)
	res := new(http.Response)

	res.Header = make(http.Header)

	reqres := new(CaddyResponse)
	reqres.res = res

	pinner := runtime.Pinner{}
	defer pinner.Unpin()

	pinner.Pin(res)
	pinner.Pin(reqres)
	pinner.Pin(bl)

	request := C.brooks_caddy_request_builder_new()

	for hv, hn := range req.Header {
		request = C.brooks_caddy_request_builder_set_header(request, C.CString(hv), C.CString(strings.Join(hn, ",")))
	}

	request = C.brooks_caddy_request_builder_set_uri(request, C.CString(req.RequestURI))
	request = C.brooks_caddy_request_builder_set_method(request, C.CString(req.Method))
	request = C.brooks_caddy_request_builder_set_host(request, C.CString(req.Host))

	defer bl.LogToZap(caddylogger)

	if C.brooks_caddy_proxy(blc.Cookie, C.brooks_caddy_request_builder_finalize_with_body(request, nil), unsafe.Pointer(reqres), unsafe.Pointer(bl)) != 0 {
		return nil, fmt.Errorf("Did not proxy")
	}
	return res, nil
}
