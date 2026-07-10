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

pub mod cg;

use std::{
    fs::OpenOptions,
    io::{BufWriter, Read, Write},
    path::{Path, PathBuf},
};

use crate::mel::{
    analysis::Analyzed,
    ast::{AstVisitorDriver, Expr},
    c::cg::{
        LocatableString, MelCodegen, MelCodegenContext, MelCodegenError, MelCodegenLocatableError,
        mel_type_to_c_type,
    },
};

#[cfg(test)]
mod tests;

#[allow(clippy::result_large_err)]
pub fn codegen<A: Write>(
    expr: &Expr<Analyzed>,
    context: MelCodegenContext,
    mut output: A,
) -> Result<A, MelCodegenLocatableError> {
    let driver = AstVisitorDriver {};
    let visitor = MelCodegen {};

    let result = driver.visit(expr, &visitor, context)?;

    for c in &result.code {
        write!(output, "// {}\n{}\n", c.l, c.s).map_err(|e| MelCodegenLocatableError {
            location: expr.location().clone(),
            error: MelCodegenError::WriteFailed(Box::new(e)),
            context: result.clone(),
        })?;
    }

    Ok(output)
}

#[allow(clippy::result_large_err)]
pub fn codegen_function<A: Write>(
    expr: &Expr<Analyzed>,
    context: MelCodegenContext,
    function_name: &str,
    params: Vec<(String, String)>,
    mut output: A,
) -> Result<A, MelCodegenLocatableError> {
    let driver = AstVisitorDriver {};
    let visitor = MelCodegen {};

    writeln!(
        output,
        "{} {}({}) {{",
        mel_type_to_c_type(&expr.tipe()),
        function_name,
        params
            .iter()
            .map(|(tpe, name)| { format!("{} {}", tpe, name) })
            .collect::<Vec<_>>()
            .join(",")
    )
    .map_err(|e| MelCodegenLocatableError {
        location: expr.location().clone(),
        error: MelCodegenError::WriteFailed(Box::new(e)),
        context: context.clone(),
    })?;

    let mut context = driver.visit(expr, &visitor, context)?;

    // Add a return statement!
    let ret = format!("return {};", context.ssa);

    context = context.append_code(LocatableString {
        s: ret,
        l: expr.location().clone(),
    });

    for c in &context.code {
        write!(output, "// {}\n{}\n", c.l, c.s).map_err(|e| MelCodegenLocatableError {
            location: expr.location().clone(),
            error: MelCodegenError::WriteFailed(Box::new(e)),
            context: context.clone(),
        })?;
    }

    writeln!(output, "}}").map_err(|e| MelCodegenLocatableError {
        location: expr.location().clone(),
        error: MelCodegenError::WriteFailed(Box::new(e)),
        context: context.clone(),
    })?;
    Ok(output)
}

#[allow(clippy::result_large_err)]
pub fn codegen_project(
    expr: &Expr<Analyzed>,
    context: MelCodegenContext,
    function_name: &str,
    params: Vec<(String, String)>,
    output_path: &Path,
    materials_path: &Path,
) -> Result<(), MelCodegenLocatableError> {
    let driver = AstVisitorDriver {};
    let visitor = MelCodegen {};

    let mut skeleton_contents_path = materials_path.to_path_buf();
    skeleton_contents_path.push("mel.cpp");

    let mut skeleton_contents: Vec<u8> = vec![];
    OpenOptions::new()
        .read(true)
        .open(skeleton_contents_path)
        .expect("Could not open skeleton file.")
        .read_to_end(&mut skeleton_contents)
        .expect("Could not read string from skeleton file");

    let skeleton_contents =
        String::from_utf8(skeleton_contents).expect("Could not convert source to string");

    let function_data: Vec<u8> = vec![];
    let mut function_buffer = BufWriter::new(function_data);

    writeln!(
        function_buffer,
        "{} {}({}) {{",
        mel_type_to_c_type(&expr.tipe()),
        function_name,
        params
            .iter()
            .map(|(tpe, name)| { format!("{} {}", tpe, name) })
            .collect::<Vec<_>>()
            .join(",")
    )
    .map_err(|e| MelCodegenLocatableError {
        location: expr.location().clone(),
        error: MelCodegenError::WriteFailed(Box::new(e)),
        context: context.clone(),
    })?;

    let mut context = driver.visit(expr, &visitor, context)?;

    // Add a return statement!
    let ret = format!("return {};", context.ssa);

    context = context.append_code(LocatableString {
        s: ret,
        l: expr.location().clone(),
    });

    for c in &context.code {
        write!(function_buffer, "\t// {}\n\t{}\n", c.l, c.s).map_err(|e| {
            MelCodegenLocatableError {
                location: expr.location().clone(),
                error: MelCodegenError::WriteFailed(Box::new(e)),
                context: context.clone(),
            }
        })?;
    }

    writeln!(function_buffer, "}}").map_err(|e| MelCodegenLocatableError {
        location: expr.location().clone(),
        error: MelCodegenError::WriteFailed(Box::new(e)),
        context: context.clone(),
    })?;

    let function_code = String::from_utf8_lossy(function_buffer.buffer());

    let skeleton_contents = skeleton_contents.replace("INTERPRET_FUNCTION", &function_code);

    let mut skeleton_output_path = output_path.to_path_buf();
    skeleton_output_path.push(PathBuf::from("mel.cpp"));

    let mut output = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(skeleton_output_path)
        .expect("Could not open output skeleton file");

    output
        .write_all(skeleton_contents.as_bytes())
        .expect("Could not write to output file");

    let mut cmake_path = materials_path.to_path_buf();
    cmake_path.push("CMakeLists.txt");
    let mut cmake_output_path = output_path.to_path_buf();
    cmake_output_path.push("CMakeLists.txt");
    std::fs::copy(cmake_path, cmake_output_path).expect("Could not copy CMakeLists.txt");

    Ok(())
}
