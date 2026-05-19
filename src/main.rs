use std::{error::Error, ffi::OsStr, path::PathBuf};

use clap::{arg, command, value_parser};
use codespan_reporting::{
    diagnostic::{Diagnostic, Label},
    files::SimpleFile,
    term::{
        self,
        termcolor::{ColorChoice, StandardStream},
    },
};
use wgsl_minifier::{minify_module, minify_wgsl_source};
fn main() {
    let matches = command!()
        .arg(
            arg!(
                [input] "The wgsl shader to minify"
            )
            .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            arg!(
                [output] "Where the minified file should be written"
            )
            .required(false)
            .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            arg!(
                -f --force "Forces writing the output without checking, even if the output file already exists"
            )
        )
        .get_matches();

    let input = match matches.get_one::<PathBuf>("input") {
        Some(input) => input,
        None => {
            eprintln!("Example usage: `wgsl-minify path/to/my/shader.wgsl path/to/my/output.wgsl`");
            eprintln!("No input shader provided.");
            return;
        }
    };

    let input_shader = match std::fs::read_to_string(input) {
        Ok(vs) => vs,
        Err(e) => {
            eprintln!("could not read file: {}", e);
            return;
        }
    };

    let output = matches
        .get_one::<PathBuf>("output")
        .cloned()
        .unwrap_or(PathBuf::from({
            let mut str = input.clone().into_os_string();
            str.push(OsStr::new(".minified"));
            str
        }));

    let should_force = matches.get_flag("force");
    let output_path = std::path::Path::new(&output);
    if output_path.exists() && !should_force {
        eprintln!(
            "output file `{}` already exists - force overwriting with the --force parameter",
            output_path.display()
        );
        return;
    }

    let input_file_name = input
        .file_name()
        .unwrap_or(OsStr::new("input"))
        .to_string_lossy();

    let mut module = match naga::front::wgsl::parse_str(&input_shader) {
        Ok(module) => module,
        Err(e) => {
            eprintln!(
                "{}\nfailed to parse shader",
                e.emit_to_string_with_path(&input_file_name, &*input.to_string_lossy())
            );
            return;
        }
    };

    // Validate before minification so that errors are better
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    if let Err(e) = validator.validate(&module) {
        let mut e_base: &dyn Error = &e;
        let mut message = format!("{}", e);
        while let Some(e) = e_base.source() {
            message = format!("{}: {}", message, e);
            e_base = e;
        }

        let diagnostic = Diagnostic::error()
            .with_message(message.to_string())
            .with_labels(
                e.spans()
                    .map(|(span, msg)| {
                        Label::primary((), span.to_range().unwrap()).with_message(msg)
                    })
                    .collect(),
            );

        let source = input.to_string_lossy();
        let files = SimpleFile::new(input_file_name.as_ref(), &source);
        let config = codespan_reporting::term::Config::default();
        let writer = StandardStream::stderr(ColorChoice::Auto);
        term::emit_to_write_style(&mut writer.lock(), &config, &files, &diagnostic)
            .expect("cannot write error");
    }

    // Now minify!
    minify_module(&mut module);

    // Write to string
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    let info = validator
        .validate(&module)
        .expect("previously validated shader should be valid after semantic minification");

    let output =
        naga::back::wgsl::write_string(&module, &info, naga::back::wgsl::WriterFlags::empty())
            .expect("if was representable in wgsl, should still be representable in wgsl");

    // Minify string
    let output = minify_wgsl_source(&output);

    // Sanity check
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    validator
        .validate(&module)
        .expect("previously validated shader should be valid after text minification");

    // And we're done
    if let Err(e) = std::fs::write(output_path, output) {
        eprintln!(
            "failed to write minified shader to {}: {}",
            output_path.display(),
            e
        );
    }
}
