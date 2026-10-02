// brooks, Copyright 2026, Will Hawkins
//
// This file is part of brooks.

// This file is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

// @ts-types="npm:@types/express@4.17.15"
import express from "express";
import { Request, Response } from "express";

import {
  Brooks,
  BrooksLogger,
  BrooksRequestBuilder,
  BrooksResponseConfiguration,
  isError,
} from "./brooks.ts";

class BrooksExpressRequest {
  static to(r: Request, builder: BrooksRequestBuilder): Deno.PointerValue {
    for (const header in r.headers) {
      const [name, value] = [header, r.get(header)];
      if (!value) {
        console.warn(`Skipping header ${name} because it has no value.`);
        continue;
      }
      console.log(`Setting ${name} = ${value}`);
      builder.set_header(name, value);
    }

    builder.set_host(r.hostname);

    console.log(`Original url: ${r.url}`);
    builder.set_uri(
      r.secure ? "https" : "http" + "://" + r.hostname + r.originalUrl,
    );

    builder.set_method(r.method);

    return builder.finalize(r.body);
  }
}

class BrooksExpressResponse {
  private _res: Response;
  private _body: Uint8Array | null = null;

  public constructor(res: Response) {
    this._res = res;
  }

  public get res() {
    return this._res;
  }

  public set res(nr: Response) {
    this._res = nr;
  }

  public get body() {
    if (!this._body) {
      return new Uint8Array();
    }
    return this._body;
  }

  public set body(nb: Uint8Array) {
    this._body = nb;
  }

  public send(): Response {
    return this._res.type("text/html").send(this.body);
  }
}

class BrooksExpressResponseMediator {
  public static to(
    response: BrooksExpressResponse,
  ): BrooksResponseConfiguration<BrooksExpressResponse> {
    return new BrooksResponseConfiguration(
      response,
      BrooksExpressResponseMediator.set_header,
      BrooksExpressResponseMediator.clear_header,
      BrooksExpressResponseMediator.set_body,
      BrooksExpressResponseMediator.set_status,
    );
  }

  private static set_header(
    slf: BrooksExpressResponse,
    header_name: string,
    header_value: string,
  ) {
    console.log(`Setting header named ${header_name} to ${header_value}`);
    slf.res = slf.res.setHeader(header_name, header_value);
  }

  private static clear_header(slf: BrooksExpressResponse, header_name: string) {
    console.log(`Clearing header named ${header_name}`);
    slf.res.removeHeader(header_name);
  }

  private static set_body(slf: BrooksExpressResponse, body: Uint8Array) {
    console.log(`Setting body of length ${body.length}`);
    slf.body = body;
  }

  private static set_status(slf: BrooksExpressResponse, status: number) {
    console.log(`Setting status to ${status}`);
    slf.res = slf.res.status(status);
  }
}

if (import.meta.main) {
  const app = express();

  const brooks_logger = new BrooksLogger(
    console.error,
    console.warn,
    console.debug,
    console.trace,
  );

  const maybe_brooks = Brooks.New("http://localhost:8081/", brooks_logger);

  if (isError(maybe_brooks)) {
    console.error(`Error loading Brooks: ${maybe_brooks}`);
    Deno.exit(-1);
  }
  const brooks = maybe_brooks;

  app.get(/.*/, (req, res) => {
    const brooks_request = BrooksExpressRequest.to(
      req,
      brooks.make_request_builder(),
    );
    const brooks_response = new BrooksExpressResponse(res);
    const response_handler = BrooksExpressResponseMediator.to(brooks_response);
    const result = brooks.proxy(brooks_request, response_handler);

    console.log(`${result}`);

    return brooks_response.send();
  });

  app.listen(8080);
  console.log(`Server is running on http://localhost:8080`);
}
