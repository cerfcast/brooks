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

export function isError(x: any): x is Error {
  return x instanceof Error;
}

export function toCString(str: string): Deno.PointerValue {
  return Deno.UnsafePointer.of(
    new Uint8ClampedArray([
      ...Array.from(str).map((v: string): number => {
        return v.charCodeAt(0);
      }),
      0,
    ]),
  );
}

export function toBodyPointer(body: any): Deno.PointerValue {
  if (body instanceof ArrayBuffer) {
    return Deno.UnsafePointer.of(body);
  } else {
    return Deno.UnsafePointer.of(new Uint8Array());
  }
}

type BrooksLogNativeCB = Deno.UnsafeCallback<
  { parameters: ["u8", "pointer"]; result: "void" }
>;

function make_logger(
  error: BrooksLogFunctionCB,
  warn: BrooksLogFunctionCB,
  debug: BrooksLogFunctionCB,
  trace: BrooksLogFunctionCB,
): BrooksLogNativeCB {
  const logit = new Map<number, (...data: any) => void>([
    [3, function (...data: any): void {
      error(`Error: ${data}`);
    }],
    [2, function (...data: any): void {
      warn(`Warn: ${data}`);
    }],
    [1, function (...data: any): void {
      debug(`Debug: ${data}`);
    }],
    [0, function (...data: any): void {
      trace(`Trace: ${data}`);
    }],
  ]);

  return new Deno.UnsafeCallback(
    { parameters: ["u8", "pointer"], result: "void" } as const,
    (level: number, value: Deno.PointerValue) => {
      logit.get(level)!((new Deno.UnsafePointerView(value!)).getCString());
    },
  );
}

export type BrooksLogFunctionCB = (...data: any) => void;

export class BrooksLogger {
  protected _native_cb: BrooksLogNativeCB;
  public constructor(
    error: BrooksLogFunctionCB,
    warn: BrooksLogFunctionCB,
    debug: BrooksLogFunctionCB,
    trace: BrooksLogFunctionCB,
  ) {
    this._native_cb = make_logger(error, warn, debug, trace);
  }

  public get pointer() {
    return this._native_cb.pointer;
  }
}

export class OutputPointer {
  private _raw: Uint8Array<ArrayBuffer>;
  private _ptr: Deno.PointerValue;

  public constructor() {
    this._raw = new Uint8Array(8);
    this._ptr = Deno.UnsafePointer.of(this._raw)!;
  }

  public buffer(): ArrayBufferLike {
    return this._raw.buffer;
  }

  public get pointer() {
    return this._ptr;
  }

  public dereference(): Deno.PointerValue {
    const tx = (new BigUint64Array(this.buffer())).at(0)!;
    return Deno.UnsafePointer.create(tx)!;
  }
}

export const BrooksResponseConfigurationNative = {
  struct: [
    "function", /* set header */
    "function", /* clear header */
    "function", /* set body */
    "function", /* set status */
  ],
} as const;

export type BrooksResponseConfigurationSetHeaderCB<T> = (
  slf: T,
  header_name: string,
  header_value: string,
) => void;
export type BrooksResponseConfigurationClearHeaderCB<T> = (
  slf: T,
  header_name: string,
) => void;
export type BrooksResponseConfigurationSetBodyCB<T> = (
  slf: T,
  body: Uint8Array,
) => void;
export type BrooksResponseConfigurationSetStatusCB<T> = (
  slf: T,
  status: number,
) => void;

export class BrooksResponseConfiguration<T> {
  private slf: T;

  private set_header: BrooksResponseConfigurationSetHeaderCB<T>;
  private clear_header: BrooksResponseConfigurationClearHeaderCB<T>;
  private set_body: BrooksResponseConfigurationSetBodyCB<T>;
  private set_status: BrooksResponseConfigurationSetStatusCB<T>;

  constructor(
    slf: T,
    set_header: BrooksResponseConfigurationSetHeaderCB<T>,
    clear_header: BrooksResponseConfigurationClearHeaderCB<T>,
    set_body: BrooksResponseConfigurationSetBodyCB<T>,
    set_status: BrooksResponseConfigurationSetStatusCB<T>,
  ) {
    this.slf = slf;
    this.set_header = set_header;
    this.clear_header = clear_header;
    this.set_body = set_body;
    this.set_status = set_status;
  }

  public get_configuration(): Uint8Array {
    const clear_header_cb = new Deno.UnsafeCallback(
      {
        parameters: ["pointer"],
        result: "void",
      } as const,
      (name: Deno.PointerValue): void => {
        const header_name = (new Deno.UnsafePointerView(name!)).getCString();
        this.clear_header(this.slf, header_name);
      },
    );

    const set_header_cb = new Deno.UnsafeCallback(
      {
        parameters: ["pointer", "pointer"],
        result: "void",
      } as const,
      (name: Deno.PointerValue, value: Deno.PointerValue): void => {
        const header_name = (new Deno.UnsafePointerView(name!)).getCString();
        const header_value = (new Deno.UnsafePointerView(value!)).getCString();
        this.set_header(this.slf, header_name, header_value);
      },
    );

    const set_body_cb = new Deno.UnsafeCallback(
      {
        parameters: ["pointer", "u32"],
        result: "void",
      } as const,
      (body: Deno.PointerValue, len: number): void => {
        // Check that this copy is necessary -- I think that it is.
        const response_body = new Uint8Array(len);
        new Deno.UnsafePointerView(body!).copyInto(response_body);
        this.set_body(this.slf, response_body);
      },
    );

    const set_status_cb = new Deno.UnsafeCallback(
      {
        parameters: ["u16"],
        result: "void",
      } as const,
      (status: number): void => {
        this.set_status(this.slf, status);
      },
    );

    const cbs = new BigInt64Array(
      [
        set_header_cb.pointer,
        clear_header_cb.pointer,
        set_body_cb.pointer,
        set_status_cb.pointer,
      ].map((v) => {
        return Deno.UnsafePointer.value(v);
      }),
    );

    return new Uint8Array(cbs.buffer);
  }
}

