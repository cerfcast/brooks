package brooks

import (
	"io"
	"net/http"

	"github.com/caddyserver/caddy/v2"
	"github.com/caddyserver/caddy/v2/caddyconfig/httpcaddyfile"
	"github.com/caddyserver/caddy/v2/modules/caddyhttp"
	"go.uber.org/zap"
)

func init() {
	// Register the middleware.
	caddy.RegisterModule(BrooksProxy{})

	// Support Caddyfile syntax for configuring this middleware.
	httpcaddyfile.RegisterHandlerDirective("brooks", parseCaddyfile)
}

type BrooksProxy struct {
	Path string `json:"path,omitempty"`

	blc    BrooksLibConfiguration
	logger *zap.Logger
}

// Provision implements [caddy.Provisioner].
func (b *BrooksProxy) Provision(ctx caddy.Context) error {
	b.logger = ctx.Logger()
	return nil
}

// Parse Caddyfile syntax for configuring the Brooks proxy as a middleware. Syntax:
//
// brooks [match] <path to socket>
//
// Note: The library that dispatches to this function has already handled the match set.
func parseCaddyfile(h httpcaddyfile.Helper) (caddyhttp.MiddlewareHandler, error) {
	h.Next() // consume directive name

	if h.CountRemainingArgs() == 0 {
		return nil, h.Errf("Not enough arguments specified for configuration of brooks proxy middleware -- 1 expected.", h.CountRemainingArgs())
	}
	if h.CountRemainingArgs() > 1 {
		return nil, h.Errf("Too many arguments specified for configuration of brooks proxy middleware -- 1 expected and %d were specified.", h.CountRemainingArgs())
	}
	h.NextArg()

	bp := BrooksProxy{Path: h.Val(), logger: nil}
	return &bp, nil
}

func (b *BrooksProxy) Validate() error {
	return b.blc.Configure(b.Path, b.logger)
}

func (b *BrooksProxy) ServeHTTP(writer http.ResponseWriter, req *http.Request, nxt caddyhttp.Handler) error {
	// Do the proxy.
	proxy, proxy_err := b.blc.ServeHTTP(req, b.logger)
	if proxy_err != nil {
		return proxy_err
	}

	// Get the result body into memory.
	body, bodyerr := io.ReadAll(proxy.Body)
	if bodyerr != nil {
		return bodyerr
	}

	// Write out the response.
	for hv := range proxy.Header {
		writer.Header().Set(hv, proxy.Header.Get(hv))
	}
	writer.WriteHeader(proxy.StatusCode)
	_, write_err := writer.Write(body)
	if write_err != nil {
		return write_err
	}

	return nxt.ServeHTTP(writer, req)
}

func (BrooksProxy) CaddyModule() caddy.ModuleInfo {
	return caddy.ModuleInfo{
		ID:  "http.handlers.brooks",
		New: func() caddy.Module { return new(BrooksProxy) },
	}
}

var (
	_ caddyhttp.MiddlewareHandler = (*BrooksProxy)(nil)
	_ caddy.Validator             = (*BrooksProxy)(nil)
	_ caddy.Provisioner           = (*BrooksProxy)(nil)
)
