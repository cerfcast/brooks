/*
 * This program is free software; you can redistribute it and/or
 * modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation; either version 2
 * of the License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA
 * 02110-1301, USA.
 */

/*
 * The basis of this module is the source code of the ngx_http_response_module.
 * That module is (c) Kirill A. Korinskiy.
 * Copyright (C) Kirill A. Korinskiy
 *
 * We believe that the code is now sufficiently different that we can
 * license as GPLv2.
 */

// Turn off clang-format here so that it does not rearrange include
// files. Order matters in nginx world.
// clang-format off
#include <nginx.h>
#include <ngx_config.h>
#include <ngx_core.h>
#include <ngx_http.h>
#include <assert.h>
#include <stdbool.h>
#include <stdio.h>
// clang-format on
// Turn clang-format back on.


#include "brooks.h"

/*
 * The data structure that holds the configuration that the user
 * provides for the brooks module.
 *
 * There is only one customizable value.
 */
struct ngx_http_brooks_conf_s {
	struct BrooksC *bc;
  ngx_str_t path;
};

typedef struct ngx_http_brooks_conf_s ngx_http_brooks_conf_t;

struct ngx_http_brooks_ctx_s {
  size_t       length;
  ngx_str_t    dev_zero_path;
  ngx_fd_t     dev_zero_fd;
  ngx_buf_t   *buffer;
  ngx_chain_t *output_chain;
};
typedef struct ngx_http_brooks_ctx_s ngx_http_brooks_ctx_t;

/*
 * Declare some functions that will do the real work of
 * managing the configuration parsing process. Declare before
 * implementation because we want to make pointers to them.
 */
static void *ngx_http_brooks_create_conf(ngx_conf_t *);
static char *ngx_http_brooks_merge_conf(ngx_conf_t *, void *, void *);
static char *ngx_http_brooks_enable(ngx_conf_t *, ngx_command_t *, void *);

/*
 * Specify the configuration options available for the user
 * of this module.
 */
static ngx_command_t ngx_http_brooks_commands[] = {
    {ngx_string("brooks"), NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1,
     ngx_http_brooks_enable, NGX_HTTP_LOC_CONF_OFFSET, 0, NULL},
    ngx_null_command};

/*
 * This struct will be used to tell nginx which of its
 * many *configuration* phases this module will join.
 */
static ngx_http_module_t ngx_http_brooks_module_ctx = {
    NULL, /* preconfiguration */
    NULL, /* postconfiguration */

    NULL, /* create main configuration */
    NULL, /* init main configuration */

    NULL, /* create server configuration */
    NULL, /* merge server configuration */

    ngx_http_brooks_create_conf, /* create location configuration */
    ngx_http_brooks_merge_conf   /* merge location configuration */
};

/*
 * The previous data structure was used to tell
 * nginx about which of its configuration phases
 * to join. This data structure will tell nginx
 * that we want to work with configuration and
 * that we are a module!
 */
ngx_module_t ngx_http_brooks_module = {
    NGX_MODULE_V1,
    &ngx_http_brooks_module_ctx, /* module context */
    ngx_http_brooks_commands,    /* module directives */
    NGX_HTTP_MODULE,           /* module type */
    NULL,                      /* init master */
    NULL,                      /* init module */
    NULL,                      /* init process */
    NULL,                      /* init thread */
    NULL,                      /* exit thread */
    NULL,                      /* exit process */
    NULL,                      /* exit master */
    NGX_MODULE_V1_PADDING};


/*
 * This callback function will be invoked when the pool
 * associated with the connection is cleaned up.
 *
 * See (below) for where/how it is installed.
 */

/*
 * ngx_http_brooks_handler
 *
 * This is the function that will execute to generate an
 * http response.
 *
 * Input: Information about the request being satisfied.
 * Output: An error code indicating to nginx whether we
 * were successful in generating that response.
 */
