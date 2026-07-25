#ifndef BROOKS_H
#define BROOKS_H

struct BrooksC;

void ngx_brooks_proxy(struct BrooksC *, ngx_http_request_t *);
bool ngx_brooks_analyze(const char *path, struct BrooksC **);
#endif
