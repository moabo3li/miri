use std::env;
use std::path::PathBuf;
use std::process::Command;

use regex::bytes::Regex;
use ui_test::spanned::Spanned;
use ui_test::status_emitter::StatusEmitter;
use ui_test::{CommandBuilder, Config, default_file_filter, run_tests_generic};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // `priroda` is a target of the `miri` package, so `CARGO_MANIFEST_DIR` is the Miri repo root.
    let miri_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let priroda_dir = miri_dir.join("priroda");

    // The fixtures assume a host build of Miri, so skip them under cross-target bootstrap runs.
    // The bootstrap Priroda test step sets `MIRI_TEST_TARGET`.
    if let Some(miri_test_target) = env::var_os("MIRI_TEST_TARGET") {
        let host_target = Command::new("rustc").arg("--print").arg("host-tuple").output()?;
        let host_target = String::from_utf8(host_target.stdout)?.trim().to_owned();
        if miri_test_target.to_str() != Some(&host_target) {
            println!("skipping Priroda tests for non-host target {miri_test_target:?}");
            return Ok(());
        }
    }

    // The test fixtures use paths relative to the Priroda directory; run the harness from there
    // so the relative `tests/ui/...` paths resolve in the same way as a standalone crate.
    std::env::set_current_dir(&priroda_dir)?;

    let rustc_sysroot = Command::new("rustc").arg("--print").arg("sysroot").output()?;
    let rustc_sysroot = String::from_utf8(rustc_sysroot.stdout)?.trim().to_owned();

    let mut program = CommandBuilder::rustc();
    program.program = PathBuf::from(env!("CARGO_BIN_EXE_priroda"));

    // Remove logging env vars that might leak into stderr
    program.envs.push(("RUSTC_LOG".into(), None));
    program.envs.push(("RUST_LOG".into(), None));

    let mut config = Config {
        program,
        out_dir: PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("priroda_ui"),
        ..Config::rustc("tests/ui")
    };

    // Replace the dynamic paths in the actual stdout with the stable placeholders
    let manifest_dir_regex =
        Regex::new(&regex::escape(&priroda_dir.display().to_string())).unwrap();
    let miri_dir_regex = Regex::new(&regex::escape(&miri_dir.display().to_string())).unwrap();
    let rustc_sysroot_regex = Regex::new(&regex::escape(&rustc_sysroot)).unwrap();
    let pointer_regex = Regex::new(r"0x[0-9a-f]+\[alloc[0-9]+\]<[0-9]+>").unwrap();
    let crlf_regex = Regex::new(r"\r\n").unwrap();
    // DAP Content-Length headers embed the byte count of the following JSON,
    // which changes when path normalisation alters the embedded file paths.
    // Replace them with a placeholder so path-length differences between
    // machines do not make Content-Length drift from the normalised body.
    let content_length_regex = Regex::new(r"Content-Length: \d+").unwrap();
    // The set of compiler-generated MIR temporaries and the numeric MIR local
    // indices are properties of rustc's MIR construction, not of Priroda's
    // rendering, so they churn on every rustup that touches MIR building.
    // Skip the fixtures that only assert that inventory:
    //
    // - A row with no source name whose value is `<dead>` is a compiler slot
    //   that is out of scope, so in a blanket `locals` listing it records only
    //   rustc's liveness accounting, never a name or a rendered value. Every
    //   named local and every live value is still asserted, including pointer
    //   and uninit rendering. An explicit `print <id>` still asserts such a
    //   slot on purpose; those rows carry no `Name:` field and are unaffected.
    // - `Id: _N` is canonicalized rather than deleted: the projection suffix
    //   (`.0`, `.*`, ` as variant#1.0`) is what records which field or variant
    //   a source name maps to, so it stays.
    //
    // DAP output is unaffected: it carries the index inside a JSON name
    // (`"name":"_3"`), never as `Id: _3`. The `print <id>` commands in the
    // `.stdin` fixtures are also unaffected by stdout normalization.
    // The optional `(priroda) ` prompt prefix has to be part of the match: the
    // first row of a block is printed on the prompt line, so dropping only the
    // row would splice the next row onto a dangling prompt.
    let unnamed_dead_local_regex =
        Regex::new(r"(?:\(priroda\) )?Name: <none>, Id: _[0-9]+[^\n]*Value: <dead>\n?").unwrap();
    let local_id_regex = Regex::new(r"Id: _[0-9]+").unwrap();
    config.comment_defaults.base().normalize_stdout.extend([
        (manifest_dir_regex.into(), b"{MANIFEST_DIR}".to_vec()),
        (miri_dir_regex.into(), b"{MIRI_DIR}".to_vec()),
        (rustc_sysroot_regex.into(), b"{RUSTC_SYSROOT}".to_vec()),
        (pointer_regex.into(), b"{ALLOC_PTR}".to_vec()),
        // DAP frames use CRLF headers; keep checked-in stdout fixtures readable.
        (crlf_regex.into(), b"\n".to_vec()),
        (content_length_regex.into(), b"Content-Length: {CONTENT_LENGTH}".to_vec()),
        (unnamed_dead_local_regex.into(), b"".to_vec()),
        (local_id_regex.into(), b"Id: _{LOCAL}".to_vec()),
    ]);

    // Priroda UI tests do not currently require annotation comments in the test files.
    config.comment_defaults.base().exit_status = Spanned::dummy(0).into();
    config.comment_defaults.base().require_annotations = Spanned::dummy(false).into();

    let mut args = ui_test::Args::test()?;
    args.bless |= env::var_os("RUSTC_BLESS").is_some_and(|v| v != "0");
    config.with_args(&args);

    run_tests_generic(
        vec![config],
        default_file_filter,
        |_, _| {},
        Box::<dyn StatusEmitter>::from(args.format),
    )?;

    Ok(())
}