type BrooksLib = Deno.DynamicLibrary<{
  brooks_express_request_builder_new: {
    parameters: [];
    result: "pointer";
  };
  brooks_express_request_builder_set_header: {
    parameters: ["pointer", "pointer", "pointer"];
    result: "pointer";
  };
  brooks_express_request_builder_set_host: {
    parameters: ["pointer", "pointer"];
    result: "pointer";
  };
  brooks_express_request_builder_set_method: {
    parameters: ["pointer", "pointer"];
    result: "pointer";
  };
  brooks_express_request_builder_set_uri: {
    parameters: ["pointer", "pointer"];
    result: "pointer";
  };
  brooks_express_request_builder_finalize_with_body: {
    parameters: ["pointer", "pointer"];
    result: "pointer";
  };
  express_brooks_configure: {
    parameters: ["pointer", "pointer", /* out */ "function"];
    result: "bool";
  };
  brooks_express_proxy: {
    parameters: ["pointer", "pointer", any, "function"];
    result: "usize";
  };
}>;

export function load_brooks(): BrooksLib | Error {
  try {
    return Deno.dlopen(
      "libbrooks_lib.so",
      {
        brooks_express_request_builder_new: {
          parameters: [],
          result: "pointer",
        } as const,
        brooks_express_request_builder_set_header: {
          parameters: ["pointer", "pointer", "pointer"],
          result: "pointer",
        } as const,
        brooks_express_request_builder_set_host: {
          parameters: ["pointer", "pointer"],
          result: "pointer",
        } as const,
        brooks_express_request_builder_set_method: {
          parameters: ["pointer", "pointer"],
          result: "pointer",
        } as const,
        brooks_express_request_builder_set_uri: {
          parameters: ["pointer", "pointer"],
          result: "pointer",
        } as const,
        brooks_express_request_builder_finalize_with_body: {
          parameters: ["pointer", "pointer"],
          result: "pointer",
        } as const,
        express_brooks_configure: {
          parameters: ["pointer", "pointer", /* out */ "function"],
          result: "bool",
        } as const,
        brooks_express_proxy: {
          parameters: [
            "pointer",
            "pointer",
            BrooksResponseConfigurationNative,
            "function",
          ],
          result: "usize",
        },
      },
    );
  } catch (err) {
    return new Error(`Failed to load Brooks library: ${err}`);
  }
}

export class BrooksRequestBuilder {
  public lib: BrooksLib;
  public req: Deno.PointerValue;

  public constructor(lib: BrooksLib) {
    this.lib = lib;
    this.req = lib.symbols.brooks_express_request_builder_new();
  }

  public set_header(name: string, value: string) {
    this.req = this.lib.symbols.brooks_express_request_builder_set_header(
      this.req,
      toCString(name),
      toCString(value),
    );
  }

  public set_host(host: string) {
    this.req = this.lib.symbols.brooks_express_request_builder_set_host(
      this.req,
      toCString(host),
    );
  }

  public set_uri(uri: string) {
    this.req = this.lib.symbols.brooks_express_request_builder_set_uri(
      this.req,
      toCString(uri),
    );
  }

  public set_method(method: string) {
    this.req = this.lib.symbols.brooks_express_request_builder_set_method(
      this.req,
      toCString(method),
    );
  }

  public finalize(body: ArrayBuffer): Deno.PointerValue {
    return this.lib.symbols
      .brooks_express_request_builder_finalize_with_body(
        this.req,
        toBodyPointer(body),
      );
  }
}

export class Brooks {
  private lib: BrooksLib;
  private logger: BrooksLogger;
  private configuration: Deno.PointerValue;

  private constructor(
    lib: BrooksLib,
    logger: BrooksLogger,
    configuration: Deno.PointerValue,
  ) {
    this.lib = lib;
    this.logger = logger;
    this.configuration = configuration;
  }

  public static New(path: string, logger: BrooksLogger): Brooks | Error {
    const maybe_brooks = load_brooks();

    if (isError(maybe_brooks)) {
      return maybe_brooks;
    }

    const brooks = maybe_brooks;
    const brooks_configuration = new OutputPointer();

    const configure_result = brooks.symbols.express_brooks_configure(
      toCString(path),
      brooks_configuration.pointer,
      logger.pointer,
    );

    if (!configure_result) {
      return new Error(`Error configuring Brooks`);
    }

    return new Brooks(maybe_brooks, logger, brooks_configuration.dereference());
  }

  public make_request_builder(): BrooksRequestBuilder {
    return new BrooksRequestBuilder(this.lib)
  }

  public proxy<T>(
    request: Deno.PointerValue,
    response_configuration: BrooksResponseConfiguration<T>,
  ): bigint {
    return this.lib.symbols.brooks_express_proxy(
      this.configuration,
      request,
      response_configuration.get_configuration(),
      this.logger.pointer,
    );
  }
}
