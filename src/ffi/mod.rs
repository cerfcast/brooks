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

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unsafe_op_in_unsafe_fn)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::{
    ffi::{CStr, c_char},
    fs::OpenOptions,
    io::Read,
    marker::PhantomData,
};

use libc::malloc;

use crate::{
    mel::{
        scope::{Scopes, minimal_core_variable_types},
        tvs::Type,
    },
    ps::{
        interpret::{EffectfulProcessableRequestResponse, PsInterpretMode, interpret_stage},
        spec::{TypedGenericStage, TypedStage},
        verify::{PsVerificationKey, verify_ps_request_stage},
    },
};

unsafe fn to_nginx_str(s: &str) -> ngx_str_t {
    let len = s.len();
    let data = malloc(len) as *mut u8;
    let mut datai = data;

    for y in s.bytes() {
        *datai = y;
        datai = datai.wrapping_add(1);
    }

    ngx_str_t { len, data }
}

#[repr(C)]
pub struct BrooksC {
    _data: TypedStage<PsVerificationKey>,
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ngx_brooks_analyze(
    path: *const c_char,
    cookie: *mut *const BrooksC,
) -> bool {
    let path = match CStr::from_ptr(path).to_str() {
        Ok(o) => o,
        Err(_) => return false,
    }
    .to_string();

    let mut ps_contents: Vec<u8> = vec![];
    let mut ps_file = match OpenOptions::new().read(true).open(path) {
        Ok(o) => o,
        Err(_) => return false,
    };

    if ps_file.read_to_end(&mut ps_contents).is_err() {
        return false;
    }

    let ps_source = &String::from_utf8_lossy(&ps_contents);

    let result = match serde_json::from_str::<TypedGenericStage>(ps_source) {
        Ok(o) => o,
        Err(_) => return false,
    };

    let types_scope = Scopes::<Type> {
        scopes: vec![minimal_core_variable_types()],
    };
    let result = match verify_ps_request_stage(&result, types_scope) {
        Ok(o) => o,
        Err(_) => return false,
    };

    *cookie = Box::into_raw(Box::new(BrooksC {
        _data: result,
        _marker: PhantomData {},
    }));

    true
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ngx_brooks_proxy(
    cookie: *mut BrooksC,
    _i: *mut ngx_http_headers_in_t,
    _o: *mut ngx_http_headers_out_t,
) {
    let typed_stage = &(*cookie)._data;
    println!("typed_stage: {:?}", typed_stage);

    let mut effectful_req = EffectfulProcessableRequestResponse::default();
    let result = interpret_stage(typed_stage, &mut effectful_req, PsInterpretMode::Response)
        .expect("Could not interpret a valid client response");

    println!("result: {:?}", result);

    todo!()
}
