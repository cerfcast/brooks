package brooks

// #cgo LDFLAGS: -L${SRCDIR}/c/install/ -lbrooks_lib -lbrooks_support
// #cgo CFLAGS: -I${SRCDIR}/c/
// #include "brooks.h"
import "C"

import (
	"bytes"
	"io"
	"net/http"
	"unsafe"
)

// For communication of the HTTP request/response being manipulated between brooks and Go.
//
// The brooks library cannot see into this type.
type CaddyResponse struct {
	res *http.Response
}

//export caddy_response_set_body
func caddy_response_set_body(rr uintptr, l C.int, h *C.uint8_t) {
	x := (*CaddyResponse)(unsafe.Pointer(rr))

	b := C.GoBytes(unsafe.Pointer(h), l)

	x.res.Body = io.NopCloser(bytes.NewReader(b))
}

//export caddy_response_clear_header
func caddy_response_clear_header(rr uintptr, h *C.char) {
	// Fetch the request URI and prepare to use it as a parameter to the user-specified callback.
	x := (*CaddyResponse)(unsafe.Pointer(rr))

	x.res.Header.Del(C.GoString(h))
}

//export caddy_response_set_header
func caddy_response_set_header(rr uintptr, h *C.char, hv *C.char) bool {
	x := (*CaddyResponse)(unsafe.Pointer(rr))

	x.res.Header.Set(C.GoString(h), C.GoString(hv))

	return true
}

//export caddy_response_set_status
func caddy_response_set_status(rr uintptr, sc int) {
	// Fetch the request URI and prepare to use it as a parameter to the user-specified callback.
	x := (*CaddyResponse)(unsafe.Pointer(rr))

	x.res.StatusCode = sc
}
