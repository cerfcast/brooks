#include <stdbool.h>
#include <stdint.h>

#include "brooks.h"

int caddy_brooks_proxy(struct BrooksC *context, struct BrooksCaddyRequest *request, void *request_response_ffi, void *logger) {
    return 0;
}

bool caddy_brooks_configure(struct BrooksC **context, const char *path, void *logger) {
    return true;
}

struct BrooksCaddyRequestBuilder *brooks_caddy_request_builder_new() { return NULL; }
struct BrooksCaddyRequestBuilder *brooks_caddy_request_builder_set_header(struct BrooksCaddyRequestBuilder *crb, char *hn, char *hv) { return NULL; };
struct BrooksCaddyRequestBuilder *brooks_caddy_request_builder_set_method(struct BrooksCaddyRequestBuilder *crb, char *method) { return NULL; }
struct BrooksCaddyRequestBuilder *brooks_caddy_request_builder_set_uri(struct BrooksCaddyRequestBuilder *crb, char *uri) { return NULL; }
struct BrooksCaddyRequestBuilder *brooks_caddy_request_builder_set_host(struct BrooksCaddyRequestBuilder *crb, char *host) { return NULL; }
struct BrooksCaddyRequest *brooks_caddy_request_builder_finalize_with_body(struct BrooksCaddyRequestBuilder *crb, char *body) { return NULL; }