static ngx_int_t ngx_http_brooks_handler(ngx_http_request_t *r) {
  ngx_http_brooks_conf_t *conf = NULL;

	ngx_http_complex_value_t  cv;

	if (!(r->method & (NGX_HTTP_GET|NGX_HTTP_HEAD))) {
					return NGX_HTTP_NOT_ALLOWED;
	}

	ngx_memzero(&cv, sizeof(ngx_http_complex_value_t));


  /*
   * We could fail to read the module configuration.
   */
  conf = ngx_http_get_module_loc_conf(r, ngx_http_brooks_module);
  if (!conf) {
    ngx_log_error(
        NGX_LOG_CRIT, r->connection->log, 0,
        "brooks could not access configuration data when handling a request");
    return NGX_HTTP_INTERNAL_SERVER_ERROR;
  }

  ngx_log_error(NGX_LOG_DEBUG, r->connection->log, 0,
                "ngx_http_brooks_create_conf starting");

  ngx_log_debug1(NGX_LOG_DEBUG_HTTP, r->connection->log, 0,
                   "output: %lu", r->headers_in.server.len);

  ngx_brooks_proxy(conf->bc, r);

  ngx_log_debug1(NGX_LOG_DEBUG_HTTP, r->connection->log, 0,
                   "output: \"%V\"", &r->headers_out.content_type);

	cv.value.len = r->headers_out.content_type.len;
	cv.value.data = r->headers_out.content_type.data;

  return ngx_http_send_response(r, r->headers_out.status, &r->headers_out.content_type, &cv);
}


/*
 * ngx_http_brooks_create_conf
 *
 * Thsi function will allocate the space necessary for
 * the data structure that will hold configuration information
 * about our module. We set the initial value of the
 * configuration options to `UNSET` so that nginx knows
 * what to do with them when running various helper functions
 * (see ngx_http_brooks_merge_conf, below)
 *
 * Input: A pointer to the overall server configuration (unused)
 * Output: A pointer to the (newly) allocated memory
 * that will hold the configuration for this module.
 */
static void *ngx_http_brooks_create_conf(ngx_conf_t *cf) {
  ngx_http_brooks_conf_t *conf;

  conf = ngx_pcalloc(cf->pool, sizeof(ngx_http_brooks_conf_t));
  if (conf == NULL) {
    return NULL;
  }

  /*
   * set by ngx_pcalloc():
   *
   *     conf->path = { 0, NULL };
   */
  
  return conf;
}

/*
 * ngx_http_brooks_merge_conf
 *
 * So-called location configurations can nest in nginx. This
 * function will handle "merging" nested configuration options.
 *
 * Input: A pointer to the overall server configuration (unused)
 *        A pointer to the enclosing location configuration
 *        A pointer to the immediate configuration
 * Output: A status indicator telling nginx whether we could
 * successfully merge the two configurations into the immediate
 * configuration.
 */
static char *ngx_http_brooks_merge_conf(ngx_conf_t *cf, void *parent,
                                      void *child) {
  ngx_http_brooks_conf_t *prev = parent;
  ngx_http_brooks_conf_t *conf = child;
    
  if (conf->path.len == 0) {
    conf->path = prev->path;
  }

  return NGX_CONF_OK;
}

/*
 * ngx_http_brooks_enable
 *
 * This function is invoked by nginx when it sees a `brooks`
 * directive in the configuration file.
 *
 * Input: The overall server configuration
 *        The text of the raw configuration command being processed
 *        A pointer to the extra information specified for this
 *        callback above (see ngx_http_brooks_commands).
 * Output: The result of processing the command (indirectly through
 * a helper function provided be nginx that actually processes the
 * command after we do some prework.
 */
static char *ngx_http_brooks_enable(ngx_conf_t *cf, ngx_command_t *cmd,
                                  void *conf) {
  ngx_http_brooks_conf_t *rlcf = conf;
  ngx_str_t *value;
  value = cf->args->elts;
  rlcf->path = value[1];
  u_char pathstr[NGX_MAX_PATH] = {0, };

	if (rlcf->path.len >= NGX_MAX_PATH) {
    ngx_log_error(
        NGX_LOG_CRIT, cf->log, 0,
        "brooks: path to processing stages JSON document too long");
    return NGX_CONF_ERROR;
	}

	ngx_memcpy(pathstr, rlcf->path.data, rlcf->path.len);

	if (!ngx_brooks_analyze((const char*)pathstr, &rlcf->bc)) {
    ngx_log_error(
        NGX_LOG_CRIT, cf->log, 0,
        "brooks: path to processing stages JSON document could not be verified");
    return NGX_CONF_ERROR;
	}
    
  ngx_http_core_loc_conf_t *clcf =
      ngx_http_conf_get_module_loc_conf(cf, ngx_http_core_module);
  clcf->handler = ngx_http_brooks_handler;
  return NGX_CONF_OK;
}
