use anyhow::{Context, Result};
use itertools::Itertools;
use precious_helpers::exec;
use precious_testhelper::TestHelper;
use pretty_assertions::{assert_eq, assert_str_eq};
use regex::Regex;
use serial_test::serial;
use std::{collections::HashMap, env, fs, path::PathBuf};

const CONFIG: &str = r#"
exclude = [
  "target",
]

[commands.rustfmt]
type    = "both"
include = "**/*.rs"
cmd     = [ "rustfmt", "--edition", "2021" ]
lint_flags = "--check"
ok_exit_codes = 0
lint_failure_exit_codes = 1

[commands.true]
type    = "lint"
include = "**/*.rs"
cmd     = [ "true" ]
ok_exit_codes = 0
lint_failure_exit_codes = 1

[commands.stderr]
type    = "lint"
include = "**/*.rs"
cmd     = [ "sh", "-c", "echo 'some stderr output' 1>&2" ]
ok_exit_codes = 0
lint_failure_exit_codes = 1
ignore_stderr = "some.+output"
"#;

const GOOD_RUST: &str = r#"
fn good_func() {
    let a = 1 + 2;
    println!("a = {}", a);
}
"#;

#[test]
#[serial]
fn all() -> Result<()> {
    let helper = do_test_setup()?;

    let precious = precious_path()?;
    let env = HashMap::new();
    exec::run(
        &precious,
        &["lint", "--all"],
        &env,
        &[0],
        None,
        Some(&helper.precious_root()),
    )?;
    exec::run(
        &precious,
        &["tidy", "--all"],
        &env,
        &[0],
        None,
        Some(&helper.precious_root()),
    )?;

    Ok(())
}

#[test]
#[serial]
fn git() -> Result<()> {
    let helper = do_test_setup()?;
    helper.modify_files()?;

    let precious = precious_path()?;
    let env = HashMap::new();
    exec::run(
        &precious,
        &["lint", "--git"],
        &env,
        &[0],
        None,
        Some(&helper.precious_root()),
    )?;
    exec::run(
        &precious,
        &["tidy", "--git"],
        &env,
        &[0],
        None,
        Some(&helper.precious_root()),
    )?;

    Ok(())
}

#[test]
#[serial]
fn staged() -> Result<()> {
    let helper = do_test_setup()?;
    helper.modify_files()?;
    helper.stage_all()?;

    let precious = precious_path()?;
    let env = HashMap::new();
    exec::run(
        &precious,
        &["lint", "--staged"],
        &env,
        &[0],
        None,
        Some(&helper.precious_root()),
    )?;
    exec::run(
        &precious,
        &["tidy", "--staged"],
        &env,
        &[0],
        None,
        Some(&helper.precious_root()),
    )?;

    Ok(())
}

#[test]
#[serial]
fn cli_paths() -> Result<()> {
    let helper = do_test_setup()?;
    let files = helper.modify_files()?;

    let precious = precious_path()?;
    let env = HashMap::new();
    let mut args = vec!["lint"];
    args.append(&mut files.iter().map(|p| p.to_str().unwrap()).collect());
    exec::run(
        &precious,
        &args,
        &env,
        &[0],
        None,
        Some(&helper.precious_root()),
    )?;

    let mut args = vec!["tidy"];
    args.append(&mut files.iter().map(|p| p.to_str().unwrap()).collect());
    exec::run(
        &precious,
        &args,
        &env,
        &[0],
        None,
        Some(&helper.precious_root()),
    )?;

    Ok(())
}

#[test]
#[serial]
fn all_in_subdir() -> Result<()> {
    let helper = do_test_setup()?;

    let precious = precious_path()?;
    let env = HashMap::new();

    let mut cwd = helper.precious_root();
    cwd.push("src");

    exec::run(&precious, &["lint", "--all"], &env, &[0], None, Some(&cwd))?;
    exec::run(&precious, &["tidy", "--all"], &env, &[0], None, Some(&cwd))?;

    Ok(())
}

#[test]
#[serial]
fn git_in_subdir() -> Result<()> {
    let helper = do_test_setup()?;
    helper.modify_files()?;

    let precious = precious_path()?;
    let env = HashMap::new();

    let mut cwd = helper.precious_root();
    cwd.push("src");

    exec::run(&precious, &["lint", "--git"], &env, &[0], None, Some(&cwd))?;
    exec::run(&precious, &["tidy", "--git"], &env, &[0], None, Some(&cwd))?;

    Ok(())
}

#[test]
#[serial]
fn staged_in_subdir() -> Result<()> {
    let helper = do_test_setup()?;
    helper.modify_files()?;
    helper.stage_all()?;

    let precious = precious_path()?;
    let env = HashMap::new();

    let mut cwd = helper.precious_root();
    cwd.push("src");

    exec::run(
        &precious,
        &["lint", "--staged"],
        &env,
        &[0],
        None,
        Some(&cwd),
    )?;
    exec::run(
        &precious,
        &["tidy", "--staged"],
        &env,
        &[0],
        None,
        Some(&cwd),
    )?;

    Ok(())
}

