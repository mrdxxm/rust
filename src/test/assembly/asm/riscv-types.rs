// no-system-llvm
// revisions: riscv64 riscv32
// assembly-output: emit-asm
//[riscv64] compile-flags: --target riscv64imac-unknown-none-elf
//[riscv32] compile-flags: --target riscv32imac-unknown-none-elf
// compile-flags: -C target-feature=+d
// needs-llvm-components: riscv

#![feature(no_core, lang_items, rustc_attrs)]
#![crate_type = "rlib"]
#![no_core]
#![allow(asm_sub_register)]

#[rustc_builtin_macro]
macro_rules! asm {
    () => {};
}
#[rustc_builtin_macro]
macro_rules! concat {
    () => {};
}
#[rustc_builtin_macro]
macro_rules! stringify {
    () => {};
}

#[lang = "sized"]
trait Sized {}
#[lang = "copy"]
trait Copy {}

type ptr = *mut u8;

impl Copy for i8 {}
impl Copy for i16 {}
impl Copy for i32 {}
impl Copy for f32 {}
impl Copy for i64 {}
impl Copy for f64 {}
impl Copy for ptr {}

extern "C" {
    fn extern_func();
    static extern_static: u8;
}

// CHECK-LABEL: sym_fn:
// CHECK: #APP
// CHECK: call extern_func
// CHECK: #NO_APP
#[no_mangle]
pub unsafe fn sym_fn() {
    asm!("call {}", sym extern_func);
}

// CHECK-LABEL: sym_static:
// CHECK: #APP
// CHECK: lb t0, extern_static
// CHECK: #NO_APP
#[no_mangle]
pub unsafe fn sym_static() {
    asm!("lb t0, {}", sym extern_static);
}

macro_rules! check {
    ($func:ident $ty:ident $class:ident $mov:literal) => {
        #[no_mangle]
        pub unsafe fn $func(x: $ty) -> $ty {
            // Hack to avoid function merging
            extern "Rust" {
                fn dont_merge(s: &str);
            }
            dont_merge(stringify!($func));

            let y;
            asm!(concat!($mov, " {}, {}"), out($class) y, in($class) x);
            y
        }
    };
}

// CHECK-LABEL: reg_i8:
// CHECK: #APP
// CHECK: mv {{[a-z0-9]+}}, {{[a-z0-9]+}}
// CHECK: #NO_APP
check!(reg_i8 i8 reg "mv");

// CHECK-LABEL: reg_i16:
// CHECK: #APP
// CHECK: mv {{[a-z0-9]+}}, {{[a-z0-9]+}}
// CHECK: #NO_APP
check!(reg_i16 i16 reg "mv");

// CHECK-LABEL: reg_i32:
// CHECK: #APP
// CHECK: mv {{[a-z0-9]+}}, {{[a-z0-9]+}}
// CHECK: #NO_APP
check!(reg_i32 i32 reg "mv");

// CHECK-LABEL: reg_f32:
// CHECK: #APP
// CHECK: mv {{[a-z0-9]+}}, {{[a-z0-9]+}}
// CHECK: #NO_APP
check!(reg_f32 f32 reg "mv");

// riscv64-LABEL: reg_i64:
// riscv64: #APP
// riscv64: mv {{[a-z0-9]+}}, {{[a-z0-9]+}}
// riscv64: #NO_APP
#[cfg(riscv64)]
check!(reg_i64 i64 reg "mv");

// riscv64-LABEL: reg_f64:
// riscv64: #APP
// riscv64: mv {{[a-z0-9]+}}, {{[a-z0-9]+}}
// riscv64: #NO_APP
#[cfg(riscv64)]
check!(reg_f64 f64 reg "mv");

// CHECK-LABEL: reg_ptr:
// CHECK: #APP
// CHECK: mv {{[a-z0-9]+}}, {{[a-z0-9]+}}
// CHECK: #NO_APP
check!(reg_ptr ptr reg "mv");

// CHECK-LABEL: freg_f32:
// CHECK: #APP
// CHECK: fmv.s f{{[a-z0-9]+}}, f{{[a-z0-9]+}}
// CHECK: #NO_APP
check!(freg_f32 f32 freg "fmv.s");

// CHECK-LABEL: freg_f64:
// CHECK: #APP
// CHECK: fmv.d f{{[a-z0-9]+}}, f{{[a-z0-9]+}}
// CHECK: #NO_APP
check!(freg_f64 f64 freg "fmv.d");