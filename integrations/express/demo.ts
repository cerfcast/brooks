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

import {
  configure_brooks,
  isError,
  make_logger,
  OutputPointer,
  toCString,
} from "./brooks.ts";

function proxy(): boolean | Error {
  const maybe_brooks = configure_brooks();

  if (isError(maybe_brooks)) {
    return maybe_brooks;
  }

  const brooks = maybe_brooks;
  const brooks_configuration = new OutputPointer();

  // Create a function that will handle log messages from Brooks.
  const logger = make_logger();

  const configure_result = brooks.symbols.express_brooks_configure(
    toCString("http://localhost:8081/"),
    brooks_configuration.ptr(),
    logger.pointer,
  );

  console.log(`configure_result: ${configure_result}`);

  let req = brooks.symbols.brooks_express_request_builder_new();
  req = brooks.symbols.brooks_express_request_builder_set_header(
    req,
    toCString("X-Header1"),
    toCString("Header1Value"),
  );
  req = brooks.symbols.brooks_express_request_builder_set_host(
    req,
    toCString("www.cnn.com"),
  );

  req = brooks.symbols.brooks_express_request_builder_set_uri(
    req,
    toCString("https://www.cnn.com/testing"),
  );

  req = brooks.symbols.brooks_express_request_builder_set_method(
    req,
    toCString("GET"),
  );
  const finalized_request = brooks.symbols
    .brooks_express_request_builder_finalize_with_body(
      req,
      toCString(""),
    );

  const result = brooks.symbols.brooks_express_proxy(
    brooks_configuration.dereference(),
    finalized_request,
    Deno.UnsafePointer.create(BigInt(0)),
    logger.pointer,
  );

  console.log(`${result}`);

  logger.close();
  brooks.close();

  return true;
}

if (import.meta.main) {
  proxy();
}