#[test]
#[serial]
fn cli_paths_in_subdir() -> Result<()> {
    let helper = do_test_setup()?;
    helper.modify_files()?;

    let precious = precious_path()?;
    let env = HashMap::new();

    let mut cwd = helper.precious_root();
    cwd.push("src");

    exec::run(
        &precious,
        &["lint", "module.rs", "../README.md", "../tests/data/foo.txt"],
        &env,
        &[0],
        None,
        Some(&cwd),
    )?;
    exec::run(
        &precious,
        &["tidy", "module.rs", "../README.md", "../tests/data/foo.txt"],
        &env,
        &[0],
        None,
        Some(&cwd),
    )?;

    Ok(())
}

#[test]
#[serial]
fn one_command() -> Result<()> {
    let helper = do_test_setup()?;
    let content = r#"
fn foo() -> u8   {
    42
}
"#;
    helper.write_file("src/module.rs", content)?;

    let precious = precious_path()?;
    let env = HashMap::new();

    let mut cwd = helper.precious_root();
    cwd.push("src");

    // This succeeds because we're not checking with rustfmt.
    exec::run(
        &precious,
        &["lint", "--command", "true", "module.rs"],
        &env,
        &[0],
        None,
        Some(&cwd),
    )?;
    // This fails now that we check with rustfmt.
    exec::run(
        &precious,
        &["lint", "module.rs"],
        &env,
        &[1],
        None,
        Some(&cwd),
    )?;

    Ok(())
}

#[test]
#[serial]
fn all_invocation_options() -> Result<()> {
    let helper = do_test_setup()?;
    write_bash_script(&helper)?;
    create_file_tree(&helper)?;

    let docs =
        fs::read_to_string(PathBuf::from("../docs/invocation-examples.md"))?.replace("\r\n", "\n");
    let docs_re = Regex::new(
        r#"(?xsm)
            ```toml\n
            \[commands\.some-linter\]\n
            (?P<config>.+?)
            ```
            \n+
            ```\n
            (?P<output>.+?)
            ```
        "#,
    )?;

    let mut count = 0;
    for caps in docs_re.captures_iter(&docs) {
        let config = &caps["config"];
        match run_one_invocation_test(&helper, config, &caps["output"]) {
            Ok(..) => (),
            Err(e) => {
                eprintln!("Error from this config:\n{}", config);
                return Err(e);
            }
        }
        count += 1;
    }
    const EXPECT_COUNT: u8 = 28;
    assert_eq!(count, EXPECT_COUNT, "tested {EXPECT_COUNT} examples");

    Ok(())
}

fn write_bash_script(helper: &TestHelper) -> Result<()> {
    let script_contents = r#"
if [ -z "$PRECIOUS_INTEGRATION_TEST_OUTPUT_FILE" ]; then
    echo "No PRECIOUS_INTEGRATION_TEST_OUTPUT_FILE set!"
    exit 1
fi

if [ -z "$PRECIOUS_INTEGRATION_TEST_ROOT" ]; then
    echo "No PRECIOUS_INTEGRATION_TEST_ROOT set!"
    exit 1
fi

# Since precious runs the linter in parallel on different files we need to
# lock the output file.
(
    flock --exclusive --wait 2.0 42 || exit 1

    echo "----" 1>&42

    cwd=$(pwd)
    echo "cwd = $cwd" 1>&42

    echo "some-linter $@" 1>&42

) 42>>"$PRECIOUS_INTEGRATION_TEST_OUTPUT_FILE"

exit 0
"#;
    let mut script_file = helper.precious_root();
    script_file.push("some-linter.sh");
    fs::write(&script_file, script_contents)?;

    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = script_file.metadata()?.permissions();
        perms.set_mode(0o0755);
        fs::set_permissions(&script_file, perms)?;
    }

    Ok(())
}

// example
// ├── app.go
// ├── main.go
// ├── pkg1
// │  ├── pkg1.go
// ├── pkg2
// │  ├── pkg2.go
// │  ├── pkg2_test.go
// │  └── subpkg
// │     └── subpkg.go
fn create_file_tree(helper: &TestHelper) -> Result<()> {
    let root = helper.precious_root();

    for path in &[
        "app.go",
        "main.go",
        "pkg1/pkg1.go",
        "pkg2/pkg2.go",
        "pkg2/pkg2_test.go",
        "pkg2/subpkg/subpkg.go",
    ] {
        let mut file = root.clone();
        file.push(path);
        fs::create_dir_all(file.parent().unwrap())?;
        fs::write(&file, "x")?;
    }

    Ok(())
}

