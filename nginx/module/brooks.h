#ifndef BROOKS_H
#define BROOKS_H

/*! \file brooks.h
 * \brief C interface to the Brooks interpreter.
 */

/**
 * Contextualizes analyzed processing stages document.
 */
struct BrooksC;

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
 * @param pool The memory pool to use for allocations.
 * @param req The request to proxy.
 * @param body The body of the result.
 */
int ngx_brooks_proxy(struct BrooksC *context, ngx_http_request_t *req, ngx_buf_t **res_body);

/**
 * Analyze (parse/verify) a processing stages document.
 */
bool ngx_brooks_analyze(const char *path, struct BrooksC **, ngx_log_t *log);
#endif
