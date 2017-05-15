#![cfg_attr(feature = "benchcmp", feature(test))]

mod tests {
    #[cfg(feature = "benchcmp")]
    extern crate test;

    use std::env;
    use std::process;

    #[cfg(feature = "benchcmp")]
    #[bench]
    fn benchcmp(_: &mut test::Bencher) {
        let travis_pr_branch = env::var("TRAVIS_PULL_REQUEST_BRANCH");

        let travis_branch = match env::var("TRAVIS_BRANCH") {
            Ok(s) => s,
            Err(_) => "benchcmp".to_owned(),
        };

        let travis_build_dir = match env::var("TRAVIS_BUILD_DIR") {
            Ok(s) => s,
            Err(_) => ".".to_owned(),
        };

        let remote_url = "https://github.com/twitter/rpc-perf";

        let working_branch = match travis_pr_branch {
            Ok(s) => s,
            Err(_) => travis_branch,
        };

        if working_branch == "master" {
            println!("SKIP: push to master");
            process::exit(0);
        }

        process::Command::new("mkdir")
            .arg("-p")
            .arg("target")
            .status()
            .expect("Failed to make target dir");

        let _ = process::Command::new("git")
            .arg("clone")
            .arg(remote_url)
            .arg("benchcmp".to_owned())
            .current_dir(travis_build_dir.to_owned() + "/target")
            .output();

        process::Command::new("bash")
            .arg("-c")
            .arg("cargo bench --features unstable | tee result.baseline")
            .current_dir(travis_build_dir.to_owned() + "/target/benchcmp")
            .status()
            .expect("Failed to cargo bench baseline");

        process::Command::new("bash")
            .arg("-c")
            .arg("cargo bench --features unstable | tee target/benchcmp/result.current")
            .current_dir(travis_build_dir.to_owned())
            .status()
            .expect("Failed to cargo bench current");

        let regressions = process::Command::new("cargo")
            .arg("benchcmp")
            .arg("target/benchcmp/result.baseline")
            .arg("target/benchcmp/result.current")
            .arg("--threshold")
            .arg("20")
            .arg("--regressions")
            .output()
            .expect("Failed to cargo benchcmp");

        if regressions.stdout.is_empty() {
            process::exit(0);
        } else {
            let output = String::from_utf8(regressions.stdout).unwrap();
            println!("{}", output);
            process::exit(1);
        }
    }
}
