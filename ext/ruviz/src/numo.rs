// Numo::NArray native-buffer access.
//
// Reused from the ruby-polars fork's numo bridge (ext/polars/src/interop/numo):
// the repr(C) structs mirror numo-narray-alt's `narray.h`, and the data pointer
// is fetched via rb-sys' stable API. For ruviz we only need the *read* direction
// (Numo -> Vec<f64>): a single memcpy for float64, one cast pass for other
// numeric dtypes — never a per-element Ruby Object round-trip (spec §8, §23).

use magnus::rb_sys::AsRawValue;
use magnus::{prelude::*, Error, Value};
use rb_sys::StableApiDefinition;

// numo-narray-alt narray.h layout (must stay in sync):
//   narray_t     : ndim(u8) type(u8) flag[2] elmsz(u16) size shape* reduce  = 32B
//   narray_data_t: narray_t base; char* ptr; bool owned;   (ptr @32, owned @40)
#[repr(C)]
#[allow(dead_code)] // some fields are here only to reproduce the C layout
struct NarrayT {
    ndim: std::os::raw::c_uchar,
    ntype: std::os::raw::c_uchar,
    flag: [std::os::raw::c_uchar; 2],
    elmsz: std::os::raw::c_ushort,
    size: usize,
    shape: *const usize,
    reduce: rb_sys::VALUE,
}
#[repr(C)]
#[allow(dead_code)]
struct NarrayDataT {
    base: NarrayT,
    ptr: *mut u8,
    owned: bool,
}
const NARRAY_DATA_T: u8 = 0x1;

/// Pointer to the `narray_data_t` behind a Numo NArray VALUE (rb-sys stable API).
fn narray_data_t(numo: Value) -> Option<*mut NarrayDataT> {
    let raw = numo.as_raw();
    let p =
        unsafe { rb_sys::stable_api::get_default().rtypeddata_get_data(raw) } as *mut NarrayDataT;
    if p.is_null() {
        None
    } else {
        Some(p)
    }
}

/// Validated data pointer of a 1-D contiguous DATA_T narray.
///
/// Cross-checks the struct's `ndim`/`size` against the Ruby-reported values
/// (ABI-drift guard): if numo-narray-alt ever changes the layout, this fails
/// and callers fall back to the public-API path rather than misread memory.
fn read_data_ptr(numo: Value, ndim: usize, len: usize) -> Option<*const u8> {
    let nd = unsafe { &*narray_data_t(numo)? };
    if nd.base.ndim as usize == ndim
        && nd.base.size == len
        && nd.base.ntype == NARRAY_DATA_T
        && !nd.ptr.is_null()
    {
        Some(nd.ptr as *const u8)
    } else {
        None
    }
}

fn class_name(val: Value) -> Result<String, Error> {
    let class: Value = val.funcall("class", ())?;
    class.funcall("name", ())
}

/// True if `val` is a Numo::NArray (any dtype).
pub fn is_numo(val: Value) -> bool {
    class_name(val)
        .map(|n| n.starts_with("Numo::"))
        .unwrap_or(false)
}

/// Copy `len` native-endian elements at `buf` into a `Vec<f64>`, casting to f64.
///
/// Safety: `buf` must point to `len` valid contiguous native-endian elements of
/// the dtype named by `class_name`, alive for the duration of the call.
unsafe fn read_native(class_name: &str, buf: *const u8, len: usize) -> Option<Vec<f64>> {
    macro_rules! cast_from {
        ($t:ty) => {{
            let sl = std::slice::from_raw_parts(buf as *const $t, len);
            sl.iter().map(|&x| x as f64).collect()
        }};
    }
    let v: Vec<f64> = match class_name {
        "Numo::DFloat" => std::slice::from_raw_parts(buf as *const f64, len).to_vec(),
        "Numo::SFloat" => cast_from!(f32),
        "Numo::Int64" => cast_from!(i64),
        "Numo::Int32" => cast_from!(i32),
        "Numo::Int16" => cast_from!(i16),
        "Numo::Int8" => cast_from!(i8),
        "Numo::UInt64" => cast_from!(u64),
        "Numo::UInt32" => cast_from!(u32),
        "Numo::UInt16" => cast_from!(u16),
        "Numo::UInt8" => cast_from!(u8),
        _ => return None,
    };
    Some(v)
}

/// Convert a 1-D Numo NArray to `Vec<f64>`.
///
/// Fast path (native, single copy): read the data pointer directly and copy/cast
/// once. Any case the fast path can't handle — non-1-D, byte-swapped, a view, an
/// unsupported dtype, or an unexpected struct layout — falls back to the public
/// `to_a` API so the result is always correct (worst case: as slow as `to_a`).
pub fn to_f64_vec(val: Value) -> Result<Vec<f64>, Error> {
    let ndim: usize = val.funcall("ndim", ())?;
    if ndim != 1 {
        return Err(Error::new(
            magnus::exception::arg_error(),
            format!("expected a 1-D Numo array, got ndim={ndim}"),
        ));
    }
    let size: usize = val.funcall("size", ())?;
    let swapped: bool = val.funcall("byte_swapped?", ()).unwrap_or(false);

    if !swapped {
        if let Ok(name) = class_name(val) {
            if let Some(buf) = read_data_ptr(val, 1, size) {
                // Safety: read_data_ptr validated `size` contiguous native-endian
                // elements at `buf`, owned by the live `val`; read_native copies out.
                if let Some(v) = unsafe { read_native(&name, buf, size) } {
                    return Ok(v);
                }
            }
        }
    }

    // Fallback: public API only (view / unsupported dtype / byte-swapped / drift).
    let arr: Value = val.funcall("to_a", ())?;
    magnus::TryConvert::try_convert(arr)
}