fn run_one_invocation_test(helper: &TestHelper, config: &str, expect: &str) -> Result<()> {
    let mut precious_toml = helper.precious_root();
    precious_toml.push("precious.toml");
    let precious = precious_path()?;

    let full_config = format!(
        r#"
[commands.some-linter]
type = "lint"
include = "**/*.go"
cmd = [ "bash", "{}/some-linter.sh" ]
ok_exit_codes = 0
{config}
"#,
        helper.precious_root().display(),
    );

    if cfg!(windows) {
        fs::write(&precious_toml, &full_config.replace('\n', "\r\n"))?;
    } else {
        fs::write(&precious_toml, &full_config)?;
    }

    let td = tempfile::Builder::new()
        .prefix("precious-all_invocation_options-")
        .tempdir()?;
    let mut output_file = td.path().to_path_buf();
    output_file.push("linter-output.txt");

    let (_output_dir, _preserved_tempdir) = match env::var("PRECIOUS_TESTS_PRESERVE_TEMPDIR") {
        Ok(v) if !(v.is_empty() || v == "0") => (None, Some(td.into_path())),
        _ => (Some(td), None),
    };

    let env = HashMap::from([
        (
            String::from("PRECIOUS_INTEGRATION_TEST_OUTPUT_FILE"),
            output_file.to_string_lossy().to_string(),
        ),
        (
            String::from("PRECIOUS_INTEGRATION_TEST_ROOT"),
            helper.precious_root().to_string_lossy().to_string(),
        ),
    ]);
    let _result = exec::run(
        &precious,
        &["--debug", "lint", "--all"],
        &env,
        &[0],
        Some(&[Regex::new(".*")?]),
        Some(&helper.precious_root()),
    )?;
    // println!("STDERR");
    // println!("{}", _result.stderr.as_deref().unwrap_or(""));

    let got = munge_invocation_output(output_file, helper.precious_root())?;

    let expect = expect
        .replace("/example", &helper.precious_root().to_string_lossy())
        .replace(" \\\n    ", " ");
    // println!("GOT");
    // println!("{got}");
    // println!("EXPECT");
    // println!("{expect}");
    assert_str_eq!(got, expect, "\n{config}");

    Ok(())
}

fn munge_invocation_output(output_file: PathBuf, precious_root: PathBuf) -> Result<String> {
    let got = fs::read_to_string(&output_file)
        .with_context(|| format!("Could not read file {}", output_file.display()))?
        .replace("\r\n", "\n");
    let output_re = Regex::new(
        r#"(?x)
           ----\n
           cwd\ =\ (?P<cwd>.+?)\n
           (?P<cmd>some-linter)(?:\ (?P<paths>.+?)?)\n
        "#,
    )?;

    #[derive(Debug)]
    struct Invocation<'a> {
        cwd: &'a str,
        cmd: &'a str,
        paths: Option<&'a str>,
    }

    let mut invocations: Vec<Invocation> = vec![];
    for caps in output_re.captures_iter(&got) {
        invocations.push(Invocation {
            cwd: caps.name("cwd").unwrap().as_str(),
            cmd: caps.name("cmd").unwrap().as_str(),
            paths: caps.name("paths").map(|p| p.as_str()),
        });
    }
    invocations.sort_by(|a, b| {
        if a.cwd != b.cwd {
            return a.cwd.cmp(b.cwd);
        }
        a.paths.unwrap_or("").cmp(b.paths.unwrap_or(""))
    });

    let mut last_cd = format!("{}", precious_root.display());
    Ok(invocations
        .iter()
        .map(|i| {
            let mut output = String::new();
            if last_cd != i.cwd {
                output.push_str("cd ");
                output.push_str(i.cwd);
                output.push('\n');
            }
            last_cd = i.cwd.to_string();
            output.push_str(i.cmd);
            if let Some(paths) = i.paths {
                output.push(' ');
                output.push_str(paths);
            }
            output.push('\n');
            output
        })
        .join(""))
}

fn precious_path() -> Result<String> {
    let mut precious = env::current_dir()?;
    precious.push("..");
    precious.push("target");
    precious.push("debug");
    precious.push("precious");
    Ok(precious.to_string_lossy().to_string())
}

fn do_test_setup() -> Result<TestHelper> {
    let cargo_build_re = Regex::new("Finished dev")?;
    let env = HashMap::new();
    exec::run(
        "cargo",
        &["build", "--package", "precious"],
        &env,
        &[0],
        Some(&[cargo_build_re]),
        Some(&PathBuf::from("..")),
    )?;

    let helper = TestHelper::new()?
        .with_git_repo()?
        .with_config_file("precious.toml", CONFIG)?;
    helper.write_file("src/good.rs", GOOD_RUST.trim_start())?;

    Ok(helper)
}
