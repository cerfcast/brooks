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
  return Deno.UnsafePointer.of(new Uint8ClampedArray([
    ...Array.from(str).map((v: string): number => {
      return v.charCodeAt(0);
    }),
    0,
  ]));
}

export function make_logger(): Deno.UnsafeCallback<
  { parameters: ["u8", "pointer"]; result: "void" }
> {
  const logit = new Map<number, (...data: any) => void>([
    [3, function (...data: any): void {
      console.error(`Error: ${data}`);
    }],
    [2, function (...data: any): void {
      console.warn(`Warn: ${data}`);
    }],
    [1, function (...data: any): void {
      console.debug(`Debug: ${data}`);
    }],
    [0, function (...data: any): void {
      console.trace(`Trace: ${data}`);
    }],
  ]);

  return new Deno.UnsafeCallback(
    { parameters: ["u8", "pointer"], result: "void" } as const,
    (level: number, value: Deno.PointerValue) => {
      logit.get(level)!((new Deno.UnsafePointerView(value!)).getCString());
    },
  );
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

  public ptr(): Deno.PointerValue {
    return this._ptr;
  }

  public dereference(): Deno.PointerValue {
    const tx = (new BigUint64Array(this.buffer())).at(0)!;
    return Deno.UnsafePointer.create(tx)!;
  }
}

export function configure_brooks():
  | Deno.DynamicLibrary<{
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
      parameters: ["pointer", "pointer", "pointer", "function"];
      result: "usize";
    };
  }>
  | Error {
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
          parameters: ["pointer", "pointer", "pointer", "function"],
          result: "usize",
        },
      },
    );
  } catch (err) {
    return new Error(`Failed to load Brooks library: ${err}`);
  }
}
