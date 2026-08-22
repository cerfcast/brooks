#ifndef BROOKS_H
#define BROOKS_H

#include <stdbool.h>
#include <stdint.h>
#include <stddef.h>

/*! \file brooks.h
 * \brief C interface to the Brooks interpreter.
 */

/**
 * Contextualizes analyzed processing stages document.
 */
struct BrooksC;

struct BrooksCaddyRequestBuilder;
struct BrooksCaddyRequest;

struct BrooksCaddyRequestBuilder *brooks_caddy_request_builder_new();
struct BrooksCaddyRequestBuilder *brooks_caddy_request_builder_set_header(struct BrooksCaddyRequestBuilder *, char *, char *);
struct BrooksCaddyRequestBuilder *brooks_caddy_request_builder_set_method(struct BrooksCaddyRequestBuilder *, char *);
struct BrooksCaddyRequestBuilder *brooks_caddy_request_builder_set_uri(struct BrooksCaddyRequestBuilder *, char *);
struct BrooksCaddyRequestBuilder *brooks_caddy_request_builder_set_host(struct BrooksCaddyRequestBuilder *, char *);
struct BrooksCaddyRequest *brooks_caddy_request_builder_finalize_with_body(struct BrooksCaddyRequestBuilder *, char *);

/**
 * Proxy an HTTP request.
 *
 * Proxy \a req according to the processing stages document
 * contextualized by \a context. The response headers are
 * part of \a req (the way that they are in nginx) and the
 * response body is in \a res_body.
 *
 * @param context Contextualization for the processing stages
 * used to proxy \a req.
 * @param req_res The request/response to proxy.
 */
int brooks_caddy_proxy(struct BrooksC *context, struct BrooksCaddyRequest *request, void *request_response_ffi, void *logger);

typedef int (*caddy_brooks_result_user)(void *cookie, void *value);
typedef int (*caddy_brooks_result_user2)(void *cookie, void *value1, void *value2);

/**
 * Configure the brooks library.
 */
bool caddy_brooks_configure(struct BrooksC **, const char *path, void *logger);

int caddy_brooks_result_user_indirect(caddy_brooks_result_user called, uintptr_t data, uintptr_t cookie);
int caddy_brooks_result_user_indirect2(caddy_brooks_result_user2 called, uintptr_t data, uintptr_t data2, uintptr_t cookie);

#endif
