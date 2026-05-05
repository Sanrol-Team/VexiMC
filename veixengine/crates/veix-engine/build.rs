fn main() {
    #[cfg(target_os = "windows")]
    {
        let sched_root = std::path::PathBuf::from("../../native/veix-sched");
        let mut sched = cc::Build::new();
        sched
            .std("c11")
            .warnings(true)
            .define("WIN32_LEAN_AND_MEAN", None)
            .define("NOMINMAX", None)
            .include(sched_root.join("include"))
            .file(sched_root.join("src/sched_win.c"))
            .file(sched_root.join("src/process_win.c"));
        if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
            sched.define("_CRT_SECURE_NO_WARNINGS", None);
        }
        sched.compile("veix_sched");

        for rel in [
            "include/veix/sched.h",
            "include/veix/process.h",
            "src/sched_win.c",
            "src/process_win.c",
        ] {
            println!("cargo:rerun-if-changed={}", sched_root.join(rel).display());
        }
    }

    #[cfg(all(feature = "inject-native", target_os = "windows", target_env = "msvc"))]
    {
        let mut build = cc::Build::new();
        build
            .cpp(true)
            .std("c++17")
            .warnings(true)
            .define("NOMINMAX", None)
            .define("WIN32_LEAN_AND_MEAN", None)
            .file("native/src/win/iat_hijack.cpp")
            .file("native/src/win/jit_trampoline.cpp")
            .file("native/src/win/proxy_runtime.cpp");

        build.compile("veix_inject_build1");

        println!("cargo:rerun-if-changed=native/src/win/iat_hijack.cpp");
        println!("cargo:rerun-if-changed=native/src/win/jit_trampoline.cpp");
        println!("cargo:rerun-if-changed=native/src/win/proxy_runtime.cpp");
    }

    #[cfg(all(feature = "inject-native", not(all(target_os = "windows", target_env = "msvc"))))]
    {
        println!("cargo:warning=inject-native C++ 注入原语仅在 Windows MSVC 上编译；其它配置使用 Rust 占位实现。");
    }
}
