use raw::*;
use function::UncompiledFunction;
use function::ABI::CDECL;
use types::get;
use libc::c_long;
use value::Value;
use std::c_str::CString;
use std::mem::transmute;
use types::Type;
use util::NativeRef;
/// A type that can be compiled into a LibJIT representation
pub trait Compile for Sized? {
    /// Get a JIT representation of this value
    fn compile<'a>(&self, func:&UncompiledFunction<'a>) -> Value<'a>;
    /// Get the JIT type repr of the value
    fn jit_type(_:Option<Self>) -> Type;
}
impl Compile for () {
    #[inline(always)]
    fn compile<'a>(&self, func:&UncompiledFunction<'a>) -> Value<'a> {
        let ty = get::<()>();
        Value::new(func, ty)
    }
    #[inline(always)]
    fn jit_type(_:Option<()>) -> Type {
        unsafe {
            NativeRef::from_ptr(jit_type_void)
        }
    }
}
compile_prims!{
    (f64, f64) => (jit_type_float64, jit_value_create_float64_constant),
    (f32, f32) => (jit_type_float32, jit_value_create_float32_constant),
    (int, c_long) => (jit_type_nint, jit_value_create_nint_constant),
    (uint, c_long) => (jit_type_nuint, jit_value_create_nint_constant),
    (i64, c_long) => (jit_type_long, jit_value_create_long_constant),
    (u64, c_long) => (jit_type_ulong, jit_value_create_long_constant),
    (i32, c_long) => (jit_type_int, jit_value_create_nint_constant),
    (u32, c_long) => (jit_type_uint, jit_value_create_nint_constant),
    (i16, c_long) => (jit_type_short, jit_value_create_nint_constant),
    (u16, c_long) => (jit_type_ushort, jit_value_create_nint_constant),
    (i8, c_long) => (jit_type_sbyte, jit_value_create_nint_constant),
    (u8, c_long) => (jit_type_ubyte, jit_value_create_nint_constant),
    (bool, c_long) => (jit_type_sys_bool, jit_value_create_nint_constant),
    (char, c_long) => (jit_type_sys_char, jit_value_create_nint_constant)
}
impl Compile for *const u8 {
    fn compile<'a>(&self, func:&UncompiledFunction<'a>) -> Value<'a> {
        let c_str = unsafe { CString::new(transmute(*self), false) };
        let ty = get::<&u8>();
        let ptr = Value::new(func, ty);
        let length = c_str.len() + 1u;
        func.insn_store(&ptr, &func.insn_alloca(&func.insn_of(&length)));
        for (pos, ch) in c_str.iter().enumerate() {
            let char_v = ch.compile(func);
            func.insn_store_relative(&ptr, pos as int, &char_v);
        }
        func.insn_store_relative(&ptr, c_str.len() as int, &func.insn_of(&'\0'));
        ptr
    }
    #[inline(always)]
    fn jit_type(_:Option<*const u8>) -> Type {
        get::<&u8>()
    }
}
impl<T:Compile> Compile for *mut T {
    fn compile<'a>(&self, func:&UncompiledFunction<'a>) -> Value<'a> {
        unsafe {
            NativeRef::from_ptr(jit_value_create_nint_constant(
                func.as_ptr(),
                get::<*mut T>().as_ptr(),
                self.to_uint() as c_long
            ))
        }
    }
    #[inline(always)]
    fn jit_type(_:Option<*mut T>) -> Type {
        Type::create_pointer(get::<T>())
    }
}
impl<'s> Compile for CString {
    fn compile<'a>(&self, func:&UncompiledFunction<'a>) -> Value<'a> {
        let ty = get::<CString>();
        let val = Value::new(func, ty.clone());
        let string:*const u8 = unsafe { transmute(self.as_ptr()) };
        func.insn_store_relative(&val, 0, &string.compile(func));
        func.insn_store_relative(&val, ty.find_name("is_owned").get_offset() as int, &true.compile(func));
        val
    }
    #[inline]
    fn jit_type(_:Option<CString>) -> Type {
        jit!(struct {
            "ptr": *const u8,
            "is_owned": bool
        })
    }
}
impl<'a> Compile for &'a str {
    fn compile<'b>(&self, func:&UncompiledFunction<'b>) -> Value<'b> {
        let str_ptr = {
            let ty = get::<*const u8>();
            let ptr = Value::new(func, ty);
            func.insn_store(&ptr, &func.insn_alloca(&func.insn_of(&self.len())));
            for enum_char in self.bytes().enumerate() {
                let (pos, ch) = enum_char;
                let char_v = ch.compile(func);
                func.insn_store_relative(&ptr, pos as int, &char_v);
            }
            ptr
        };
        let ty = get::<&'a str>();
        let val = Value::new(func, ty.clone());
        func.insn_store_relative(&val, 0, &str_ptr);
        func.insn_store_relative(&val, ty.find_name("len").get_offset() as int, &self.len().compile(func));
        val
    }
    #[inline]
    fn jit_type(_:Option<&'a str>) -> Type {
        jit!(struct {
            "ptr": *const u8,
            "len": uint
        })
    }
}
impl<'a> Compile for String {
    fn compile<'b>(&self, func:&UncompiledFunction<'b>) -> Value<'b> {
        let str_ptr = {
            let ty = get::<*const u8>();
            let ptr = Value::new(func, ty);
            func.insn_store(&ptr, &func.insn_alloca(&func.insn_of(&self.len())));
            for (pos, ch) in self.as_slice().bytes().enumerate() {
                let char_v = ch.compile(func);
                func.insn_store_relative(&ptr, pos as int, &char_v);
            }
            ptr
        };
        let ty = get::<String>();
        let val = Value::new(func, ty.clone());
        let length = self.len().compile(func);
        func.insn_store_relative(&val, 0, &length);
        func.insn_store_relative(&val, ty.clone().find_name("cap").get_offset() as int, &length);
        func.insn_store_relative(&val, ty.find_name("ptr").get_offset() as int, &str_ptr);
        val
    }
    #[inline]
    fn jit_type(_:Option<String>) -> Type {
        jit!(struct {
            "len": uint,
            "cap": uint,
            "ptr": &u8
        })
    }
}
impl<T:Compile> Compile for Vec<T> {
    fn compile<'b>(&self, func:&UncompiledFunction<'b>) -> Value<'b> {
        let vec_ptr = {
            let ty = get::<*mut T>();
            let inner_ty = get::<T>();
            let ptr = Value::new(func, ty);
            let ptr_size = self.len() * inner_ty.get_size();
            let ptr_size = func.insn_of(&ptr_size);
            func.insn_store(&ptr, &func.insn_alloca(&ptr_size));
            for (pos, val) in self.iter().enumerate() {
                let val_v = val.compile(func);
                func.insn_store_relative(&ptr, pos as int, &val_v);
            }
            ptr
        };
        let ty = get::<String>();
        let val = Value::new(func, ty.clone());
        let length = func.insn_of(&self.len());
        func.insn_store_relative(&val, 0, &length);
        func.insn_store_relative(&val, ty.clone().find_name("cap").get_offset() as int, &length);
        func.insn_store_relative(&val, ty.find_name("ptr").get_offset() as int, &vec_ptr);
        val
    }
    #[inline]
    fn jit_type(_:Option<Vec<T>>) -> Type {
        jit!(struct {
            "len": uint,
            "cap": uint,
            "ptr": *mut T
        })
    }
}
impl<T:Compile> Compile for &'static T {
    #[inline(always)]
    fn compile<'a>(&self, func:&UncompiledFunction<'a>) -> Value<'a> {
        unsafe {
            NativeRef::from_ptr(jit_value_create_nint_constant(
                func.as_ptr(),
                get::<&'static T>().as_ptr(),
                (*self as *const T).to_uint() as c_long
            ))
        }
    }
    #[inline(always)]
    fn jit_type(_:Option<&'static T>) -> Type {
        Type::create_pointer(get::<T>())
    }
}
compile_tuple!(A, B => a, b);
compile_tuple!(A, B, C => a, b, c);
compile_tuple!(A, B, C, D => a, b, c, d);
compile_tuple!(A, B, C, D, E => a, b, c, d, e);
compile_func!(fn() -> R, fn() -> R, extern fn() -> R);
compile_func!(fn(A) -> R, fn(A) -> R, extern fn(A) -> R);
compile_func!(fn(A, B) -> R, fn(A, B) -> R, extern fn(A, B) -> R);
compile_func!(fn(A, B, C) -> R, fn(A, B, C) -> R, extern fn(A, B, C) -> R);
compile_func!(fn(A, B, C, D) -> R, fn(A, B, C, D) -> R, extern fn(A, B, C, D) -> R);