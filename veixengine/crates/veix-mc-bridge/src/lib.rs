//! Minecraft Java 版物品注册桥：DLL 由 Agent `System.load`，`JNI_OnLoad` 缓存 `JavaVM`。
//! C++ 模组链接 `veix_mc_register_item`（见 `native/veix-mod-sdk/include/veix/mod_api.h`）。

use jni::objects::{JValue, JValueOwned};
use jni::JavaVM;
use std::ffi::{c_char, c_int, c_void, CStr};
use std::sync::Mutex;

static JVM: Mutex<Option<JavaVM>> = Mutex::new(None);

#[no_mangle]
pub extern "system" fn JNI_OnLoad(vm: *mut jni_sys::JavaVM, _: *mut c_void) -> jni_sys::jint {
    unsafe {
        if let Ok(jvm) = JavaVM::from_raw(vm) {
            *JVM.lock().unwrap() = Some(jvm);
        }
    }
    jni_sys::JNI_VERSION_1_8
}

/// C++/宿主若在未触发 JNI_OnLoad 的环境下加载本 DLL，可显式传入由 `JNI_GetCreatedJavaVMs` 取得的 VM。
#[no_mangle]
pub extern "C" fn veix_mc_set_java_vm(vm: *mut jni_sys::JavaVM) -> c_int {
    if vm.is_null() {
        return -1;
    }
    unsafe {
        match JavaVM::from_raw(vm) {
            Ok(jvm) => {
                *JVM.lock().unwrap() = Some(jvm);
                0
            }
            Err(_) => -2,
        }
    }
}

#[no_mangle]
pub extern "C" fn veix_mc_register_item(namespace: *const c_char, path: *const c_char) -> c_int {
    if namespace.is_null() || path.is_null() {
        return -100;
    }
    let ns = unsafe { CStr::from_ptr(namespace) }.to_string_lossy();
    let p = unsafe { CStr::from_ptr(path) }.to_string_lossy();
    let full = format!("{}:{}", ns, p);

    let guard = match JVM.lock() {
        Ok(g) => g,
        Err(_) => return -1,
    };
    let vm = match guard.as_ref() {
        Some(v) => v,
        None => return -1,
    };

    let mut env = match vm.attach_current_thread_permanently() {
        Ok(e) => e,
        Err(_) => return -2,
    };

    let cls = match env.find_class("veix/VeixRegistryBridge") {
        Ok(c) => c,
        Err(_) => return -3,
    };

    let jstr = match env.new_string(full) {
        Ok(s) => s,
        Err(_) => return -4,
    };

    let ret: JValueOwned<'_> = match env.call_static_method(
        &cls,
        "registerItemByKey",
        "(Ljava/lang/String;)I",
        &[JValue::Object(&jstr)],
    ) {
        Ok(v) => v,
        Err(_) => return -5,
    };

    match ret.i() {
        Ok(i) => i as c_int,
        Err(_) => -99,
    }
}
